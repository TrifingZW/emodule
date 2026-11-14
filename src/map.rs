//! 地图网格与省界覆盖层。

use bevy::{
    asset::RenderAssetUsages,
    color::palettes,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::*,
    sprite_render::AlphaMode2d,
};

use crate::{
    camera::MapCamera,
    state::EditorState,
    wc4::{
        Bin, Btl, INNER_RADIUS, OUTER_RADIUS, Z_GRID, Z_PROVINCE_BOUNDARIES, grid_to_world,
        province_at,
    },
};

/// 世界坐标轴线的半长。
const AXIS_HALF_LENGTH: f32 = 8192.0;
/// 网格线宽（逻辑像素）。
const GRID_LINE_WIDTH: f32 = 2.0;
/// 省界线宽（逻辑像素）。
const PROVINCE_LINE_WIDTH: f32 = 3.0;
/// 坐标轴线宽（逻辑像素）。
const AXIS_LINE_WIDTH: f32 = 2.0;
/// 缩放变化超过该比例时重建线段宽度。
const LINE_SCALE_THRESHOLD: f32 = 0.1;

/// 标记网格覆盖层根实体。
#[derive(Component, Clone, Default)]
pub(crate) struct GridOverlay;

/// 网格覆盖层的线段网格句柄。
#[derive(Component)]
pub(crate) struct GridOverlayState {
    lines: Handle<Mesh>,
    x_axis: Handle<Mesh>,
    y_axis: Handle<Mesh>,
    built_scale: f32,
}

/// 标记省界覆盖层根实体。
#[derive(Component, Clone, Default)]
pub(crate) struct ProvinceOverlay;

/// 省界覆盖层的线段网格句柄。
#[derive(Component)]
pub(crate) struct ProvinceOverlayState {
    mesh: Handle<Mesh>,
    built_scale: f32,
}

/// 管理网格覆盖层的生成、线宽与可见性。
pub(crate) fn draw_grid(
    state: Res<EditorState>,
    btl: Option<Res<Btl>>,
    camera: Single<&Projection, With<MapCamera>>,
    mut overlay: Query<(Entity, &mut Visibility, Option<&mut GridOverlayState>), With<GridOverlay>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Projection::Orthographic(orthographic) = camera.into_inner() else {
        return;
    };
    let scale = orthographic.scale;
    let visible = state.grid_enabled();
    let desired = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    if let Ok((root, mut visibility, overlay_state)) = overlay.single_mut() {
        *visibility = desired;

        if !visible {
            return;
        }

        let Some(btl) = btl else {
            return;
        };
        let columns = btl.master.map_width;
        let rows = btl.master.map_height;
        if columns <= 0 || rows <= 0 {
            return;
        }

        if let Some(mut overlay_state) = overlay_state {
            if needs_rebuild(overlay_state.built_scale, scale) {
                rebuild_grid_meshes(&mut meshes, &mut overlay_state, columns, rows, scale);
            }
            return;
        }

        commands.entity(root).despawn_related::<Children>();
        let overlay_state = spawn_grid_children(
            &mut commands,
            &mut meshes,
            &mut materials,
            root,
            columns,
            rows,
            scale,
        );
        commands.entity(root).insert(overlay_state);
        return;
    }

    if !visible {
        return;
    }

    let Some(btl) = btl else {
        return;
    };
    let columns = btl.master.map_width;
    let rows = btl.master.map_height;
    if columns <= 0 || rows <= 0 {
        return;
    }

    let root = commands
        .spawn((GridOverlay, Transform::from_xyz(0.0, 0.0, Z_GRID), Visibility::Visible))
        .id();
    let overlay_state =
        spawn_grid_children(&mut commands, &mut meshes, &mut materials, root, columns, rows, scale);
    commands.entity(root).insert(overlay_state);
}

/// 管理省界覆盖层的生成、更新、线宽与可见性。
pub(crate) fn draw_province_boundaries(
    state: Res<EditorState>,
    btl: Option<Res<Btl>>,
    bin: Option<Res<Bin>>,
    camera: Single<&Projection, With<MapCamera>>,
    mut overlay: Query<
        (Entity, &mut Visibility, Option<&mut ProvinceOverlayState>),
        With<ProvinceOverlay>,
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Projection::Orthographic(orthographic) = camera.into_inner() else {
        return;
    };
    let scale = orthographic.scale;
    let visible = state.province_boundaries_enabled();
    let desired = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let data_changed = btl.as_ref().is_some_and(|btl| btl.is_changed())
        || bin.as_ref().is_some_and(|bin| bin.is_changed());

    if let Ok((root, mut visibility, overlay_state)) = overlay.single_mut() {
        let was_visible = *visibility == Visibility::Visible;
        *visibility = desired;

        if !visible {
            return;
        }

        let Some((btl, bin)) = btl.as_ref().zip(bin.as_ref()) else {
            return;
        };

        let scale_changed =
            overlay_state.as_ref().map_or(true, |state| needs_rebuild(state.built_scale, scale));
        if was_visible && !data_changed && !scale_changed {
            return;
        }

        if let Some(mut overlay_state) = overlay_state {
            if let Some(mut mesh) = meshes.get_mut(&overlay_state.mesh) {
                *mesh = line_quad_mesh(
                    province_boundary_segments(btl, bin),
                    PROVINCE_LINE_WIDTH * scale,
                );
            }
            overlay_state.built_scale = scale;
            return;
        }

        commands.entity(root).despawn_related::<Children>();
        let overlay_state =
            spawn_province_child(&mut commands, &mut meshes, &mut materials, root, btl, bin, scale);
        commands.entity(root).insert(overlay_state);
        return;
    }

    if !visible {
        return;
    }

    let Some((btl, bin)) = btl.as_ref().zip(bin.as_ref()) else {
        return;
    };

    let root = commands
        .spawn((
            ProvinceOverlay,
            Transform::from_xyz(0.0, 0.0, Z_PROVINCE_BOUNDARIES),
            Visibility::Visible,
        ))
        .id();
    let overlay_state =
        spawn_province_child(&mut commands, &mut meshes, &mut materials, root, btl, bin, scale);
    commands.entity(root).insert(overlay_state);
}

/// 构建网格覆盖层的全部子实体。
fn spawn_grid_children(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    root: Entity,
    columns: i32,
    rows: i32,
    scale: f32,
) -> GridOverlayState {
    let (lines, lines_mesh) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        hex_segments(columns, rows),
        GRID_LINE_WIDTH * scale,
        palettes::tailwind::GRAY_300.into(),
    );
    let (x_axis, x_axis_mesh) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        vec![(Vec2::new(-AXIS_HALF_LENGTH, 0.0), Vec2::new(AXIS_HALF_LENGTH, 0.0))],
        AXIS_LINE_WIDTH * scale,
        palettes::tailwind::RED_400.into(),
    );
    let (y_axis, y_axis_mesh) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        vec![(Vec2::new(0.0, -AXIS_HALF_LENGTH), Vec2::new(0.0, AXIS_HALF_LENGTH))],
        AXIS_LINE_WIDTH * scale,
        palettes::tailwind::GREEN_400.into(),
    );

    commands.entity(root).add_children(&[lines, x_axis, y_axis]);

    GridOverlayState {
        lines: lines_mesh,
        x_axis: x_axis_mesh,
        y_axis: y_axis_mesh,
        built_scale: scale,
    }
}

/// 构建省界覆盖层的子实体。
fn spawn_province_child(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    root: Entity,
    btl: &Btl,
    bin: &Bin,
    scale: f32,
) -> ProvinceOverlayState {
    let (child, mesh) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        province_boundary_segments(btl, bin),
        PROVINCE_LINE_WIDTH * scale,
        palettes::tailwind::YELLOW_300.into(),
    );

    commands.entity(root).add_child(child);

    ProvinceOverlayState {
        mesh,
        built_scale: scale,
    }
}

/// 就地更新网格覆盖层的线段宽度。
fn rebuild_grid_meshes(
    meshes: &mut Assets<Mesh>,
    state: &mut GridOverlayState,
    columns: i32,
    rows: i32,
    scale: f32,
) {
    if let Some(mut mesh) = meshes.get_mut(&state.lines) {
        *mesh = line_quad_mesh(hex_segments(columns, rows), GRID_LINE_WIDTH * scale);
    }
    if let Some(mut mesh) = meshes.get_mut(&state.x_axis) {
        *mesh = line_quad_mesh(
            vec![(Vec2::new(-AXIS_HALF_LENGTH, 0.0), Vec2::new(AXIS_HALF_LENGTH, 0.0))],
            AXIS_LINE_WIDTH * scale,
        );
    }
    if let Some(mut mesh) = meshes.get_mut(&state.y_axis) {
        *mesh = line_quad_mesh(
            vec![(Vec2::new(0.0, -AXIS_HALF_LENGTH), Vec2::new(0.0, AXIS_HALF_LENGTH))],
            AXIS_LINE_WIDTH * scale,
        );
    }

    state.built_scale = scale;
}

/// 返回缩放变化后是否需要调整线宽。
fn needs_rebuild(built_scale: f32, scale: f32) -> bool {
    built_scale <= 0.0 || (scale - built_scale).abs() > built_scale * LINE_SCALE_THRESHOLD
}

/// 生成覆盖整张地图的六边形线段。
fn hex_segments(columns: i32, rows: i32) -> Vec<(Vec2, Vec2)> {
    let mut segments = Vec::with_capacity(columns as usize * rows as usize * 3 + 16);

    for y in 0..rows {
        for x in 0..columns {
            let cell = IVec2::new(x, y);
            let corners = hex_corners(grid_to_world(x, y));

            for edge in 0..6 {
                let neighbor = hex_neighbor(cell, edge);
                let neighbor_inside =
                    neighbor.x >= 0 && neighbor.y >= 0 && neighbor.x < columns && neighbor.y < rows;

                // 内部边只由坐标序较小的一侧绘制，地图外边界始终绘制。
                if neighbor_inside
                    && (neighbor.x < cell.x || (neighbor.x == cell.x && neighbor.y <= cell.y))
                {
                    continue;
                }

                segments.push((corners[edge], corners[(edge + 1) % corners.len()]));
            }
        }
    }

    segments
}

/// 生成省界线段：仅保留两侧省份不同的地块共享边。
fn province_boundary_segments(btl: &Btl, bin: &Bin) -> Vec<(Vec2, Vec2)> {
    let columns = btl.master.map_width;
    let rows = btl.master.map_height;
    let mut segments = Vec::new();

    for y in 0..rows {
        for x in 0..columns {
            let cell = IVec2::new(x, y);
            let Some(province) = province_at(btl, bin, cell) else {
                continue;
            };
            let corners = hex_corners(grid_to_world(x, y));

            for edge in 0..6 {
                let neighbor = hex_neighbor(cell, edge);

                if neighbor.x < 0 || neighbor.y < 0 || neighbor.x >= columns || neighbor.y >= rows {
                    continue;
                }

                // 每条共享边只由坐标序较小的一侧绘制，避免重复。
                if neighbor.x < cell.x || (neighbor.x == cell.x && neighbor.y <= cell.y) {
                    continue;
                }

                let Some(other) = province_at(btl, bin, neighbor) else {
                    continue;
                };
                if province == other {
                    continue;
                }

                segments.push((corners[edge], corners[(edge + 1) % corners.len()]));
            }
        }
    }

    segments
}

/// 返回平顶六边形指定边对应的相邻地块。
fn hex_neighbor(cell: IVec2, edge: usize) -> IVec2 {
    let offsets = if cell.x.rem_euclid(2) == 0 {
        [
            IVec2::new(1, -1),
            IVec2::new(0, -1),
            IVec2::new(-1, -1),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(1, 0),
        ]
    } else {
        [
            IVec2::new(1, 0),
            IVec2::new(0, -1),
            IVec2::new(-1, 0),
            IVec2::new(-1, 1),
            IVec2::new(0, 1),
            IVec2::new(1, 1),
        ]
    };

    cell + offsets[edge % offsets.len()]
}

/// 计算平顶六边形的六个顶点。
fn hex_corners(center: Vec2) -> [Vec2; 6] {
    let half_width = OUTER_RADIUS;
    let half_height = INNER_RADIUS;
    let diagonal = OUTER_RADIUS * 0.5;

    [
        center + Vec2::new(half_width, 0.0),
        center + Vec2::new(diagonal, half_height),
        center + Vec2::new(-diagonal, half_height),
        center + Vec2::new(-half_width, 0.0),
        center + Vec2::new(-diagonal, -half_height),
        center + Vec2::new(diagonal, -half_height),
    ]
}

/// 将线段列表转换为带像素宽度的三角形网格。
fn line_quad_mesh(mut segments: Vec<(Vec2, Vec2)>, width: f32) -> Mesh {
    if segments.is_empty() {
        // 无边界时保留一个退化线段，使网格资源仍可安全更新。
        segments.push((Vec2::ZERO, Vec2::ZERO));
    }

    let half_width = width.max(0.1) * 0.5;
    let mut positions = Vec::with_capacity(segments.len() * 4);
    let mut uvs = Vec::with_capacity(segments.len() * 4);
    let mut normals = Vec::with_capacity(segments.len() * 4);
    let mut indices = Vec::with_capacity(segments.len() * 6);

    for (start, end) in segments {
        let delta = end - start;
        let normal = if delta.length_squared() > f32::EPSILON {
            Vec2::new(-delta.y, delta.x).normalize() * half_width
        } else {
            Vec2::ZERO
        };

        let base = positions.len() as u32;
        positions.extend([
            [start.x + normal.x, start.y + normal.y, 0.0],
            [start.x - normal.x, start.y - normal.y, 0.0],
            [end.x + normal.x, end.y + normal.y, 0.0],
            [end.x - normal.x, end.y - normal.y, 0.0],
        ]);
        uvs.extend([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
        normals.extend([[0.0, 0.0, 1.0]; 4]);
        indices.extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
}

/// 生成使用指定颜色与像素宽度的线段实体，并返回其网格句柄。
fn spawn_line_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    segments: Vec<(Vec2, Vec2)>,
    width: f32,
    color: Color,
) -> (Entity, Handle<Mesh>) {
    let mesh = meshes.add(line_quad_mesh(segments, width));
    let material = materials.add(ColorMaterial {
        color,
        alpha_mode: AlphaMode2d::Blend,
        ..default()
    });

    let entity = commands
        .spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material),
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();

    (entity, mesh)
}

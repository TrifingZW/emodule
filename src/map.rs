//! 地图网格与省界覆盖层。

use bevy::{
    asset::RenderAssetUsages,
    color::palettes,
    mesh::{Mesh, PrimitiveTopology},
    prelude::*,
    sprite_render::AlphaMode2d,
};

use crate::{
    state::EditorState,
    wc4::{
        Bin, Btl, INNER_RADIUS, OUTER_RADIUS, Z_GRID, Z_PROVINCE_BOUNDARIES, grid_to_world,
        province_at,
    },
};

/// 世界坐标轴线的半长。
const AXIS_HALF_LENGTH: f32 = 8192.0;

/// 标记网格覆盖层根实体。
#[derive(Component, Clone, Default)]
pub(crate) struct GridOverlay;

/// 标记省界覆盖层根实体。
#[derive(Component, Clone, Default)]
pub(crate) struct ProvinceOverlay;

/// 省界网格句柄，用于数据变化时就地更新。
#[derive(Component)]
pub(crate) struct ProvinceBoundaryMesh {
    mesh: Handle<Mesh>,
}

/// 管理网格覆盖层的生成与可见性。
pub(crate) fn draw_grid(
    state: Res<EditorState>,
    btl: Option<Res<Btl>>,
    mut overlay: Query<&mut Visibility, With<GridOverlay>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let desired = if state.grid_enabled() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    if let Ok(mut visibility) = overlay.single_mut() {
        *visibility = desired;
        return;
    }

    if !state.grid_enabled() {
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
        .spawn((
            GridOverlay,
            Transform::from_xyz(0.0, 0.0, Z_GRID),
            Visibility::Visible,
        ))
        .id();

    let (lines, _) = spawn_line_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        hex_segments(columns, rows),
        palettes::tailwind::GRAY_700.into(),
    );
    let (x_axis, _) = spawn_line_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        vec![(
            Vec2::new(-AXIS_HALF_LENGTH, 0.0),
            Vec2::new(AXIS_HALF_LENGTH, 0.0),
        )],
        palettes::tailwind::RED_900.into(),
    );
    let (y_axis, _) = spawn_line_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        vec![(
            Vec2::new(0.0, -AXIS_HALF_LENGTH),
            Vec2::new(0.0, AXIS_HALF_LENGTH),
        )],
        palettes::tailwind::GREEN_900.into(),
    );

    commands.entity(root).add_children(&[lines, x_axis, y_axis]);
}

/// 管理省界覆盖层的生成、更新与可见性。
pub(crate) fn draw_province_boundaries(
    state: Res<EditorState>,
    btl: Option<Res<Btl>>,
    bin: Option<Res<Bin>>,
    mut overlay: Query<
        (Entity, &mut Visibility, Option<&ProvinceBoundaryMesh>),
        With<ProvinceOverlay>,
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let visible = state.province_boundaries_enabled();
    let desired = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let changed = btl.as_ref().is_some_and(|btl| btl.is_changed())
        || bin.as_ref().is_some_and(|bin| bin.is_changed());

    if let Ok((root, mut visibility, boundary)) = overlay.single_mut() {
        let was_visible = *visibility == Visibility::Visible;
        *visibility = desired;

        if !visible || (was_visible && !changed) {
            return;
        }

        let Some((btl, bin)) = btl.as_ref().zip(bin.as_ref()) else {
            return;
        };

        if let Some(boundary) = boundary {
            if let Some(mut mesh) = meshes.get_mut(&boundary.mesh) {
                *mesh = line_mesh(province_boundary_segments(btl, bin));
            }
            return;
        }

        let (child, mesh) = spawn_line_mesh(
            &mut commands,
            &mut meshes,
            &mut materials,
            province_boundary_segments(btl, bin),
            palettes::tailwind::ORANGE_400.into(),
        );
        commands.entity(root).add_child(child);
        commands.entity(root).insert(ProvinceBoundaryMesh { mesh });
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

    let (child, mesh) = spawn_line_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        province_boundary_segments(btl, bin),
        palettes::tailwind::ORANGE_400.into(),
    );
    commands.entity(root).add_child(child);
    commands.entity(root).insert(ProvinceBoundaryMesh { mesh });
}

/// 生成覆盖整张地图的六边形线段。
fn hex_segments(columns: i32, rows: i32) -> Vec<(Vec2, Vec2)> {
    let mut segments = Vec::with_capacity(columns as usize * rows as usize * 6);

    for y in 0..rows {
        for x in 0..columns {
            let corners = hex_corners(grid_to_world(x, y));

            for index in 0..corners.len() {
                segments.push((corners[index], corners[(index + 1) % corners.len()]));
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

                if neighbor.x < 0
                    || neighbor.y < 0
                    || neighbor.x >= columns
                    || neighbor.y >= rows
                {
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

/// 生成线段列表网格。
fn line_mesh(mut segments: Vec<(Vec2, Vec2)>) -> Mesh {
    if segments.is_empty() {
        // 无边界时保留一个退化线段，使网格资源仍可安全更新。
        segments.push((Vec2::ZERO, Vec2::ZERO));
    }

    let mut positions = Vec::with_capacity(segments.len() * 2);

    for (start, end) in segments {
        positions.push([start.x, start.y, 0.0]);
        positions.push([end.x, end.y, 0.0]);
    }

    let uvs = vec![[0.0, 0.0]; positions.len()];
    let normals = vec![[0.0, 0.0, 1.0]; positions.len()];

    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}

/// 生成使用指定颜色渲染的线段实体，并返回其网格句柄。
fn spawn_line_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    segments: Vec<(Vec2, Vec2)>,
    color: Color,
) -> (Entity, Handle<Mesh>) {
    let mesh = meshes.add(line_mesh(segments));
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

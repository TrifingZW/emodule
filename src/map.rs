//! 地图网格、省界与国家边界覆盖层。

use bevy::{
    asset::RenderAssetUsages,
    color::palettes,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use crate::{
    camera::MapCamera,
    state::EditorState,
    wc4::{
        Bin, Btl, INNER_RADIUS, OUTER_RADIUS, TileOwners, Z_COUNTRY_BOUNDARIES, Z_GRID,
        Z_PROVINCE_BOUNDARIES, grid_to_world, province_at, water_at,
    },
};

/// 覆盖层线条的着色器路径。
const OVERLAY_LINE_SHADER_PATH: &str = "shaders/overlay_line.wesl";
/// 世界坐标轴线的半长。
const AXIS_HALF_LENGTH: f32 = 8192.0;
/// 网格线宽（逻辑像素）。
const GRID_LINE_WIDTH: f32 = 2.0;
/// 省界线宽（逻辑像素）。
const PROVINCE_LINE_WIDTH: f32 = 2.0;
/// 国界线宽（逻辑像素）。
const COUNTRY_LINE_WIDTH: f32 = 3.0;
/// 坐标轴线宽（逻辑像素）。
const AXIS_LINE_WIDTH: f32 = 2.0;
/// 省界虚线的实线段长度（逻辑像素）。
const PROVINCE_DASH_LENGTH: f32 = 9.0;
/// 省界虚线的间隔长度（逻辑像素）。
const PROVINCE_GAP_LENGTH: f32 = 6.0;
/// 省界虚线的实线段与间隔长度。
const PROVINCE_DASH: (f32, f32) = (PROVINCE_DASH_LENGTH, PROVINCE_GAP_LENGTH);
/// 缩放变化超过该比例时重建线段与虚线长度。
const LINE_SCALE_THRESHOLD: f32 = 0.02;

/// 注册覆盖层材质与绘制系统。
pub(crate) struct MapOverlayPlugin;

impl Plugin for MapOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<OverlayLineMaterial>::default())
            .add_systems(Update, (draw_grid, draw_province_boundaries, draw_country_boundaries));
    }
}

/// 覆盖层线条材质。
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct OverlayLineMaterial {
    /// 线条样式。
    #[uniform(0)]
    style: OverlayLineStyle,
}

/// 覆盖层线条样式。
#[derive(Clone, Copy, Debug, ShaderType)]
pub(crate) struct OverlayLineStyle {
    /// 线条颜色（线性空间）。
    color: Vec4,
    /// 虚线的实线段长度（世界单位），为零表示实线。
    dash_length: f32,
    /// 虚线的间隔长度（世界单位）。
    gap_length: f32,
    /// 对齐到 uniform 的 16 字节长度。
    padding: Vec2,
}

impl OverlayLineStyle {
    /// 以世界单位的虚线长度创建样式。
    fn new(color: Color, dash: Option<(f32, f32)>) -> Self {
        let (dash_length, gap_length) = dash.unwrap_or_default();

        Self {
            color: linear_color(color),
            dash_length,
            gap_length,
            padding: Vec2::ZERO,
        }
    }
}

impl Material2d for OverlayLineMaterial {
    fn fragment_shader() -> ShaderRef {
        OVERLAY_LINE_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// 单层线条的配色、像素线宽与虚线设置。
#[derive(Clone, Copy)]
struct LineStyle {
    /// 线条颜色。
    color: Color,
    /// 线条的世界宽度。
    width: f32,
    /// 虚线的实线段与间隔长度（世界单位），`None` 表示实线。
    dash: Option<(f32, f32)>,
}

/// 标记网格覆盖层根实体。
#[derive(Component, Clone, Default)]
pub(crate) struct GridOverlay;

/// 网格覆盖层的线段网格句柄与缓存线段。
#[derive(Component)]
pub(crate) struct GridOverlayState {
    lines: Handle<Mesh>,
    x_axis: Handle<Mesh>,
    y_axis: Handle<Mesh>,
    segments: Vec<(Vec2, Vec2)>,
    columns: i32,
    rows: i32,
    built_scale: f32,
    fixed_width: bool,
}

/// 标记省界覆盖层根实体。
#[derive(Component, Clone, Default)]
pub(crate) struct ProvinceOverlay;

/// 省界覆盖层的线段网格与缓存线段。
#[derive(Component)]
pub(crate) struct ProvinceOverlayState {
    mesh: Handle<Mesh>,
    material: Handle<OverlayLineMaterial>,
    segments: Vec<(Vec2, Vec2)>,
    built_scale: f32,
    fixed_width: bool,
}

/// 标记国界覆盖层根实体。
#[derive(Component, Clone, Default)]
pub(crate) struct CountryOverlay;

/// 国界覆盖层的线段网格与缓存线段。
#[derive(Component)]
pub(crate) struct CountryOverlayState {
    mesh: Handle<Mesh>,
    segments: Vec<(Vec2, Vec2)>,
    built_scale: f32,
    fixed_width: bool,
}

/// 管理网格覆盖层的生成、线宽与可见性。
fn draw_grid(
    state: Res<EditorState>,
    btl: Option<Res<Btl>>,
    camera: Single<&Projection, With<MapCamera>>,
    mut overlay: Query<(Entity, &mut Visibility, Option<&mut GridOverlayState>), With<GridOverlay>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OverlayLineMaterial>>,
) {
    let Projection::Orthographic(orthographic) = camera.into_inner() else {
        return;
    };
    let scale = orthographic.scale;
    let fixed = state.line_width_fixed();
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
            let dimensions_changed = overlay_state.columns != columns || overlay_state.rows != rows;
            if dimensions_changed {
                (overlay_state.columns, overlay_state.rows) = (columns, rows);
                overlay_state.segments = hex_segments(columns, rows);
            }
            let scale_changed = fixed && needs_rebuild(overlay_state.built_scale, scale);
            if dimensions_changed || overlay_state.fixed_width != fixed || scale_changed {
                rebuild_grid_meshes(&mut meshes, &mut overlay_state, scale, fixed);
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
            fixed,
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
    let overlay_state = spawn_grid_children(
        &mut commands,
        &mut meshes,
        &mut materials,
        root,
        columns,
        rows,
        scale,
        fixed,
    );
    commands.entity(root).insert(overlay_state);
}

/// 管理省界覆盖层的生成、更新、线宽与可见性。
fn draw_province_boundaries(
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
    mut materials: ResMut<Assets<OverlayLineMaterial>>,
) {
    let Projection::Orthographic(orthographic) = camera.into_inner() else {
        return;
    };
    let scale = orthographic.scale;
    let visible = state.province_boundaries_enabled();
    let fixed = state.line_width_fixed();
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

        let width_changed = overlay_state.as_ref().map_or(true, |overlay| {
            overlay.fixed_width != fixed || (fixed && needs_rebuild(overlay.built_scale, scale))
        });
        if was_visible && !data_changed && !width_changed {
            return;
        }

        if let Some(mut overlay_state) = overlay_state {
            if data_changed || !was_visible {
                overlay_state.segments = province_boundary_segments(btl, bin);
            }
            rebuild_line_mesh(
                &mut meshes,
                &overlay_state.mesh,
                &overlay_state.segments,
                world_length(PROVINCE_LINE_WIDTH, scale, fixed),
            );
            sync_dash_style(
                &mut materials,
                &overlay_state.material,
                dash_lengths(PROVINCE_DASH, scale, fixed),
            );
            overlay_state.built_scale = scale;
            overlay_state.fixed_width = fixed;
            return;
        }

        commands.entity(root).despawn_related::<Children>();
        let overlay_state = spawn_province_child(
            &mut commands,
            &mut meshes,
            &mut materials,
            root,
            btl,
            bin,
            scale,
            fixed,
        );
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
    let overlay_state = spawn_province_child(
        &mut commands,
        &mut meshes,
        &mut materials,
        root,
        btl,
        bin,
        scale,
        fixed,
    );
    commands.entity(root).insert(overlay_state);
}

/// 管理国界覆盖层的生成、更新、线宽与可见性。
fn draw_country_boundaries(
    state: Res<EditorState>,
    btl: Option<Res<Btl>>,
    bin: Option<Res<Bin>>,
    camera: Single<&Projection, With<MapCamera>>,
    mut overlay: Query<
        (Entity, &mut Visibility, Option<&mut CountryOverlayState>),
        With<CountryOverlay>,
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OverlayLineMaterial>>,
) {
    let Projection::Orthographic(orthographic) = camera.into_inner() else {
        return;
    };
    let scale = orthographic.scale;
    let visible = state.country_boundaries_enabled();
    let fixed = state.line_width_fixed();
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

        let width_changed = overlay_state.as_ref().map_or(true, |overlay| {
            overlay.fixed_width != fixed || (fixed && needs_rebuild(overlay.built_scale, scale))
        });
        if was_visible && !data_changed && !width_changed {
            return;
        }

        if let Some(mut overlay_state) = overlay_state {
            if data_changed || !was_visible {
                overlay_state.segments = country_boundary_segments(btl, bin);
            }
            rebuild_line_mesh(
                &mut meshes,
                &overlay_state.mesh,
                &overlay_state.segments,
                world_length(COUNTRY_LINE_WIDTH, scale, fixed),
            );
            overlay_state.built_scale = scale;
            overlay_state.fixed_width = fixed;
            return;
        }

        commands.entity(root).despawn_related::<Children>();
        let overlay_state = spawn_country_child(
            &mut commands,
            &mut meshes,
            &mut materials,
            root,
            btl,
            bin,
            scale,
            fixed,
        );
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
            CountryOverlay,
            Transform::from_xyz(0.0, 0.0, Z_COUNTRY_BOUNDARIES),
            Visibility::Visible,
        ))
        .id();
    let overlay_state = spawn_country_child(
        &mut commands,
        &mut meshes,
        &mut materials,
        root,
        btl,
        bin,
        scale,
        fixed,
    );
    commands.entity(root).insert(overlay_state);
}

/// 构建网格覆盖层的全部子实体。
fn spawn_grid_children(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<OverlayLineMaterial>,
    root: Entity,
    columns: i32,
    rows: i32,
    scale: f32,
    fixed: bool,
) -> GridOverlayState {
    let grid_width = world_length(GRID_LINE_WIDTH, scale, fixed);
    let axis_width = world_length(AXIS_LINE_WIDTH, scale, fixed);
    let segments = hex_segments(columns, rows);

    let (lines, lines_mesh, _) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        &segments,
        LineStyle {
            color: palettes::tailwind::GRAY_300.into(),
            width: grid_width,
            dash: None,
        },
    );
    let (x_axis, x_axis_mesh, _) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        &[(Vec2::new(-AXIS_HALF_LENGTH, 0.0), Vec2::new(AXIS_HALF_LENGTH, 0.0))],
        LineStyle {
            color: palettes::tailwind::RED_400.into(),
            width: axis_width,
            dash: None,
        },
    );
    let (y_axis, y_axis_mesh, _) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        &[(Vec2::new(0.0, -AXIS_HALF_LENGTH), Vec2::new(0.0, AXIS_HALF_LENGTH))],
        LineStyle {
            color: palettes::tailwind::GREEN_400.into(),
            width: axis_width,
            dash: None,
        },
    );

    commands.entity(root).add_children(&[lines, x_axis, y_axis]);

    GridOverlayState {
        lines: lines_mesh,
        x_axis: x_axis_mesh,
        y_axis: y_axis_mesh,
        segments,
        columns,
        rows,
        built_scale: scale,
        fixed_width: fixed,
    }
}

/// 构建省界覆盖层的子实体。
fn spawn_province_child(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<OverlayLineMaterial>,
    root: Entity,
    btl: &Btl,
    bin: &Bin,
    scale: f32,
    fixed: bool,
) -> ProvinceOverlayState {
    let segments = province_boundary_segments(btl, bin);
    let (child, mesh, material) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        &segments,
        LineStyle {
            color: palettes::tailwind::YELLOW_300.into(),
            width: world_length(PROVINCE_LINE_WIDTH, scale, fixed),
            dash: Some(dash_lengths(PROVINCE_DASH, scale, fixed)),
        },
    );

    commands.entity(root).add_child(child);

    ProvinceOverlayState {
        mesh,
        material,
        segments,
        built_scale: scale,
        fixed_width: fixed,
    }
}

/// 构建国界覆盖层的子实体。
fn spawn_country_child(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<OverlayLineMaterial>,
    root: Entity,
    btl: &Btl,
    bin: &Bin,
    scale: f32,
    fixed: bool,
) -> CountryOverlayState {
    let segments = country_boundary_segments(btl, bin);
    let (child, mesh, _) = spawn_line_mesh(
        commands,
        meshes,
        materials,
        &segments,
        LineStyle {
            color: palettes::tailwind::SLATE_900.into(),
            width: world_length(COUNTRY_LINE_WIDTH, scale, fixed),
            dash: None,
        },
    );

    commands.entity(root).add_child(child);

    CountryOverlayState {
        mesh,
        segments,
        built_scale: scale,
        fixed_width: fixed,
    }
}

/// 就地更新网格覆盖层的线段宽度。
fn rebuild_grid_meshes(
    meshes: &mut Assets<Mesh>,
    state: &mut GridOverlayState,
    scale: f32,
    fixed: bool,
) {
    let grid_width = world_length(GRID_LINE_WIDTH, scale, fixed);
    let axis_width = world_length(AXIS_LINE_WIDTH, scale, fixed);

    rebuild_line_mesh(meshes, &state.lines, &state.segments, grid_width);
    rebuild_line_mesh(
        meshes,
        &state.x_axis,
        &[(Vec2::new(-AXIS_HALF_LENGTH, 0.0), Vec2::new(AXIS_HALF_LENGTH, 0.0))],
        axis_width,
    );
    rebuild_line_mesh(
        meshes,
        &state.y_axis,
        &[(Vec2::new(0.0, -AXIS_HALF_LENGTH), Vec2::new(0.0, AXIS_HALF_LENGTH))],
        axis_width,
    );

    state.built_scale = scale;
    state.fixed_width = fixed;
}

/// 复用已有网格资源重建线段几何。
fn rebuild_line_mesh(
    meshes: &mut Assets<Mesh>,
    mesh: &Handle<Mesh>,
    segments: &[(Vec2, Vec2)],
    width: f32,
) {
    if let Some(mut mesh_asset) = meshes.get_mut(mesh) {
        *mesh_asset = line_quad_mesh(segments, width);
    }
}

/// 更新虚线的世界长度。
fn sync_dash_style(
    materials: &mut Assets<OverlayLineMaterial>,
    material: &Handle<OverlayLineMaterial>,
    dash: (f32, f32),
) {
    if let Some(mut material) = materials.get_mut(material) {
        material.style.dash_length = dash.0;
        material.style.gap_length = dash.1;
    }
}

/// 将逻辑像素定义的长度换算为世界长度。
///
/// `fixed` 为真时按屏幕像素固定，随缩放换算；否则按世界尺寸固定，
/// 线宽与虚线随地图一同缩放。
fn world_length(pixels: f32, scale: f32, fixed: bool) -> f32 {
    if fixed {
        pixels * scale
    } else {
        pixels
    }
}

/// 将逻辑像素定义的虚线长度换算为世界长度。
fn dash_lengths(dash: (f32, f32), scale: f32, fixed: bool) -> (f32, f32) {
    (world_length(dash.0, scale, fixed), world_length(dash.1, scale, fixed))
}

/// 返回缩放变化超过阈值后是否需要重建线段。
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
    boundary_segments(btl.master.map_width, btl.master.map_height, |cell| {
        province_at(btl, bin, cell).filter(|province| *province >= 0)
    })
}

/// 生成国界线段：仅保留两侧陆地归属不同的地块共享边。
///
/// 水域不参与国界，避免沿海地块描出整条海岸线；无主陆地视为独立区域。
fn country_boundary_segments(btl: &Btl, bin: &Bin) -> Vec<(Vec2, Vec2)> {
    let owners = TileOwners::resolve(btl, bin);

    boundary_segments(btl.master.map_width, btl.master.map_height, |cell| {
        (!water_at(btl, bin, cell)).then(|| owners.get(cell))
    })
}

/// 收集两侧区域不同的地块共享边。
fn boundary_segments<K: PartialEq>(
    columns: i32,
    rows: i32,
    region: impl Fn(IVec2) -> Option<K>,
) -> Vec<(Vec2, Vec2)> {
    let mut segments = Vec::new();

    for y in 0..rows {
        for x in 0..columns {
            let cell = IVec2::new(x, y);
            let Some(current) = region(cell) else {
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

                let Some(other) = region(neighbor) else {
                    continue;
                };
                if current == other {
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

/// 将线段列表转换为带像素宽度与沿线距离的三角形网格。
fn line_quad_mesh(segments: &[(Vec2, Vec2)], width: f32) -> Mesh {
    let half_width = width.max(0.1) * 0.5;
    let count = segments.len().max(1);
    let mut positions = Vec::with_capacity(count * 4);
    let mut uvs = Vec::with_capacity(count * 4);
    let mut normals = Vec::with_capacity(count * 4);
    let mut indices = Vec::with_capacity(count * 6);

    // 无边界时保留一个退化线段，使网格资源仍可安全更新。
    for (start, end) in
        segments.iter().copied().chain(segments.is_empty().then_some((Vec2::ZERO, Vec2::ZERO)))
    {
        let delta = end - start;
        let length = delta.length();
        let normal = if length > f32::EPSILON {
            Vec2::new(-delta.y, delta.x) / length * half_width
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
        // uv.x 记录沿线方向的世界距离，供虚线着色器使用。
        uvs.extend([[0.0, 0.0], [0.0, 1.0], [length, 0.0], [length, 1.0]]);
        normals.extend([[0.0, 0.0, 1.0]; 4]);
        indices.extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    // 线段需要在缩放时原地更新，因此保留主世界副本。
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// 生成使用指定样式的线段实体，并返回实体与网格、材质句柄。
fn spawn_line_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<OverlayLineMaterial>,
    segments: &[(Vec2, Vec2)],
    style: LineStyle,
) -> (Entity, Handle<Mesh>, Handle<OverlayLineMaterial>) {
    let mesh = meshes.add(line_quad_mesh(segments, style.width));
    let material = materials.add(OverlayLineMaterial {
        style: OverlayLineStyle::new(style.color, style.dash),
    });

    let entity = commands
        .spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();

    (entity, mesh, material)
}

/// 返回线性空间的线条颜色。
fn linear_color(color: Color) -> Vec4 {
    let color = color.to_linear();

    Vec4::new(color.red, color.green, color.blue, color.alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_length_is_screen_fixed_only_when_locked() {
        assert!((world_length(2.0, 4.0, false) - 2.0).abs() < f32::EPSILON);
        assert!((world_length(2.0, 4.0, true) - 8.0).abs() < f32::EPSILON);

        assert_eq!(dash_lengths((9.0, 6.0), 2.0, false), (9.0, 6.0));
        assert_eq!(dash_lengths((9.0, 6.0), 2.0, true), (18.0, 12.0));
    }

    #[test]
    fn needs_rebuild_tracks_relative_scale_change() {
        assert!(needs_rebuild(0.0, 1.0));
        assert!(!needs_rebuild(1.0, 1.01));
        assert!(needs_rebuild(1.0, 1.5));
    }

    #[test]
    fn boundary_segments_draws_each_shared_edge_once() {
        assert!(boundary_segments(1, 1, |_| Some(0)).is_empty());

        let segments = boundary_segments(2, 1, |cell| Some(cell.x));
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn line_quad_mesh_records_along_line_distance_in_uv() {
        let mesh = line_quad_mesh(&[(Vec2::ZERO, Vec2::new(3.0, 4.0))], 1.0);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("线段网格缺少 uv 属性");
        };

        assert_eq!(uvs.len(), 4);
        assert!((uvs[2][0] - 5.0).abs() < f32::EPSILON);
    }
}

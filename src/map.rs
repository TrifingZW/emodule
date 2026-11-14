//! 地图网格覆盖层。

use bevy::{
    asset::RenderAssetUsages,
    color::palettes,
    mesh::{Mesh, PrimitiveTopology},
    prelude::*,
    sprite_render::AlphaMode2d,
};

use crate::{
    state::EditorState,
    wc4::{Btl, INNER_RADIUS, OUTER_RADIUS, Z_GRID, grid_to_world},
};

/// 世界坐标轴线的半长。
const AXIS_HALF_LENGTH: f32 = 8192.0;

/// 标记网格覆盖层根实体。
#[derive(Component, Clone, Default)]
pub(crate) struct GridOverlay;

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
        .spawn((GridOverlay, Transform::from_xyz(0.0, 0.0, Z_GRID), Visibility::Visible))
        .id();

    let lines = spawn_line_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        hex_segments(columns, rows),
        palettes::tailwind::GRAY_700.into(),
    );
    let x_axis = spawn_line_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        vec![(Vec2::new(-AXIS_HALF_LENGTH, 0.0), Vec2::new(AXIS_HALF_LENGTH, 0.0))],
        palettes::tailwind::RED_900.into(),
    );
    let y_axis = spawn_line_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        vec![(Vec2::new(0.0, -AXIS_HALF_LENGTH), Vec2::new(0.0, AXIS_HALF_LENGTH))],
        palettes::tailwind::GREEN_900.into(),
    );

    commands.entity(root).add_children(&[lines, x_axis, y_axis]);
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
fn line_mesh(segments: Vec<(Vec2, Vec2)>) -> Mesh {
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

/// 生成使用指定颜色渲染的线段实体。
fn spawn_line_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    segments: Vec<(Vec2, Vec2)>,
    color: Color,
) -> Entity {
    let material = materials.add(ColorMaterial {
        color,
        alpha_mode: AlphaMode2d::Blend,
        ..default()
    });

    commands
        .spawn((
            Mesh2d(meshes.add(line_mesh(segments))),
            MeshMaterial2d(material),
            Transform::default(),
            Visibility::Inherited,
        ))
        .id()
}

//! 地图网格覆盖层。

use bevy::{color::palettes, math::Isometry2d, prelude::*};

use crate::state::EditorState;

/// 每个轴上的网格单元数量。
const GRID_CELLS: UVec2 = UVec2::splat(256);

/// 每个网格单元的世界单位尺寸。
const GRID_CELL_SIZE: f32 = 64.0;

/// 世界坐标轴线的半长。
const AXIS_HALF_LENGTH: f32 = 8192.0;

/// 在网格启用时绘制网格与世界坐标轴。
pub(crate) fn draw_grid(state: Res<EditorState>, mut gizmos: Gizmos) {
    if !state.grid_enabled() {
        return;
    }

    let _ = gizmos.grid_2d(
        Isometry2d::IDENTITY,
        GRID_CELLS,
        Vec2::splat(GRID_CELL_SIZE),
        palettes::tailwind::GRAY_700,
    );

    gizmos.line_2d(
        Vec2::new(-AXIS_HALF_LENGTH, 0.0),
        Vec2::new(AXIS_HALF_LENGTH, 0.0),
        palettes::tailwind::RED_900,
    );
    gizmos.line_2d(
        Vec2::new(0.0, -AXIS_HALF_LENGTH),
        Vec2::new(0.0, AXIS_HALF_LENGTH),
        palettes::tailwind::GREEN_900,
    );
}

//! WC4 六边形网格坐标换算。

use bevy::prelude::*;

/// 六边形网格的纵向压缩比例（`√3 / 2`）。
pub(crate) const GRID_RATE: f32 = 0.866_025_4;

/// 单个网格的像素宽度。
pub(crate) const GRID_WIDTH: f32 = 148.0;

/// 单个网格的像素高度。
pub(crate) const GRID_HEIGHT: f32 = GRID_WIDTH * GRID_RATE;

/// 六边形边长。
pub(crate) const SIDE_LENGTH: f32 = GRID_WIDTH / 2.0;

/// 六边形外接圆半径。
pub(crate) const OUTER_RADIUS: f32 = SIDE_LENGTH;

/// 六边形内切圆半径。
pub(crate) const INNER_RADIUS: f32 = OUTER_RADIUS * GRID_RATE;

/// 将网格坐标换算为网格中心的世界坐标。
pub(crate) fn grid_to_world(x: i32, y: i32) -> Vec2 {
    let column_offset = x.rem_euclid(2) as f32 * INNER_RADIUS;

    Vec2::new(
        OUTER_RADIUS + x as f32 * OUTER_RADIUS * 1.5,
        -(INNER_RADIUS + y as f32 * GRID_HEIGHT + column_offset),
    )
}

/// 计算网格区域在世界坐标中的尺寸。
pub(crate) fn grid_world_size(columns: usize, rows: usize) -> Vec2 {
    let overlapping_columns = columns.saturating_sub(1) as f32;
    let columns = columns as f32;

    Vec2::new(
        columns * GRID_WIDTH - overlapping_columns * (SIDE_LENGTH / 2.0),
        (rows as f32 + 0.5) * GRID_HEIGHT,
    )
}

/// 将网格坐标换算为行优先索引。
pub(crate) fn grid_to_index(x: usize, y: usize, width: usize) -> usize {
    y * width + x
}

/// 将行优先索引换算为网格坐标。
pub(crate) fn index_to_grid(index: usize, width: usize) -> IVec2 {
    IVec2::new((index % width) as i32, (index / width) as i32)
}

/// 将世界坐标换算为最近的网格坐标。
pub(crate) fn world_to_grid(position: Vec2) -> IVec2 {
    // 去掉首个网格的中心偏移，再转换为 y 向下的轴坐标。
    let pixel = Vec2::new(position.x - OUTER_RADIUS, -position.y - INNER_RADIUS);
    let q = pixel.x / (OUTER_RADIUS * 1.5);
    let r = pixel.y / GRID_HEIGHT - q / 2.0;

    // 轴坐标取整为最近的六边形中心，可保证任意多边形内部点回到自身格子。
    let (q, r) = round_axial(q, r);
    let row = r + (q - (q & 1)) / 2;

    IVec2::new(q, row)
}

/// 将窗口光标位置换算为网格坐标。
///
/// 光标不在窗口内，或相机无法投影该位置时返回 `None`。
pub(crate) fn cursor_to_grid(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<IVec2> {
    let viewport_position = window.cursor_position()?;
    let world_position = camera.viewport_to_world_2d(camera_transform, viewport_position).ok()?;

    Some(world_to_grid(world_position))
}

/// 将分数轴坐标取整为最近的整数轴坐标。
fn round_axial(q: f32, r: f32) -> (i32, i32) {
    let s = -q - r;
    let mut rounded_q = q.round();
    let mut rounded_r = r.round();
    let rounded_s = s.round();

    let q_delta = (rounded_q - q).abs();
    let r_delta = (rounded_r - r).abs();
    let s_delta = (rounded_s - s).abs();

    if q_delta > r_delta && q_delta > s_delta {
        rounded_q = -rounded_r - rounded_s;
    } else if r_delta > s_delta {
        rounded_r = -rounded_q - rounded_s;
    }

    (rounded_q as i32, rounded_r as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_to_world_places_the_origin_at_the_first_cell_center() {
        assert_eq!(grid_to_world(0, 0), Vec2::new(OUTER_RADIUS, -INNER_RADIUS));
    }

    #[test]
    fn world_to_grid_inverts_grid_to_world_for_centers() {
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(world_to_grid(grid_to_world(x, y)), IVec2::new(x, y));
            }
        }
    }

    #[test]
    fn world_to_grid_stays_within_the_source_hex() {
        for y in 0..8 {
            for x in 0..8 {
                let center = grid_to_world(x, y);

                for dy in (-64..=64).step_by(2) {
                    for dx in (-74..=74).step_by(2) {
                        let offset = Vec2::new(dx as f32, dy as f32);

                        // 仅采样从六边形边界向内收缩 1.5 单位的区域。
                        if offset.x.abs() + (37.0 / 64.25) * offset.y.abs() <= 72.5 {
                            assert_eq!(
                                world_to_grid(center + offset),
                                IVec2::new(x, y),
                                "cell=({x},{y}) offset={offset:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn world_to_grid_maps_all_hex_corners_to_their_cell() {
        for (x, y) in [(9, 0), (10, 0), (11, 0), (0, 0), (1, 1)] {
            let center = grid_to_world(x, y);

            for offset in [
                Vec2::new(36.0, 34.0),
                Vec2::new(36.0, -34.0),
                Vec2::new(-36.0, 34.0),
                Vec2::new(-36.0, -34.0),
            ] {
                assert_eq!(
                    world_to_grid(center + offset),
                    IVec2::new(x, y),
                    "cell=({x},{y}) offset={offset:?}"
                );
            }
        }
    }

    #[test]
    fn index_conversions_round_trip() {
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(
                    index_to_grid(grid_to_index(x, y, 4), 4),
                    IVec2::new(x as i32, y as i32)
                );
            }
        }
    }

    #[test]
    fn grid_world_size_accounts_for_column_overlap() {
        assert_eq!(grid_world_size(1, 1).x, GRID_WIDTH);
        assert_eq!(grid_world_size(2, 1).x, GRID_WIDTH * 2.0 - SIDE_LENGTH / 2.0);
        assert_eq!(grid_world_size(0, 1).x, 0.0);
    }
}

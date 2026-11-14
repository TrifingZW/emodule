//! 地块信息与可编辑字段。

use bevy::prelude::*;

use crate::wc4::{
    bin::Bin,
    btl::Btl,
    btl_data::Master,
    grid::grid_to_index,
    numeric::{clamp_i8, clamp_i16, clamp_u8},
};

/// 地块的可编辑字段名称，顺序与访问器中的字段下标一致。
const TILE_FIELDS: &[&str] = &[
    "Terrain",
    "Terrain Id",
    "Offset X",
    "Offset Y",
    "Decoration A",
    "Decoration A Id",
    "Decoration A Offset X",
    "Decoration A Offset Y",
    "Decoration B",
    "Decoration B Id",
    "Decoration B Offset X",
    "Decoration B Offset Y",
    "Province",
    "Belong",
];

/// 选中地块的只读摘要。
pub(crate) struct TileSummary {
    /// 网格坐标。
    pub(crate) grid: IVec2,
    /// 地图绝对坐标。
    pub(crate) map: IVec2,
    /// 水域标记。
    pub(crate) water: bool,
    /// 道路标记。
    pub(crate) road: bool,
    /// 海岸标记。
    pub(crate) coast: bool,
}

/// 汇总地块信息。
///
/// 网格坐标无效或地图数据尚未加载时返回 `None`。
pub(crate) fn tile_summary(btl: &Btl, bin: &Bin, grid: IVec2) -> Option<TileSummary> {
    let map = map_position(&btl.master, grid)?;
    let topography = bin.topography(map.x, map.y)?;

    Some(TileSummary {
        grid,
        map,
        water: topography.water != 0,
        road: topography.road != 0,
        coast: topography.coast != 0,
    })
}

/// 返回地块可编辑字段的名称。
pub(crate) fn tile_field_names() -> &'static [&'static str] {
    TILE_FIELDS
}

/// 读取地块字段值。
///
/// 网格坐标无效或字段不存在时返回 `None`。
pub(crate) fn tile_field_value(btl: &Btl, bin: &Bin, grid: IVec2, field: usize) -> Option<i32> {
    let index = topography_index(btl, bin, grid)?;
    let topography = bin.topographies.get(index)?;

    Some(match field {
        0 => i32::from(topography.r#type),
        1 => i32::from(topography.id),
        2 => i32::from(topography.x),
        3 => i32::from(topography.y),
        4 => i32::from(topography.decoration_a),
        5 => i32::from(topography.decoration_a_id),
        6 => i32::from(topography.decoration_a_x),
        7 => i32::from(topography.decoration_a_y),
        8 => i32::from(topography.decoration_b),
        9 => i32::from(topography.decoration_b_id),
        10 => i32::from(topography.decoration_b_x),
        11 => i32::from(topography.decoration_b_y),
        12 => i32::from(*btl.province_vec.get(scene_index(btl, bin, grid)?)?),
        13 => i32::from(*btl.belong_vec.get(scene_index(btl, bin, grid)?)?),
        _ => return None,
    })
}

/// 写入地块字段值。
///
/// 网格坐标或字段不存在时返回 `false`；数值超出字段类型范围时截断到边界。
pub(crate) fn set_tile_field(
    btl: &mut Btl,
    bin: &mut Bin,
    grid: IVec2,
    field: usize,
    value: i32,
) -> bool {
    match field {
        0..=11 => {
            let Some(index) = topography_index(btl, bin, grid) else {
                return false;
            };
            let Some(topography) = bin.topographies.get_mut(index) else {
                return false;
            };

            match field {
                0 => topography.r#type = clamp_u8(value),
                1 => topography.id = clamp_u8(value),
                2 => topography.x = clamp_i8(value),
                3 => topography.y = clamp_i8(value),
                4 => topography.decoration_a = clamp_u8(value),
                5 => topography.decoration_a_id = clamp_u8(value),
                6 => topography.decoration_a_x = clamp_i8(value),
                7 => topography.decoration_a_y = clamp_i8(value),
                8 => topography.decoration_b = clamp_u8(value),
                9 => topography.decoration_b_id = clamp_u8(value),
                10 => topography.decoration_b_x = clamp_i8(value),
                11 => topography.decoration_b_y = clamp_i8(value),
                _ => return false,
            }

            true
        }
        12 => {
            let Some(index) = scene_index(btl, bin, grid) else {
                return false;
            };
            let Some(province) = btl.province_vec.get_mut(index) else {
                return false;
            };

            *province = clamp_i16(value);
            true
        }
        13 => {
            let Some(index) = scene_index(btl, bin, grid) else {
                return false;
            };
            let Some(belong) = btl.belong_vec.get_mut(index) else {
                return false;
            };

            *belong = clamp_u8(value);
            true
        }
        _ => false,
    }
}

/// 将网格坐标换算为地图绝对坐标，坐标超出地图范围时返回 `None`。
pub(super) fn map_position(master: &Master, grid: IVec2) -> Option<IVec2> {
    if grid.x < 0 || grid.y < 0 || grid.x >= master.map_width || grid.y >= master.map_height {
        return None;
    }

    Some(IVec2::new(
        master.map_capture_x.checked_add(grid.x)?,
        master.map_capture_y.checked_add(grid.y)?,
    ))
}

/// 返回网格坐标在地形数据中的行优先索引。
///
/// 索引按世界宽度计算，与地形数据以及依附对象的坐标一致。
pub(super) fn tile_index(btl: &Btl, bin: &Bin, grid: IVec2) -> Option<i32> {
    let map = map_position(&btl.master, grid)?;
    bin.topography(map.x, map.y)?;

    let width = usize::try_from(bin.info.width).ok()?;
    let x = usize::try_from(map.x).ok()?;
    let y = usize::try_from(map.y).ok()?;

    i32::try_from(grid_to_index(x, y, width)).ok()
}

/// 返回网格坐标在地形数据中的行优先索引，用于下标访问 `topographies`。
fn topography_index(btl: &Btl, bin: &Bin, grid: IVec2) -> Option<usize> {
    usize::try_from(tile_index(btl, bin, grid)?).ok()
}

/// 返回网格坐标在场景数据中的行优先索引。
///
/// 省份与归属数组可能按地图宽度或世界宽度排列，按数组长度选择对应的索引空间。
fn scene_index(btl: &Btl, bin: &Bin, grid: IVec2) -> Option<usize> {
    let map = map_position(&btl.master, grid)?;
    let map_width = usize::try_from(btl.master.map_width).ok()?;
    let map_count = map_width.checked_mul(usize::try_from(btl.master.map_height).ok()?)?;

    let (x, y, width) = if btl.province_vec.len() == map_count {
        (grid.x, grid.y, btl.master.map_width)
    } else {
        (map.x, map.y, bin.info.width)
    };

    Some(grid_to_index(
        usize::try_from(x).ok()?,
        usize::try_from(y).ok()?,
        usize::try_from(width).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wc4::bin::{BinInfo, Topography};
    use crate::wc4::btl_data::Master;

    const WIDTH: i32 = 4;

    fn bin(width: i32, height: i32, topographies: Vec<Topography>) -> Bin {
        Bin {
            info: BinInfo {
                width,
                height,
                ..default()
            },
            topographies,
        }
    }

    fn btl(width: i32, height: i32) -> Btl {
        let count = usize::try_from(width * height).unwrap();

        Btl {
            master: Master {
                map_width: width,
                map_height: height,
                ..default()
            },
            province_vec: vec![0; count],
            belong_vec: vec![0; count],
            ..default()
        }
    }

    fn fixtures() -> (Btl, Bin) {
        let mut bin = bin(WIDTH, 4, vec![Topography::default(); 16]);
        bin.topographies[5] = Topography {
            r#type: 2,
            id: 3,
            ..default()
        };

        (btl(WIDTH, 4), bin)
    }

    #[test]
    fn summary_describes_the_selected_tile() {
        let (btl, bin) = fixtures();
        let summary = tile_summary(&btl, &bin, IVec2::new(1, 1)).unwrap();

        assert_eq!(summary.grid, IVec2::new(1, 1));
        assert_eq!(summary.map, IVec2::new(1, 1));
        assert!(!summary.water);
    }

    #[test]
    fn summary_rejects_positions_outside_the_map() {
        let (btl, bin) = fixtures();

        assert!(tile_summary(&btl, &bin, IVec2::new(-1, 0)).is_none());
        assert!(tile_summary(&btl, &bin, IVec2::new(WIDTH, 0)).is_none());
    }

    #[test]
    fn every_named_field_round_trips() {
        let (mut btl, mut bin) = fixtures();
        let grid = IVec2::new(1, 1);

        for field in 0..tile_field_names().len() {
            assert!(
                tile_field_value(&btl, &bin, grid, field).is_some(),
                "field {field} is not readable"
            );
            assert!(
                set_tile_field(&mut btl, &mut bin, grid, field, 7),
                "field {field} is not writable"
            );
            assert_eq!(tile_field_value(&btl, &bin, grid, field), Some(7));
        }
    }

    #[test]
    fn set_tile_field_clamps_to_the_field_range() {
        let (mut btl, mut bin) = fixtures();
        let grid = IVec2::new(1, 1);

        assert!(set_tile_field(&mut btl, &mut bin, grid, 0, 999));
        assert_eq!(tile_field_value(&btl, &bin, grid, 0), Some(i32::from(u8::MAX)));

        assert!(set_tile_field(&mut btl, &mut bin, grid, 2, 999));
        assert_eq!(tile_field_value(&btl, &bin, grid, 2), Some(i32::from(i8::MAX)));
    }

    #[test]
    fn unknown_fields_and_positions_are_rejected() {
        let (mut btl, mut bin) = fixtures();

        assert_eq!(tile_field_value(&btl, &bin, IVec2::new(1, 1), tile_field_names().len()), None);
        assert!(!set_tile_field(&mut btl, &mut bin, IVec2::new(1, 1), tile_field_names().len(), 1));
        assert!(!set_tile_field(&mut btl, &mut bin, IVec2::new(9, 9), 0, 1));
    }

    #[test]
    fn province_and_belong_use_the_map_index_space() {
        let (mut btl, mut bin) = fixtures();
        let grid = IVec2::new(2, 1);

        assert!(set_tile_field(&mut btl, &mut bin, grid, 12, 42));
        assert!(set_tile_field(&mut btl, &mut bin, grid, 13, 5));
        assert_eq!(btl.province_vec[6], 42);
        assert_eq!(btl.belong_vec[6], 5);
    }

    #[test]
    fn province_arrays_in_world_space_use_the_world_index() {
        let mut btl = btl(2, 2);
        btl.province_vec = vec![0; 64];
        btl.belong_vec = vec![0; 64];
        let mut bin = bin(8, 8, vec![Topography::default(); 64]);

        assert!(set_tile_field(&mut btl, &mut bin, IVec2::new(1, 1), 12, 9));
        assert_eq!(btl.province_vec[9], 9);
        assert_eq!(tile_field_value(&btl, &bin, IVec2::new(1, 1), 12), Some(9));
    }
}

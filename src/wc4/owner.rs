//! 地块归属国家的解析。
//!
//! `Btl::belong_vec` 只在地块上存在城市、部队等实体时记录国家，其余地块为
//! [`NO_OWNER`]。渲染所需的归属因此按省份补齐：先取省份代表地块的归属，
//! 代表地块优先选择城市所在地块，其次取该省第一个记录了归属的地块。

use std::collections::HashMap;

use bevy::prelude::*;

use super::{
    bin::Bin,
    btl::Btl,
    btl_data::City,
    grid::index_to_grid,
    tile::{belong_at, grid_position, province_at},
};

/// 地块没有归属时 `Btl::belong_vec` 的取值。
const NO_OWNER: u8 = u8::MAX;

/// 地图区域内每个地块的归属国家。
#[derive(Debug, Default)]
pub(crate) struct TileOwners {
    /// 地图区域的网格维度。
    extent: UVec2,
    /// 行优先排列的归属国家，`None` 表示无主。
    owners: Vec<Option<u8>>,
}

impl TileOwners {
    /// 解析整张地图的地块归属。
    ///
    /// 地图尺寸无效或国家表为空时，所有地块都视为无主。
    pub(crate) fn resolve(btl: &Btl, bin: &Bin) -> Self {
        let columns = btl.master.map_width;
        let rows = btl.master.map_height;

        if columns <= 0 || rows <= 0 || btl.country_vec.is_empty() {
            return Self::default();
        }

        let (Ok(extent_x), Ok(extent_y)) = (u32::try_from(columns), u32::try_from(rows)) else {
            return Self::default();
        };

        let provinces = province_owners(btl, bin);
        let owners = (0..rows)
            .flat_map(|row| (0..columns).map(move |column| IVec2::new(column, row)))
            // 地块自身记录了归属时直接采用，否则继承省份代表地块的归属。
            .map(|grid| {
                direct_owner(btl, bin, grid).or_else(|| province_owner(btl, bin, grid, &provinces))
            })
            .collect();

        Self {
            extent: UVec2::new(extent_x, extent_y),
            owners,
        }
    }

    /// 返回网格坐标的归属国家。
    pub(crate) fn get(&self, grid: IVec2) -> Option<u8> {
        self.owners.get(self.index(grid)?).copied().flatten()
    }

    /// 返回有归属的地块数量。
    pub(crate) fn owned(&self) -> usize {
        self.owners.iter().filter(|owner| owner.is_some()).count()
    }

    /// 返回网格坐标在归属表中的行优先下标。
    fn index(&self, grid: IVec2) -> Option<usize> {
        let x = u32::try_from(grid.x).ok().filter(|x| *x < self.extent.x)?;
        let y = u32::try_from(grid.y).ok().filter(|y| *y < self.extent.y)?;
        let width = usize::try_from(self.extent.x).ok()?;

        usize::try_from(y).ok()?.checked_mul(width)?.checked_add(usize::try_from(x).ok()?)
    }
}

/// 返回地块自身记录的归属国家。
///
/// [`NO_OWNER`] 与超出国家表范围的下标都视为无主。
fn direct_owner(btl: &Btl, bin: &Bin, grid: IVec2) -> Option<u8> {
    belong_at(btl, bin, grid).filter(|belong| is_country(btl, *belong))
}

/// 返回地块所在省份代表地块的归属国家。
fn province_owner(btl: &Btl, bin: &Bin, grid: IVec2, provinces: &HashMap<i16, u8>) -> Option<u8> {
    let province = province_at(btl, bin, grid).filter(|province| *province >= 0)?;

    provinces.get(&province).copied()
}

/// 判断归属取值是否指向国家表中的国家。
fn is_country(btl: &Btl, belong: u8) -> bool {
    belong != NO_OWNER && usize::from(belong) < btl.country_vec.len()
}

/// 收集每个省份代表地块的归属国家。
///
/// 代表地块优先取城市所在地块（同一省份有多个城市时取 `city_vec` 顺序中的第一个），
/// 其次取行优先顺序上第一个有归属的地块。
fn province_owners(btl: &Btl, bin: &Bin) -> HashMap<i16, u8> {
    let mut owners = HashMap::new();

    for city in &btl.city_vec {
        if let Some(grid) = city_grid(btl, bin, city) {
            bind_province(btl, bin, grid, &mut owners);
        }
    }

    for row in 0..btl.master.map_height {
        for column in 0..btl.master.map_width {
            bind_province(btl, bin, IVec2::new(column, row), &mut owners);
        }
    }

    owners
}

/// 用地块的归属登记其所在省份，已登记的省份保持不变。
fn bind_province(btl: &Btl, bin: &Bin, grid: IVec2, owners: &mut HashMap<i16, u8>) {
    let Some(province) = province_at(btl, bin, grid).filter(|province| *province >= 0) else {
        return;
    };
    let Some(owner) = direct_owner(btl, bin, grid) else {
        return;
    };

    owners.entry(province).or_insert(owner);
}

/// 返回城市所在地块的网格坐标，坐标不可用或不在可视地图内时返回 `None`。
fn city_grid(btl: &Btl, bin: &Bin, city: &City) -> Option<IVec2> {
    let index = usize::try_from(city.coordinate).ok()?;
    let width = usize::try_from(bin.info.width).ok()?;

    grid_position(&btl.master, index_to_grid(index, width))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wc4::{
        bin::{BinInfo, Topography},
        btl_data::{City, Country, Master},
    };

    const COLUMNS: i32 = 3;

    /// 构造单行三省的地图，省份与归属按列给出，城市以世界索引给出。
    fn scene(
        provinces: [i16; COLUMNS as usize],
        belongs: [u8; COLUMNS as usize],
        cities: &[i16],
    ) -> (Btl, Bin) {
        (
            Btl {
                master: Master {
                    map_width: COLUMNS,
                    map_height: 1,
                    total_tiles: COLUMNS,
                    ..default()
                },
                country_vec: (0..3).map(|_| Country::default()).collect(),
                province_vec: provinces.to_vec(),
                belong_vec: belongs.to_vec(),
                city_vec: cities
                    .iter()
                    .map(|&coordinate| City {
                        coordinate,
                        ..default()
                    })
                    .collect(),
                ..default()
            },
            Bin {
                info: BinInfo {
                    width: COLUMNS,
                    height: 1,
                    ..default()
                },
                topographies: vec![Topography::default(); COLUMNS as usize],
            },
        )
    }

    #[test]
    fn province_inherits_its_representative_owner() {
        let (btl, bin) = scene([7, 7, 7], [1, NO_OWNER, NO_OWNER], &[]);
        let owners = TileOwners::resolve(&btl, &bin);

        assert_eq!(owners.get(IVec2::new(0, 0)), Some(1));
        assert_eq!(owners.get(IVec2::new(1, 0)), Some(1));
        assert_eq!(owners.get(IVec2::new(2, 0)), Some(1));
        assert_eq!(owners.owned(), 3);
    }

    #[test]
    fn city_tile_represents_the_province_before_other_tiles() {
        let (btl, bin) = scene([7, 7, 7], [1, 2, NO_OWNER], &[1]);
        let owners = TileOwners::resolve(&btl, &bin);

        assert_eq!(owners.get(IVec2::new(0, 0)), Some(1));
        assert_eq!(owners.get(IVec2::new(1, 0)), Some(2));
        assert_eq!(owners.get(IVec2::new(2, 0)), Some(2));
    }

    #[test]
    fn tiles_without_province_keep_their_own_owner() {
        let (btl, bin) = scene([-1, -1, -1], [2, NO_OWNER, NO_OWNER], &[]);
        let owners = TileOwners::resolve(&btl, &bin);

        assert_eq!(owners.get(IVec2::new(0, 0)), Some(2));
        assert_eq!(owners.get(IVec2::new(1, 0)), None);
        assert_eq!(owners.owned(), 1);
    }

    #[test]
    fn invalid_country_index_is_treated_as_unowned() {
        let (btl, bin) = scene([7, 7, 7], [9, NO_OWNER, NO_OWNER], &[]);
        let owners = TileOwners::resolve(&btl, &bin);

        assert_eq!(owners.get(IVec2::new(0, 0)), None);
        assert_eq!(owners.get(IVec2::new(1, 0)), None);
    }

    #[test]
    fn grid_outside_the_map_is_unowned() {
        let (btl, bin) = scene([7, 7, 7], [1, 1, 1], &[]);
        let owners = TileOwners::resolve(&btl, &bin);

        assert_eq!(owners.get(IVec2::new(-1, 0)), None);
        assert_eq!(owners.get(IVec2::new(0, 1)), None);
        assert_eq!(owners.get(IVec2::new(COLUMNS, 0)), None);
    }
}

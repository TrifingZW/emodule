//! WC4 地图配置与城市名称表。

use std::{collections::HashMap, path::Path};

use bevy::{
    asset::{Asset, AssetServer, Handle},
    prelude::*,
    reflect::TypePath,
};
use ini::{Ini, Properties};
use serde::Deserialize;

use crate::wc4::Wc4Error;

/// 地形图集配置路径。
const TERRAIN_XML_PATH: &str = "config/def_mapterrain.xml";
/// 城市名称表路径。
const CITY_NAMES_PATH: &str = "assets/stringtable_cn.ini";
/// 城市名称在城市名称表中的键前缀。
const CITY_NAME_PREFIX: &str = "battle_cityname_";

/// 地图地形图集配置。
#[derive(Asset, TypePath, Debug, Deserialize)]
pub(crate) struct TerrainXml {
    /// 全部地形定义。
    #[serde(rename = "terrain")]
    pub(crate) terrains: Vec<Terrain>,
}

/// 一种地形的图集切片定义。
#[derive(Debug, Deserialize)]
pub(crate) struct Terrain {
    /// 地形类型编号。
    #[serde(rename = "@terrain")]
    pub(crate) terrain: i32,
    /// 该地形各变体对应的图集切片。
    #[serde(rename = "tile")]
    pub(crate) tiles: Vec<Tile>,
}

/// 地形变体的图集切片。
#[derive(Debug, Deserialize)]
pub(crate) struct Tile {
    /// 变体编号。
    #[serde(rename = "@idx")]
    pub(crate) idx: i32,
    /// 图集中对应的图片名称。
    #[serde(rename = "@image")]
    pub(crate) image: String,
}

/// 城市编号到名称的映射。
#[derive(Debug, Default, Resource)]
pub(crate) struct CityNames(HashMap<i16, String>);

impl CityNames {
    /// 返回指定编号的城市名称。
    pub(crate) fn name(&self, id: i16) -> Option<&str> {
        self.0.get(&id).map(String::as_str)
    }
}

/// WC4 地图配置资源。
#[derive(Resource)]
pub(crate) struct Wc4Config {
    /// 地形图集配置。
    pub(crate) terrain_xml: Handle<TerrainXml>,
    /// 城市名称表。
    pub(crate) city_names: CityNames,
}

/// 加载 WC4 地图配置。
pub(crate) fn load_wc4_config(mut commands: Commands, asset_server: Res<AssetServer>) {
    let city_names = match load_city_names(CITY_NAMES_PATH) {
        Ok(city_names) => city_names,
        Err(error) => {
            warn!("加载 WC4 城市名称表失败: {error}");
            CityNames::default()
        }
    };

    commands.insert_resource(Wc4Config {
        terrain_xml: asset_server.load(TERRAIN_XML_PATH),
        city_names,
    });
}

/// 从 INI 文件读取城市名称。
fn load_city_names(path: impl AsRef<Path>) -> Result<CityNames, Wc4Error> {
    let ini = Ini::load_from_file(path)?;

    Ok(ini.section::<String>(None).map(collect_city_names).unwrap_or_default())
}

/// 从 INI 的默认节中收集城市名称。
fn collect_city_names(properties: &Properties) -> CityNames {
    let names = properties
        .iter()
        .filter_map(|(key, value)| {
            let id = key.strip_prefix(CITY_NAME_PREFIX)?.parse::<i16>().ok()?;

            Some((id, value.to_owned()))
        })
        .collect();

    CityNames(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_city_names_keeps_only_numbered_city_entries() {
        let ini =
            Ini::load_from_str("battle_cityname_1=北京\nbattle_cityname_x=无效\nother=忽略\n")
                .unwrap();
        let names = collect_city_names(ini.section::<String>(None).unwrap());

        assert_eq!(names.name(1), Some("北京"));
        assert_eq!(names.name(2), None);
    }

    #[test]
    fn missing_city_names_section_yields_an_empty_table() {
        let ini = Ini::load_from_str("other=值\n").unwrap();
        let names = ini.section::<String>(None).map(collect_city_names).unwrap_or_default();

        assert_eq!(names.name(1), None);
    }
}

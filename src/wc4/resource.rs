//! WC4 地图图集资源。

use bevy::{
    asset::{Asset, AssetServer},
    image::Image,
    prelude::*,
    reflect::TypePath,
    text::Font,
};
use serde::Deserialize;

/// 地形图集配置。
const TERRAIN_XML_PATH: &str = "terrain_hd.xml";
/// 地形图集图片。
const TERRAIN_IMAGE_PATH: &str = "terrain_hd.webp";
/// 植被图集配置。
const PLANT_XML_PATH: &str = "plant_hd.xml";
/// 植被图集图片。
const PLANT_IMAGE_PATH: &str = "plant_hd.png";
/// 建筑图集配置。
const BUILDINGS_XML_PATH: &str = "buildings_hd.xml";
/// 建筑图集图片。
const BUILDINGS_IMAGE_PATH: &str = "buildings_hd.webp";
/// 陆地底图。
const LAND_IMAGE_PATH: &str = "map_land.png";
/// 海洋底图。
const SEA_IMAGE_PATH: &str = "map_sea.png";
/// 地块指示图标。
const CURSOR_IMAGE_PATH: &str = "anim_status.png";
/// 城市名称使用的字体。
const FONT_PATH: &str = "HarmonyOS_SansSC_Bold.ttf";

/// 图集切片配置。
#[derive(Asset, TypePath, Debug, Deserialize)]
pub(crate) struct ImageXml {
    /// 全部图集切片。
    #[serde(rename = "Image")]
    pub(crate) image_infos: Vec<ImageInfo>,
}

/// 图集中的单张图片。
#[derive(Debug, Deserialize)]
pub(crate) struct ImageInfo {
    /// 图片名称，用于与地形配置匹配。
    #[serde(rename = "@name")]
    pub(crate) name: String,
    /// 图片在纹理中的左上角 X。
    #[serde(rename = "@x")]
    pub(crate) x: i32,
    /// 图片在纹理中的左上角 Y。
    #[serde(rename = "@y")]
    pub(crate) y: i32,
    /// 图片宽度。
    #[serde(rename = "@w")]
    pub(crate) w: i32,
    /// 图片高度。
    #[serde(rename = "@h")]
    pub(crate) h: i32,
    /// 锚点 X。
    #[serde(rename = "@refx")]
    pub(crate) refx: i32,
    /// 锚点 Y。
    #[serde(rename = "@refy")]
    pub(crate) refy: i32,
}

/// 一组配套的图集配置与图片。
#[derive(Clone)]
pub(crate) struct Wc4Atlas {
    /// 图集切片配置。
    pub(crate) image_xml: Handle<ImageXml>,
    /// 图集纹理。
    pub(crate) image: Handle<Image>,
}

/// WC4 地图使用的全部资源。
#[derive(Resource)]
pub(crate) struct Wc4Resources {
    /// 地形图集。
    pub(crate) terrain: Wc4Atlas,
    /// 植被图集。
    pub(crate) plant: Wc4Atlas,
    /// 建筑图集。
    pub(crate) buildings: Wc4Atlas,
    /// 陆地底图。
    pub(crate) land: Handle<Image>,
    /// 海洋底图。
    pub(crate) sea: Handle<Image>,
    /// 地块指示图标。
    pub(crate) cursor: Handle<Image>,
    /// 城市名称字体。
    pub(crate) font: Handle<Font>,
}

/// 加载 WC4 地图资源。
pub(crate) fn load_wc4_resources(mut commands: Commands, asset_server: Res<AssetServer>) {
    let atlas = |image_xml, image| Wc4Atlas {
        image_xml: asset_server.load(image_xml),
        image: asset_server.load(image),
    };

    commands.insert_resource(Wc4Resources {
        terrain: atlas(TERRAIN_XML_PATH, TERRAIN_IMAGE_PATH),
        plant: atlas(PLANT_XML_PATH, PLANT_IMAGE_PATH),
        buildings: atlas(BUILDINGS_XML_PATH, BUILDINGS_IMAGE_PATH),
        land: asset_server.load(LAND_IMAGE_PATH),
        sea: asset_server.load(SEA_IMAGE_PATH),
        cursor: asset_server.load(CURSOR_IMAGE_PATH),
        font: asset_server.load(FONT_PATH),
    });
}

//! WC4 地图数据解析与渲染。
//!
//! 该模块迁移自旧项目 estarium 的 wc4 系列模块，提供世界地形（`.bin`）、
//! 场景数据（`.btl`）、地图配置与图集资源的加载，以及六边形地图的渲染。
//! 与 egui 绑定的旧编辑器界面不再保留，交互统一由编辑器的 Bevy UI 承担。

mod bin;
mod btl;
mod btl_data;
mod codec;
mod config;
mod grid;
mod numeric;
mod render;
mod resource;
pub(crate) mod selection;
mod tile;

use std::fmt;

use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy_common_assets::xml::XmlAssetPlugin;

use crate::state::SelectedTile;

pub(crate) use render::{DirtyTiles, refresh_tiles};

use crate::wc4::{
    bin::load_wc4_bin,
    btl::load_wc4_btl,
    config::{TerrainXml, load_wc4_config},
    render::{
        InstanceMaterial, SeaMaterial, Wc4MapSpawned, spawn_wc4_map, update_city_label_scale,
        update_selection_marker, update_tile_cursor,
    },
    resource::{ImageXml, load_wc4_resources},
};

/// WC4 数据加载或解析失败。
#[derive(Debug)]
pub(crate) enum Wc4Error {
    /// 文件读取失败。
    Io(std::io::Error),
    /// 二进制数据解码失败。
    Decode(bincode::error::DecodeError),
    /// 文本配置解析失败。
    Config(ini::Error),
    /// 数据内容不满足格式约束。
    Invalid(&'static str),
}

impl fmt::Display for Wc4Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "文件读取失败: {error}"),
            Self::Decode(error) => write!(formatter, "二进制数据解码失败: {error}"),
            Self::Config(error) => write!(formatter, "文本配置解析失败: {error}"),
            Self::Invalid(reason) => write!(formatter, "数据内容不合法: {reason}"),
        }
    }
}

impl std::error::Error for Wc4Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Decode(error) => Some(error),
            Self::Config(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<std::io::Error> for Wc4Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<bincode::error::DecodeError> for Wc4Error {
    fn from(error: bincode::error::DecodeError) -> Self {
        Self::Decode(error)
    }
}

impl From<ini::Error> for Wc4Error {
    fn from(error: ini::Error) -> Self {
        Self::Config(error)
    }
}

/// WC4 地图插件。
///
/// 注册地图资源加载器与材质，并在资源就绪后生成地图实体。
pub(crate) struct Wc4Plugin;

impl Plugin for Wc4Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            XmlAssetPlugin::<TerrainXml>::new(&["xml"]),
            XmlAssetPlugin::<ImageXml>::new(&["xml"]),
            Material2dPlugin::<InstanceMaterial>::default(),
            Material2dPlugin::<SeaMaterial>::default(),
        ))
        .init_resource::<DirtyTiles>()
        .add_systems(Startup, (load_wc4_config, load_wc4_resources, load_wc4_bin, load_wc4_btl))
        .add_systems(
            Update,
            (
                spawn_wc4_map.run_if(not(resource_exists::<Wc4MapSpawned>)),
                update_city_label_scale,
                update_tile_cursor,
                update_selection_marker.run_if(resource_changed::<SelectedTile>),
            ),
        )
        .add_systems(PostUpdate, refresh_tiles);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_includes_the_invalid_reason() {
        assert_eq!(Wc4Error::Invalid("测试原因").to_string(), "数据内容不合法: 测试原因");
    }
}

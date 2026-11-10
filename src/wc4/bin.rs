//! WC4 世界地形数据（`world.bin`）。

use std::{fs, path::Path};

use bevy::prelude::*;
use bincode::{Decode, Encode};

use crate::wc4::{
    Wc4Error,
    codec::{decode, decode_sequence},
    grid::grid_to_index,
};

/// 世界地形文件路径。
const WORLD_BIN_PATH: &str = "assets/world.bin";

/// WC4 世界地形文件头。
#[derive(Debug, Default, Clone, Copy, Encode, Decode)]
pub(crate) struct BinInfo {
    /// 尚未识别的文件头字节。
    pub(crate) unknown: [u8; 8],
    /// 世界网格列数。
    pub(crate) width: i32,
    /// 世界网格行数。
    pub(crate) height: i32,
}

/// 单个地块的地形信息。
#[derive(Debug, Default, Clone, Copy, Encode, Decode)]
pub(crate) struct Topography {
    /// 地形类型，`1` 表示水域。
    pub(crate) r#type: u8,
    /// 地形编号。
    pub(crate) id: u8,
    /// 基础层像素偏移 X。
    pub(crate) x: i8,
    /// 基础层像素偏移 Y。
    pub(crate) y: i8,
    /// 装饰 A 的地形类型。
    pub(crate) decoration_a: u8,
    /// 装饰 A 的地形编号。
    pub(crate) decoration_a_id: u8,
    /// 装饰 A 的像素偏移 X。
    pub(crate) decoration_a_x: i8,
    /// 装饰 A 的像素偏移 Y。
    pub(crate) decoration_a_y: i8,
    /// 装饰 B 的地形类型。
    pub(crate) decoration_b: u8,
    /// 装饰 B 的地形编号。
    pub(crate) decoration_b_id: u8,
    /// 装饰 B 的像素偏移 X。
    pub(crate) decoration_b_x: i8,
    /// 装饰 B 的像素偏移 Y。
    pub(crate) decoration_b_y: i8,
    /// 水域标记。
    pub(crate) water: u8,
    /// 道路标记。
    pub(crate) road: u8,
    /// 海岸标记。
    pub(crate) coast: u8,
    /// 尚未识别的字段。
    pub(crate) unknown: u8,
}

/// WC4 世界地形数据。
#[derive(Debug, Default, Resource)]
pub(crate) struct Bin {
    /// 文件头。
    pub(crate) info: BinInfo,
    /// 全部地块，按行优先排列。
    pub(crate) topographies: Vec<Topography>,
}

impl Bin {
    /// 解析世界地形二进制数据。
    pub(crate) fn parse(buffer: &[u8]) -> Result<Self, Wc4Error> {
        let mut buffer = buffer;
        let info: BinInfo = decode(&mut buffer)?;
        let topographies = decode_sequence(&mut buffer, info.land_count()?)?;

        Ok(Self {
            info,
            topographies,
        })
    }

    /// 从文件加载世界地形数据。
    pub(crate) fn load(path: impl AsRef<Path>) -> Result<Self, Wc4Error> {
        Self::parse(&fs::read(path)?)
    }

    /// 返回指定世界坐标处的地块索引。
    pub(crate) fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.info.width || y >= self.info.height {
            return None;
        }

        let width = usize::try_from(self.info.width).ok()?;

        Some(grid_to_index(usize::try_from(x).ok()?, usize::try_from(y).ok()?, width))
    }

    /// 返回指定世界坐标处的地块。
    pub(crate) fn topography(&self, x: i32, y: i32) -> Option<&Topography> {
        self.topographies.get(self.index(x, y)?)
    }
}

impl BinInfo {
    /// 计算地块总数。
    fn land_count(&self) -> Result<usize, Wc4Error> {
        let width =
            usize::try_from(self.width).map_err(|_| Wc4Error::Invalid("世界宽度必须为非负整数"))?;
        let height = usize::try_from(self.height)
            .map_err(|_| Wc4Error::Invalid("世界高度必须为非负整数"))?;

        if width == 0 || height == 0 {
            return Err(Wc4Error::Invalid("世界尺寸不能为零"));
        }

        width.checked_mul(height).ok_or(Wc4Error::Invalid("世界地块总数溢出"))
    }
}

/// 加载 WC4 世界地形数据。
pub(crate) fn load_wc4_bin(mut commands: Commands) {
    match Bin::load(WORLD_BIN_PATH) {
        Ok(bin) => {
            commands.insert_resource(bin);
        }
        Err(error) => error!("加载 WC4 世界地形失败: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use bincode::config;

    use super::*;

    fn encode_bin(info: BinInfo, topographies: &[Topography]) -> Vec<u8> {
        let config = config::standard().with_fixed_int_encoding();
        let mut bytes = bincode::encode_to_vec(info, config).unwrap();

        for topography in topographies {
            bytes.extend(bincode::encode_to_vec(topography, config).unwrap());
        }

        bytes
    }

    #[test]
    fn parse_reads_the_header_and_all_tiles() {
        let info = BinInfo {
            width: 2,
            height: 2,
            ..default()
        };
        let topographies = [
            Topography {
                r#type: 1,
                id: 7,
                ..default()
            },
            Topography::default(),
            Topography::default(),
            Topography::default(),
        ];

        let parsed = Bin::parse(&encode_bin(info, &topographies)).unwrap();

        assert_eq!(parsed.info.width, 2);
        assert_eq!(parsed.info.height, 2);
        assert_eq!(parsed.topographies.len(), 4);
        assert_eq!(parsed.topographies[0].r#type, 1);
        assert_eq!(parsed.topographies[0].id, 7);
    }

    #[test]
    fn parse_rejects_zero_sized_worlds() {
        let info = BinInfo {
            width: 0,
            height: 2,
            ..default()
        };

        assert!(matches!(Bin::parse(&encode_bin(info, &[])), Err(Wc4Error::Invalid(_))));
    }

    #[test]
    fn parse_rejects_truncated_tiles() {
        let info = BinInfo {
            width: 2,
            height: 2,
            ..default()
        };

        assert!(matches!(
            Bin::parse(&encode_bin(info, &[])),
            Err(Wc4Error::Invalid(_)) | Err(Wc4Error::Decode(_))
        ));
    }

    #[test]
    fn topography_returns_none_outside_the_world() {
        let bin = Bin {
            info: BinInfo {
                width: 2,
                height: 2,
                ..default()
            },
            topographies: vec![Topography::default(); 4],
        };

        assert!(bin.topography(0, 0).is_some());
        assert!(bin.topography(1, 1).is_some());
        assert!(bin.topography(-1, 0).is_none());
        assert!(bin.topography(0, -1).is_none());
        assert!(bin.topography(2, 0).is_none());
        assert!(bin.topography(0, 2).is_none());
    }
}

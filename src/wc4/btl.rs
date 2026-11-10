//! WC4 场景数据（`stage/*.btl`）。

use std::{fs, path::Path};

use bevy::prelude::*;

use crate::wc4::{
    Wc4Error,
    btl_data::{
        Affair, AirRaid, AirSupport, Army, ArmyPlacement, Capital, City, Country, Master, Pitfall,
        Reinforcement, Scheme, Strategy, Weather,
    },
    codec::{decode, decode_sequence},
};

/// 默认加载的场景数据路径。
const STAGE_BTL_PATH: &str = "assets/stage/conquest6.btl";

/// WC4 场景数据。
///
/// 完整保留各类场景数据，当前仅渲染城市，其余字段供后续编辑器功能使用。
#[expect(dead_code, reason = "保留完整的场景数据模型供后续编辑器功能使用")]
#[derive(Debug, Default, Resource)]
pub(crate) struct Btl {
    /// 场景主数据。
    pub(crate) master: Master,
    /// 国家列表。
    pub(crate) country_vec: Vec<Country>,
    /// 每个地块所属的省份。
    pub(crate) province_vec: Vec<i16>,
    /// 每个地块所属的国家。
    pub(crate) belong_vec: Vec<u8>,
    /// 城市列表。
    pub(crate) city_vec: Vec<City>,
    /// 军队列表。
    pub(crate) army_vec: Vec<Army>,
    /// 陷阱列表。
    pub(crate) pitfall_vec: Vec<Pitfall>,
    /// 方案列表。
    pub(crate) scheme_vec: Vec<Scheme>,
    /// 天气列表。
    pub(crate) weather_vec: Vec<Weather>,
    /// 事件列表。
    pub(crate) affair_vec: Vec<Affair>,
    /// 援军列表。
    pub(crate) reinforcement_vec: Vec<Reinforcement>,
    /// 空袭列表。
    pub(crate) air_raid_vec: Vec<AirRaid>,
    /// 兵种放置列表。
    pub(crate) army_placement_vec: Vec<ArmyPlacement>,
    /// 首都列表。
    pub(crate) capital_vec: Vec<Capital>,
    /// 策略列表。
    pub(crate) strategy_vec: Vec<Strategy>,
    /// 空中支援列表。
    pub(crate) air_support_vec: Vec<AirSupport>,
}

impl Btl {
    /// 解析场景二进制数据。
    pub(crate) fn parse(buffer: &[u8]) -> Result<Self, Wc4Error> {
        let mut buffer = buffer;
        let master: Master = decode(&mut buffer)?;

        if master.map_width <= 0 || master.map_height <= 0 {
            return Err(Wc4Error::Invalid("地图尺寸必须为正数"));
        }

        let placement_count = usize::try_from(
            i64::from(master.placement_unit_a) + i64::from(master.placement_unit_b),
        )
        .map_err(|_| Wc4Error::Invalid("放置单位总数为负"))?;

        // 各段的顺序与数量由场景主数据描述，必须与文件布局严格一致。
        let country_vec = decode_sequence(&mut buffer, count(master.total_legions)?)?;
        let province_vec = decode_sequence(&mut buffer, count(master.total_tiles)?)?;
        let belong_vec = decode_sequence(&mut buffer, count(master.total_tiles)?)?;
        let city_vec = decode_sequence(&mut buffer, count(master.total_buildings)?)?;
        let army_vec = decode_sequence(&mut buffer, count(master.total_armies)?)?;
        let pitfall_vec = decode_sequence(&mut buffer, count(master.total_traps)?)?;
        let scheme_vec = decode_sequence(&mut buffer, count(master.total_actions)?)?;
        let weather_vec = decode_sequence(&mut buffer, count(master.total_weathers)?)?;
        let affair_vec = decode_sequence(&mut buffer, count(master.total_events)?)?;
        let reinforcement_vec = decode_sequence(&mut buffer, count(master.total_reinforcements)?)?;
        let air_raid_vec = decode_sequence(&mut buffer, count(master.total_airstrikes)?)?;
        let army_placement_vec = decode_sequence(&mut buffer, placement_count)?;
        let capital_vec = decode_sequence(&mut buffer, count(master.total_capital)?)?;
        let strategy_vec = decode_sequence(&mut buffer, count(master.total_strategies)?)?;
        let air_support_vec = decode_sequence(&mut buffer, count(master.total_air_support)?)?;

        Ok(Self {
            master,
            country_vec,
            province_vec,
            belong_vec,
            city_vec,
            army_vec,
            pitfall_vec,
            scheme_vec,
            weather_vec,
            affair_vec,
            reinforcement_vec,
            air_raid_vec,
            army_placement_vec,
            capital_vec,
            strategy_vec,
            air_support_vec,
        })
    }

    /// 从文件加载场景数据。
    pub(crate) fn load(path: impl AsRef<Path>) -> Result<Self, Wc4Error> {
        Self::parse(&fs::read(path)?)
    }
}

/// 将场景数据中的计数字段转换为长度。
fn count(value: i32) -> Result<usize, Wc4Error> {
    usize::try_from(value).map_err(|_| Wc4Error::Invalid("数据计数不能为负"))
}

/// 加载 WC4 场景数据。
pub(crate) fn load_wc4_btl(mut commands: Commands) {
    match Btl::load(STAGE_BTL_PATH) {
        Ok(btl) => {
            commands.insert_resource(btl);
        }
        Err(error) => error!("加载 WC4 场景数据失败: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use bincode::config;

    use super::*;

    fn bin_config() -> impl bincode::config::Config {
        config::standard().with_fixed_int_encoding()
    }

    #[test]
    fn parse_reads_sequential_sections() {
        let master = Master {
            map_width: 4,
            map_height: 3,
            total_tiles: 2,
            ..default()
        };
        let mut bytes = bincode::encode_to_vec(master, bin_config()).unwrap();
        bytes.extend(bincode::encode_to_vec(12_i16, bin_config()).unwrap());
        bytes.extend(bincode::encode_to_vec(34_i16, bin_config()).unwrap());
        bytes.extend(bincode::encode_to_vec(1_u8, bin_config()).unwrap());
        bytes.extend(bincode::encode_to_vec(2_u8, bin_config()).unwrap());

        let btl = Btl::parse(&bytes).unwrap();

        assert_eq!(btl.master.map_width, 4);
        assert_eq!(btl.master.map_height, 3);
        assert_eq!(btl.province_vec, [12, 34]);
        assert_eq!(btl.belong_vec, [1, 2]);
    }

    #[test]
    fn parse_rejects_negative_counts() {
        let master = Master {
            map_width: 4,
            map_height: 3,
            total_legions: -1,
            ..default()
        };
        let bytes = bincode::encode_to_vec(master, bin_config()).unwrap();

        assert!(matches!(Btl::parse(&bytes), Err(Wc4Error::Invalid(_))));
    }

    #[test]
    fn parse_rejects_non_positive_map_size() {
        let master = Master {
            map_width: 0,
            map_height: 3,
            ..default()
        };
        let bytes = bincode::encode_to_vec(master, bin_config()).unwrap();

        assert!(matches!(Btl::parse(&bytes), Err(Wc4Error::Invalid(_))));
    }

    #[test]
    fn count_rejects_negative_values() {
        assert_eq!(count(0).unwrap(), 0);
        assert!(count(-1).is_err());
    }
}

//! WC4 场景数据（`.btl`）的二进制结构定义。

use bincode::{Decode, Encode};

/// 场景主数据，记录地图范围与各类数据的总数。
#[derive(Debug, Default, Encode, Decode)]
pub(crate) struct Master {
    /// Btl版本
    pub(crate) btl_version: i32,

    /// 地图序号
    pub(crate) map_index: i32,

    /// 地图截取x
    pub(crate) map_capture_x: i32,

    /// 地图截取y
    pub(crate) map_capture_y: i32,

    /// 地图宽
    pub(crate) map_width: i32,

    /// 地图高
    pub(crate) map_height: i32,

    /// 军团总数
    pub(crate) total_legions: i32,

    /// 建筑总数
    pub(crate) total_buildings: i32,

    /// 军队总数
    pub(crate) total_armies: i32,

    /// 行为总数
    pub(crate) total_actions: i32,

    /// 事件总数
    pub(crate) total_events: i32,

    /// 天气总数
    pub(crate) total_weathers: i32,

    /// 胜利条件
    pub(crate) victory_condition: i32,

    /// 最小回合
    pub(crate) min_rounds: i32,

    /// 最大回合
    pub(crate) max_rounds: i32,

    /// 援军总数
    pub(crate) total_reinforcements: i32,

    /// 空袭总数
    pub(crate) total_airstrikes: i32,

    /// 放置单位A
    pub(crate) placement_unit_a: i32,

    /// 放置单位B
    pub(crate) placement_unit_b: i32,

    /// 国家首都
    pub(crate) total_capital: i32,

    /// 战役时代
    pub(crate) campaign_era: i32,

    /// empty4
    pub(crate) empty4: i32,

    /// 地块总数
    pub(crate) total_tiles: i32,

    /// 积攒金钱
    pub(crate) accumulated_money: i32,

    /// 积攒齿轮
    pub(crate) accumulated_gears: i32,

    /// 积攒原子
    pub(crate) accumulated_atoms: i32,

    /// 陷阱总数
    pub(crate) total_traps: i32,

    /// empty5
    pub(crate) empty5: i32,

    /// 战略总数
    pub(crate) total_strategies: i32,

    /// empty6
    pub(crate) empty6: i32,

    /// empty7
    pub(crate) empty7: i32,

    /// 空中支援
    pub(crate) total_air_support: i32,
}

/// 国家数据。
#[derive(Debug, Encode, Decode)]
pub(crate) struct Country {
    /// 序号
    pub(crate) serial_number: i32,

    /// 国家
    pub(crate) country: i32,

    /// 金钱
    pub(crate) money: i32,

    /// 齿轮
    pub(crate) gears: i32,

    /// 原子
    pub(crate) atoms: i32,

    /// 控制
    pub(crate) control: i32,

    /// 阵营
    pub(crate) faction: i32,

    /// 战败条件
    pub(crate) defeat_condition: i32,

    /// 兵种加成
    pub(crate) unit_bonus: f32,

    /// 税率加成
    pub(crate) tax_bonus: f32,

    /// R8
    pub(crate) r8: u8,

    /// G8
    pub(crate) g8: u8,

    /// B8
    pub(crate) b8: u8,

    /// A8
    pub(crate) a8: u8,

    /// 原子弹数量
    pub(crate) nuclear_bombs: u32,

    /// 氢弹数量
    pub(crate) hydrogen_bombs: u32,

    /// 三相弹数量
    pub(crate) triad_bombs: i32,

    /// 反物质弹数量
    pub(crate) antimatter_bombs: i32,

    /// 机动等级
    pub(crate) mobility_level: i32,

    /// 步枪等级
    pub(crate) rifle_level: i32,

    /// 迷彩等级
    pub(crate) camouflage_level: i32,

    /// 工兵等级
    pub(crate) engineer_level: i32,

    /// 手雷等级
    pub(crate) grenade_level: i32,

    /// 迫击炮等级
    pub(crate) mortar_level: i32,

    /// 行军等级
    pub(crate) march_level: i32,

    /// 防弹衣等级
    pub(crate) armor_level: i32,

    /// 装甲等级
    pub(crate) tank_armor_level: i32,

    /// 主炮等级
    pub(crate) main_gun_level: i32,

    /// 车体等级
    pub(crate) chassis_level: i32,

    /// 引擎等级
    pub(crate) engine_level: i32,

    /// 机枪等级
    pub(crate) machine_gun_level: i32,

    /// 突袭等级
    pub(crate) raid_level: i32,

    /// 坦克防空等级
    pub(crate) tank_anti_air_level: i32,

    /// 强化车体等级
    pub(crate) enhanced_chassis_level: i32,

    /// 火炮炮击等级
    pub(crate) artillery_attack_level: i32,

    /// 火箭弹等级
    pub(crate) rocket_level: i32,

    /// 火炮牵引等级
    pub(crate) artillery_tow_level: i32,

    /// 火炮装甲等级
    pub(crate) artillery_armor_level: i32,

    /// 火炮火力等级
    pub(crate) artillery_firepower_level: i32,

    /// 火炮火箭等级
    pub(crate) artillery_rocket_level: i32,

    /// 伪装等级
    pub(crate) camouflage2_level: i32,

    /// 舰艇船体等级
    pub(crate) ship_hull_level: i32,

    /// 推进器等级
    pub(crate) propulsion_level: i32,

    /// 舰艇装甲等级
    pub(crate) ship_armor_level: i32,

    /// 武器等级
    pub(crate) weapon_level: i32,

    /// 舰艇舰炮等级
    pub(crate) ship_main_gun_level: i32,

    /// 鱼雷等级
    pub(crate) torpedo_level: i32,

    /// 舰艇扫雷
    pub(crate) ship_mine_sweeping_level: i32,

    /// 防空武器等级
    pub(crate) anti_air_weapon_level: i32,

    /// 现代舰艇等级
    pub(crate) modern_ship_level: i32,

    /// 航空燃油等级
    pub(crate) aviation_fuel_level: i32,

    /// 航空发动机等级
    pub(crate) aviation_engine_level: i32,

    /// 航空炸弹等级
    pub(crate) aviation_bomb_level: i32,

    /// 空袭等级
    pub(crate) airstrike_level: i32,

    /// 轰炸等级
    pub(crate) bombing_level: i32,

    /// 战略轰炸等级
    pub(crate) strategic_bombing_level: i32,

    /// 空降兵等级
    pub(crate) airborne_level: i32,

    /// 喷气发动机等级
    pub(crate) jet_engine_level: i32,

    /// 机枪堡等级
    pub(crate) machine_gun_bunker_level: i32,

    /// 要塞炮等级
    pub(crate) fortress_gun_level: i32,

    /// 海岸炮等级
    pub(crate) coastal_gun_level: i32,

    /// 火箭发射器等级
    pub(crate) rocket_launcher_level: i32,

    /// 工事等级
    pub(crate) fortification_level: i32,

    /// 高射机枪等级
    pub(crate) anti_aircraft_gun_level: i32,

    /// 防空炮等级
    pub(crate) anti_aircraft_cannon_level: i32,

    /// 对空导弹等级
    pub(crate) anti_air_missile_level: i32,

    /// 雷达等级
    pub(crate) radar_level: i32,

    /// 弹头
    pub(crate) warhead: i32,

    /// 固体火箭发动机等级
    pub(crate) solid_rocket_engine_level: i32,

    /// 破防等级
    pub(crate) armor_piercing_level: i32,

    /// 核聚变等级
    pub(crate) nuclear_fusion_level: i32,

    /// empty1
    pub(crate) empty1: i32,

    /// empty2
    pub(crate) empty2: i32,

    /// empty3
    pub(crate) empty3: i32,

    /// empty4
    pub(crate) empty4: i32,

    /// 科技等级
    pub(crate) tech_level: i32,

    /// empty5
    pub(crate) empty5: i32,

    /// empty6
    pub(crate) empty6: i32,
}

/// 城市数据。
#[derive(Debug, Default, Encode, Decode)]
pub(crate) struct City {
    /// 坐标
    pub(crate) coordinate: i16,

    /// 名称
    pub(crate) name: i16,

    /// 等级
    pub(crate) level: u8,

    /// 外观
    pub(crate) appearance: u8,

    /// 地标
    pub(crate) landmark: u8,

    /// 奇观
    pub(crate) wonder: u8,

    /// 奖励类型
    pub(crate) reward_type: i16,

    /// 奖励数量
    pub(crate) reward_quantity: i16,

    /// 仇恨值
    pub(crate) hatred: u8,

    /// 据点
    pub(crate) stronghold: u8,

    /// 触发事件
    pub(crate) trigger_event: u8,

    /// empty1
    pub(crate) empty1: u8,

    /// empty2
    pub(crate) empty2: u8,

    /// empty3
    pub(crate) empty3: u8,

    /// empty4
    pub(crate) empty4: u8,

    /// 运输船
    pub(crate) transport_ship: u8,

    /// 火焰类型
    pub(crate) flame_type: u8,

    /// 持续回合
    pub(crate) duration_rounds: u8,

    /// 防空武器
    pub(crate) anti_air_weapon: u8,

    /// 防空武器范围
    pub(crate) anti_air_weapon_range: u8,

    /// 工厂
    pub(crate) factory: u8,

    /// 科技
    pub(crate) technology: u8,

    /// 补给站
    pub(crate) supply_station: u8,

    /// 航空
    pub(crate) aviation: u8,

    /// 导弹
    pub(crate) missile: u8,

    /// 核弹
    pub(crate) nuclear_bomb: u8,

    /// empty5
    pub(crate) empty5: u8,

    /// empty6
    pub(crate) empty6: u8,
}

/// 军队数据。
#[derive(Debug, Default, Encode, Decode)]
pub(crate) struct Army {
    /// 坐标
    pub(crate) coordinate: i16,

    /// 兵种
    pub(crate) arms: u8,

    /// 等级
    pub(crate) level: u8,

    /// 编制
    pub(crate) organization: u8,

    /// 方向
    pub(crate) direction: u8,

    /// 移动力
    pub(crate) mobility: u8,

    /// 建造回合
    pub(crate) construction_rounds: u8,

    /// 兵种经验
    pub(crate) arms_experience: i16,

    /// 血量加成
    pub(crate) health_bonus: i16,

    /// 当前血量
    pub(crate) current_health: i16,

    /// 血量上限
    pub(crate) max_health: i16,

    /// 将领
    pub(crate) commander: i16,

    /// 军衔
    pub(crate) rank: u8,

    /// 爵位
    pub(crate) title: u8,

    /// 胸章一
    pub(crate) badge_one: u8,

    /// 胸章二
    pub(crate) badge_two: u8,

    /// 胸章三
    pub(crate) badge_three: u8,

    /// 技能等级1
    pub(crate) skill_level_1: u8,

    /// 技能等级2
    pub(crate) skill_level_2: u8,

    /// 技能等级3
    pub(crate) skill_level_3: u8,

    /// 技能等级4
    pub(crate) skill_level_4: u8,

    /// 技能等级5
    pub(crate) skill_level_5: u8,

    /// 据点
    pub(crate) stronghold: u8,

    /// 方针
    pub(crate) policy: u8,

    /// 运输船
    pub(crate) transport_ship: u8,

    /// 仇恨值
    pub(crate) hatred: u8,

    /// 移动目标
    pub(crate) move_target: i16,

    /// empty1
    pub(crate) empty1: u8,

    /// empty2
    pub(crate) empty2: u8,

    /// 行为方案
    pub(crate) behavior_plan: i16,

    /// 改变回合
    pub(crate) change_rounds: i16,

    /// 士气
    pub(crate) morale: u8,

    /// 士气持续回合
    pub(crate) morale_duration_rounds: u8,

    /// 触发事件
    pub(crate) trigger_event: u8,

    /// 盾牌标志
    pub(crate) shield_flag: u8,

    /// 固守最短距离
    pub(crate) minimum_defense_distance: i32,

    /// empty3
    pub(crate) empty3: u8,

    /// empty4
    pub(crate) empty4: u8,

    /// empty5
    pub(crate) empty5: u8,

    /// empty6
    pub(crate) empty6: u8,

    /// empty7
    pub(crate) empty7: u8,

    /// empty8
    pub(crate) empty8: u8,

    /// ribbon1
    pub(crate) ribbon1: u16,

    /// ribbon2
    pub(crate) ribbon2: u16,

    /// ribbon3
    pub(crate) ribbon3: u16,
}

/// 陷阱数据。
#[derive(Debug, Default, Encode, Decode)]
pub(crate) struct Pitfall {
    /// 坐标
    pub(crate) coordinate: i16,

    /// 所属军团
    pub(crate) affiliated_legion: i16,

    /// 陷阱编制
    pub(crate) trap_organization: i16,

    /// 陷阱血量
    pub(crate) trap_health: i32,

    /// empty
    pub(crate) empty: i16,
}

/// 方案数据。
#[derive(Debug, Encode, Decode)]
pub(crate) struct Scheme {
    /// 方案编号
    pub(crate) scheme_id: i32,

    /// 终结事件
    pub(crate) end_event: i32,

    /// 触发回合
    pub(crate) trigger_round: i32,

    /// 目标地块
    pub(crate) target_tile: i32,
}

/// 天气数据。
#[derive(Debug, Encode, Decode)]
pub(crate) struct Weather {
    /// 天气类型
    pub(crate) weather_type: i32,

    /// empty
    pub(crate) empty: i32,

    /// 触发回合
    pub(crate) trigger_round: i32,

    /// 持续回合
    pub(crate) duration_rounds: i32,
}

/// 事件数据。
#[derive(Debug, Encode, Decode)]
pub(crate) struct Affair {
    /// 事件ID
    pub(crate) event_id: i32,

    /// 关联事件
    pub(crate) related_event: i32,

    /// 触发条件
    pub(crate) trigger_conditions: i32,

    /// 事件类型
    pub(crate) event_type: i32,

    /// 触发军团
    pub(crate) triggering_legion: i32,

    /// 影响军团
    pub(crate) affected_legion: i32,

    /// 目标值
    pub(crate) target_value: i32,

    /// Zero
    pub(crate) zero: i32,

    /// 触发回合
    pub(crate) trigger_round: i32,

    /// 对话代码
    pub(crate) dialog_code: i32,

    /// 默认结束段
    pub(crate) default_end_segment: i32,
}

/// 援军数据。
#[derive(Debug, Default, Encode, Decode)]
pub(crate) struct Reinforcement {
    /// 坐标
    pub(crate) coordinate: i32,

    /// 兵种
    pub(crate) unit_type: i32,

    /// 等级
    pub(crate) level: i32,

    /// 编制
    pub(crate) organization: i32,

    /// 运输船
    pub(crate) transport_ship: i32,

    /// 方针
    pub(crate) policy: i32,

    /// 未知
    pub(crate) unknown1: i32,

    /// 将领
    pub(crate) general: i32,

    /// 军衔
    pub(crate) rank: i32,

    /// 爵位
    pub(crate) title: i32,

    /// 技能等级1
    pub(crate) skill_level1: i32,

    /// 技能等级2
    pub(crate) skill_level2: i32,

    /// 技能等级3
    pub(crate) skill_level3: i32,

    /// 技能等级4
    pub(crate) skill_level4: i32,

    /// 技能等级5
    pub(crate) skill_level5: i32,

    /// 勋章1
    pub(crate) medal1: i32,

    /// 勋章2
    pub(crate) medal2: i32,

    /// 勋章3
    pub(crate) medal3: i32,

    /// 所属国家
    pub(crate) affiliated_country: i32,

    /// 爆兵回合
    pub(crate) explosion_round: i32,

    /// 胸章1
    pub(crate) badge1: i32,

    /// 胸章2
    pub(crate) badge2: i32,

    /// 胸章3
    pub(crate) badge3: i32,

    /// 勋带1
    pub(crate) ribbon1: i32,

    /// 勋带2
    pub(crate) ribbon2: i32,

    /// 勋带3
    pub(crate) ribbon3: i32,
}

/// 空袭数据。
#[derive(Debug, Default, Encode, Decode)]
pub(crate) struct AirRaid {
    /// 坐标
    pub(crate) coordinate: i32,

    /// 兵种
    pub(crate) unit_type: i16,

    /// 精英兵种
    pub(crate) elite_unit_type: i16,

    /// 弹药
    pub(crate) ammunition: i32,

    /// 军团
    pub(crate) legion: i32,

    /// 回合
    pub(crate) round: i32,
}

/// 兵种放置数据。
#[derive(Debug, Default, Encode, Decode)]
pub(crate) struct ArmyPlacement {
    /// 坐标
    pub(crate) coordinate: i16,

    /// empty
    pub(crate) empty: i16,

    /// 方向
    pub(crate) direction: u8,

    /// Id
    pub(crate) id: u8,

    /// 运输船
    pub(crate) transport_ship: u8,

    /// empty2
    pub(crate) empty2: u8,
}

/// 首都数据。
#[derive(Debug, Default, Encode, Decode)]
pub(crate) struct Capital {
    /// 坐标
    pub(crate) coordinate: i16,

    /// empty
    pub(crate) empty: i16,
}

/// 策略数据。
#[derive(Debug, Encode, Decode)]
pub(crate) struct Strategy {
    /// 军团序号
    pub(crate) legion_number: i32,

    /// empty
    pub(crate) empty: i32,

    /// 回合
    pub(crate) round: i32,

    /// 建设代码
    pub(crate) construction_code: i32,
}

/// 空中支援数据。
#[derive(Debug, Encode, Decode)]
pub(crate) struct AirSupport {
    /// 兵种
    pub(crate) unit_type: i32,

    /// 弹药
    pub(crate) ammunition: i32,

    /// 军团
    pub(crate) legion: i32,

    /// 回合
    pub(crate) round: i32,
}

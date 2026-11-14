//! 直接依附于地块的对象实例及其可编辑字段。

use bevy::prelude::*;

use crate::wc4::{bin::Bin, btl::Btl, grid::grid_to_index, tile::map_position};

/// 直接依附于地块的对象类别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttachmentKind {
    /// 城市。
    City,
    /// 军队。
    Army,
    /// 陷阱。
    Pitfall,
    /// 援军。
    Reinforcement,
    /// 空袭。
    AirRaid,
    /// 兵种放置。
    ArmyPlacement,
    /// 首都。
    Capital,
}

impl AttachmentKind {
    /// 全部类别，顺序即检查器中的分组顺序。
    pub(crate) const ALL: [Self; 7] = [
        Self::City,
        Self::Army,
        Self::Pitfall,
        Self::Reinforcement,
        Self::AirRaid,
        Self::ArmyPlacement,
        Self::Capital,
    ];

    /// 返回面向用户的类别名称。
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::City => "City",
            Self::Army => "Army",
            Self::Pitfall => "Pitfall",
            Self::Reinforcement => "Reinforcement",
            Self::AirRaid => "Air Raid",
            Self::ArmyPlacement => "Army Placement",
            Self::Capital => "Capital",
        }
    }
}

/// 依附实例在原始数据列表中的位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Attachment {
    /// 对象类别。
    pub(crate) kind: AttachmentKind,
    /// 在该类别列表中的下标。
    pub(crate) index: usize,
}

impl Attachment {
    /// 返回实例在类别内的序号（从 1 开始）。
    pub(crate) const fn ordinal(self) -> usize {
        self.index + 1
    }
}

/// 列出直接依附于指定网格坐标的全部对象实例。
///
/// 结果按 [`AttachmentKind::ALL`] 的顺序排列，类别内保持原始数据顺序。
pub(crate) fn attachments_at(btl: &Btl, bin: &Bin, grid: IVec2) -> Vec<Attachment> {
    let Some(index) = tile_index(btl, bin, grid) else {
        return Vec::new();
    };

    let mut attachments = Vec::new();

    for kind in AttachmentKind::ALL {
        collect_kind(&mut attachments, kind, btl, index);
    }

    attachments
}

/// 收集指定类别中坐标匹配的实例。
fn collect_kind(attachments: &mut Vec<Attachment>, kind: AttachmentKind, btl: &Btl, index: i32) {
    match kind {
        AttachmentKind::City => {
            collect(attachments, kind, &btl.city_vec, |city| i32::from(city.coordinate), index)
        }
        AttachmentKind::Army => {
            collect(attachments, kind, &btl.army_vec, |army| i32::from(army.coordinate), index)
        }
        AttachmentKind::Pitfall => collect(
            attachments,
            kind,
            &btl.pitfall_vec,
            |pitfall| i32::from(pitfall.coordinate),
            index,
        ),
        AttachmentKind::Reinforcement => collect(
            attachments,
            kind,
            &btl.reinforcement_vec,
            |reinforcement| reinforcement.coordinate,
            index,
        ),
        AttachmentKind::AirRaid => {
            collect(attachments, kind, &btl.air_raid_vec, |air_raid| air_raid.coordinate, index)
        }
        AttachmentKind::ArmyPlacement => collect(
            attachments,
            kind,
            &btl.army_placement_vec,
            |placement| i32::from(placement.coordinate),
            index,
        ),
        AttachmentKind::Capital => collect(
            attachments,
            kind,
            &btl.capital_vec,
            |capital| i32::from(capital.coordinate),
            index,
        ),
    }
}

/// 返回类别的可编辑字段名称。
pub(crate) fn field_names(kind: AttachmentKind) -> &'static [&'static str] {
    match kind {
        AttachmentKind::City => city::NAMES,
        AttachmentKind::Army => army::NAMES,
        AttachmentKind::Pitfall => pitfall::NAMES,
        AttachmentKind::Reinforcement => reinforcement::NAMES,
        AttachmentKind::AirRaid => air_raid::NAMES,
        AttachmentKind::ArmyPlacement => army_placement::NAMES,
        AttachmentKind::Capital => &[],
    }
}

/// 读取依附实例的字段值。
///
/// 实例或字段不存在时返回 `None`。
pub(crate) fn field_value(btl: &Btl, attachment: Attachment, field: usize) -> Option<i32> {
    match attachment.kind {
        AttachmentKind::City => city::value(btl, attachment.index, field),
        AttachmentKind::Army => army::value(btl, attachment.index, field),
        AttachmentKind::Pitfall => pitfall::value(btl, attachment.index, field),
        AttachmentKind::Reinforcement => reinforcement::value(btl, attachment.index, field),
        AttachmentKind::AirRaid => air_raid::value(btl, attachment.index, field),
        AttachmentKind::ArmyPlacement => army_placement::value(btl, attachment.index, field),
        AttachmentKind::Capital => None,
    }
}

/// 写入依附实例的字段值。
///
/// 实例或字段不存在时返回 `false`；数值超出字段类型范围时截断到边界。
pub(crate) fn set_field_value(
    btl: &mut Btl,
    attachment: Attachment,
    field: usize,
    value: i32,
) -> bool {
    match attachment.kind {
        AttachmentKind::City => city::apply(btl, attachment.index, field, value),
        AttachmentKind::Army => army::apply(btl, attachment.index, field, value),
        AttachmentKind::Pitfall => pitfall::apply(btl, attachment.index, field, value),
        AttachmentKind::Reinforcement => reinforcement::apply(btl, attachment.index, field, value),
        AttachmentKind::AirRaid => air_raid::apply(btl, attachment.index, field, value),
        AttachmentKind::ArmyPlacement => army_placement::apply(btl, attachment.index, field, value),
        AttachmentKind::Capital => false,
    }
}

/// 将网格坐标换算为行优先索引。
///
/// 网格坐标超出地图范围或该位置没有地形数据时返回 `None`。
fn tile_index(btl: &Btl, bin: &Bin, grid: IVec2) -> Option<i32> {
    let map = map_position(&btl.master, grid)?;
    bin.topography(map.x, map.y)?;

    let width = usize::try_from(bin.info.width).ok()?;

    i32::try_from(grid_to_index(usize::try_from(map.x).ok()?, usize::try_from(map.y).ok()?, width))
        .ok()
}

/// 收集坐标匹配指定索引的实例。
fn collect<T>(
    attachments: &mut Vec<Attachment>,
    kind: AttachmentKind,
    items: &[T],
    coordinate: impl Fn(&T) -> i32,
    index: i32,
) {
    for (position, item) in items.iter().enumerate() {
        if coordinate(item) == index {
            attachments.push(Attachment {
                kind,
                index: position,
            });
        }
    }
}

/// 为数值字段生成名称列表与读写访问器。
///
/// 字段下标必须与名称列表顺序一致，`tests` 模块会逐个校验。
macro_rules! numeric_fields {
    (
        $list:ident,
        $( $index:literal => $name:literal : $field:ident as $clamp:ident ),+ $(,)?
    ) => {
        /// 字段名称列表。
        pub(super) const NAMES: &[&str] = &[ $( $name ),+ ];

        /// 读取字段值。
        pub(super) fn value(btl: &Btl, index: usize, field: usize) -> Option<i32> {
            let item = btl.$list.get(index)?;

            Some(match field {
                $( $index => i32::from(item.$field), )+
                _ => return None,
            })
        }

        /// 写入字段值。
        pub(super) fn apply(btl: &mut Btl, index: usize, field: usize, value: i32) -> bool {
            let Some(item) = btl.$list.get_mut(index) else {
                return false;
            };

            match field {
                $( $index => item.$field = $clamp(value), )+
                _ => return false,
            }

            true
        }
    };
}

mod city {
    use super::*;
    use crate::wc4::numeric::{clamp_i16, clamp_u8};

    numeric_fields! {
        city_vec,
        0 => "Name": name as clamp_i16,
        1 => "Level": level as clamp_u8,
        2 => "Appearance": appearance as clamp_u8,
        3 => "Landmark": landmark as clamp_u8,
        4 => "Wonder": wonder as clamp_u8,
        5 => "Reward Type": reward_type as clamp_i16,
        6 => "Reward Quantity": reward_quantity as clamp_i16,
        7 => "Hatred": hatred as clamp_u8,
        8 => "Stronghold": stronghold as clamp_u8,
        9 => "Trigger Event": trigger_event as clamp_u8,
        10 => "Transport Ship": transport_ship as clamp_u8,
        11 => "Flame Type": flame_type as clamp_u8,
        12 => "Duration Rounds": duration_rounds as clamp_u8,
        13 => "Anti Air Weapon": anti_air_weapon as clamp_u8,
        14 => "Anti Air Weapon Range": anti_air_weapon_range as clamp_u8,
        15 => "Factory": factory as clamp_u8,
        16 => "Technology": technology as clamp_u8,
        17 => "Supply Station": supply_station as clamp_u8,
        18 => "Aviation": aviation as clamp_u8,
        19 => "Missile": missile as clamp_u8,
        20 => "Nuclear Bomb": nuclear_bomb as clamp_u8,
    }
}

mod army {
    use super::*;
    use crate::wc4::numeric::{clamp_i16, clamp_i32, clamp_u8, clamp_u16};

    numeric_fields! {
        army_vec,
        0 => "Arms": arms as clamp_u8,
        1 => "Level": level as clamp_u8,
        2 => "Organization": organization as clamp_u8,
        3 => "Direction": direction as clamp_u8,
        4 => "Mobility": mobility as clamp_u8,
        5 => "Construction Rounds": construction_rounds as clamp_u8,
        6 => "Arms Experience": arms_experience as clamp_i16,
        7 => "Health Bonus": health_bonus as clamp_i16,
        8 => "Current Health": current_health as clamp_i16,
        9 => "Max Health": max_health as clamp_i16,
        10 => "Commander": commander as clamp_i16,
        11 => "Rank": rank as clamp_u8,
        12 => "Title": title as clamp_u8,
        13 => "Badge One": badge_one as clamp_u8,
        14 => "Badge Two": badge_two as clamp_u8,
        15 => "Badge Three": badge_three as clamp_u8,
        16 => "Skill Level 1": skill_level_1 as clamp_u8,
        17 => "Skill Level 2": skill_level_2 as clamp_u8,
        18 => "Skill Level 3": skill_level_3 as clamp_u8,
        19 => "Skill Level 4": skill_level_4 as clamp_u8,
        20 => "Skill Level 5": skill_level_5 as clamp_u8,
        21 => "Stronghold": stronghold as clamp_u8,
        22 => "Policy": policy as clamp_u8,
        23 => "Transport Ship": transport_ship as clamp_u8,
        24 => "Hatred": hatred as clamp_u8,
        25 => "Move Target": move_target as clamp_i16,
        26 => "Behavior Plan": behavior_plan as clamp_i16,
        27 => "Change Rounds": change_rounds as clamp_i16,
        28 => "Morale": morale as clamp_u8,
        29 => "Morale Duration Rounds": morale_duration_rounds as clamp_u8,
        30 => "Trigger Event": trigger_event as clamp_u8,
        31 => "Shield Flag": shield_flag as clamp_u8,
        32 => "Minimum Defense Distance": minimum_defense_distance as clamp_i32,
        33 => "Ribbon 1": ribbon1 as clamp_u16,
        34 => "Ribbon 2": ribbon2 as clamp_u16,
        35 => "Ribbon 3": ribbon3 as clamp_u16,
    }
}

mod pitfall {
    use super::*;
    use crate::wc4::numeric::{clamp_i16, clamp_i32};

    numeric_fields! {
        pitfall_vec,
        0 => "Affiliated Legion": affiliated_legion as clamp_i16,
        1 => "Trap Organization": trap_organization as clamp_i16,
        2 => "Trap Health": trap_health as clamp_i32,
    }
}

mod reinforcement {
    use super::*;
    use crate::wc4::numeric::clamp_i32;

    numeric_fields! {
        reinforcement_vec,
        0 => "Unit Type": unit_type as clamp_i32,
        1 => "Level": level as clamp_i32,
        2 => "Organization": organization as clamp_i32,
        3 => "Transport Ship": transport_ship as clamp_i32,
        4 => "Policy": policy as clamp_i32,
        5 => "General": general as clamp_i32,
        6 => "Rank": rank as clamp_i32,
        7 => "Title": title as clamp_i32,
        8 => "Skill Level 1": skill_level1 as clamp_i32,
        9 => "Skill Level 2": skill_level2 as clamp_i32,
        10 => "Skill Level 3": skill_level3 as clamp_i32,
        11 => "Skill Level 4": skill_level4 as clamp_i32,
        12 => "Skill Level 5": skill_level5 as clamp_i32,
        13 => "Medal 1": medal1 as clamp_i32,
        14 => "Medal 2": medal2 as clamp_i32,
        15 => "Medal 3": medal3 as clamp_i32,
        16 => "Affiliated Country": affiliated_country as clamp_i32,
        17 => "Explosion Round": explosion_round as clamp_i32,
        18 => "Badge 1": badge1 as clamp_i32,
        19 => "Badge 2": badge2 as clamp_i32,
        20 => "Badge 3": badge3 as clamp_i32,
        21 => "Ribbon 1": ribbon1 as clamp_i32,
        22 => "Ribbon 2": ribbon2 as clamp_i32,
        23 => "Ribbon 3": ribbon3 as clamp_i32,
    }
}

mod air_raid {
    use super::*;
    use crate::wc4::numeric::{clamp_i16, clamp_i32};

    numeric_fields! {
        air_raid_vec,
        0 => "Unit Type": unit_type as clamp_i16,
        1 => "Elite Unit Type": elite_unit_type as clamp_i16,
        2 => "Ammunition": ammunition as clamp_i32,
        3 => "Legion": legion as clamp_i32,
        4 => "Round": round as clamp_i32,
    }
}

mod army_placement {
    use super::*;
    use crate::wc4::numeric::clamp_u8;

    numeric_fields! {
        army_placement_vec,
        0 => "Direction": direction as clamp_u8,
        1 => "Id": id as clamp_u8,
        2 => "Transport Ship": transport_ship as clamp_u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wc4::{
        bin::{BinInfo, Topography},
        btl_data::{AirRaid, Army, ArmyPlacement, Capital, City, Master, Pitfall, Reinforcement},
    };

    const WIDTH: i32 = 4;

    fn bin(width: i32, height: i32) -> Bin {
        let count = usize::try_from(width * height).unwrap();

        Bin {
            info: BinInfo {
                width,
                height,
                ..default()
            },
            topographies: vec![Topography::default(); count],
        }
    }

    fn btl() -> Btl {
        Btl {
            master: Master {
                map_width: WIDTH,
                map_height: 4,
                ..default()
            },
            city_vec: vec![City {
                coordinate: 5,
                ..default()
            }],
            army_vec: vec![Army {
                coordinate: 5,
                ..default()
            }],
            pitfall_vec: vec![Pitfall {
                coordinate: 5,
                ..default()
            }],
            reinforcement_vec: vec![Reinforcement {
                coordinate: 5,
                ..default()
            }],
            air_raid_vec: vec![AirRaid {
                coordinate: 5,
                ..default()
            }],
            army_placement_vec: vec![ArmyPlacement {
                coordinate: 5,
                ..default()
            }],
            capital_vec: vec![Capital {
                coordinate: 5,
                ..default()
            }],
            ..default()
        }
    }

    #[test]
    fn attachments_at_lists_every_kind_on_the_tile() {
        let btl = btl();
        let bin = bin(WIDTH, 4);

        let attachments = attachments_at(&btl, &bin, IVec2::new(1, 1));
        let kinds = attachments.iter().map(|attachment| attachment.kind).collect::<Vec<_>>();

        assert_eq!(kinds, AttachmentKind::ALL);
        assert!(attachments.iter().all(|attachment| attachment.index == 0));
    }

    #[test]
    fn attachments_at_ignores_other_tiles_and_missing_topography() {
        let btl = btl();
        let bin = bin(WIDTH, 4);

        assert!(attachments_at(&btl, &bin, IVec2::new(2, 1)).is_empty());
        assert!(attachments_at(&btl, &bin, IVec2::new(-1, 0)).is_empty());
        assert!(attachments_at(&btl, &bin, IVec2::new(WIDTH, 0)).is_empty());
    }

    #[test]
    fn every_named_field_round_trips() {
        for kind in AttachmentKind::ALL {
            let mut btl = btl();
            let attachment = Attachment {
                kind,
                index: 0,
            };

            for field in 0..field_names(kind).len() {
                assert!(
                    field_value(&btl, attachment, field).is_some(),
                    "{kind:?} field {field} is not readable"
                );
                assert!(
                    set_field_value(&mut btl, attachment, field, 7),
                    "{kind:?} field {field} is not writable"
                );
                assert_eq!(field_value(&btl, attachment, field), Some(7));
            }
        }
    }

    #[test]
    fn set_field_value_clamps_to_the_field_range() {
        let mut btl = btl();
        let attachment = Attachment {
            kind: AttachmentKind::City,
            index: 0,
        };

        // City 的 "Level" 字段类型为 `u8`，超界值应被截断到类型上限。
        assert!(set_field_value(&mut btl, attachment, 1, 999));
        assert_eq!(field_value(&btl, attachment, 1), Some(i32::from(u8::MAX)));
    }

    #[test]
    fn unknown_fields_and_instances_are_rejected() {
        let mut btl = btl();
        let attachment = Attachment {
            kind: AttachmentKind::City,
            index: 0,
        };
        let missing = Attachment {
            kind: AttachmentKind::City,
            index: 9,
        };

        assert_eq!(field_value(&btl, attachment, 99), None);
        assert!(!set_field_value(&mut btl, attachment, 99, 1));
        assert_eq!(field_value(&btl, missing, 0), None);
        assert!(!set_field_value(&mut btl, missing, 0, 1));
    }

    #[test]
    fn capital_has_no_editable_fields() {
        assert!(field_names(AttachmentKind::Capital).is_empty());
        assert_eq!(
            field_value(
                &btl(),
                Attachment {
                    kind: AttachmentKind::Capital,
                    index: 0,
                },
                0,
            ),
            None
        );
    }
}

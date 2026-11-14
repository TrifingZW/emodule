//! 检查器面板场景。

use bevy::{
    feathers::{
        containers::{group, group_body, group_header, pane, pane_body, pane_header},
        controls::{FeathersNumberInput, NumberInputValue},
        display::{caption, label_small},
        theme::{ThemeBackgroundColor, ThemeBorderColor},
        tokens,
    },
    prelude::*,
    text::EditableText,
    ui::{Overflow, OverflowAxis},
    ui_widgets::{ScrollArea, ValueChange},
};

use bevy::input_focus::InputFocus;

use crate::state::SelectedTile;
use crate::wc4::{
    Attachment, Bin, Btl, DirtyTiles, TileSummary, attachments_at, field_names, field_value,
    set_field_value, set_tile_field, tile_field_names, tile_field_value, tile_summary,
};

use super::{BORDER_WIDTH, INSPECTOR_WIDTH, SPACE_LG, SPACE_MD, SPACE_SM};

/// 字段输入框宽度（逻辑像素）。
const FIELD_INPUT_WIDTH: f32 = 84.0;
/// 字段行最小高度（逻辑像素），与 Feathers 控件行高一致。
const FIELD_ROW_HEIGHT: f32 = 24.0;

/// 标记地块信息列表容器。
#[derive(Component, Clone, Default)]
pub(crate) struct TileDetails;

/// 标记依附对象分组容器。
#[derive(Component, Clone, Default)]
pub(crate) struct AttachmentGroups;

/// 绑定到地块字段的输入框。
#[derive(Component, Clone, Copy)]
pub(crate) struct TileField {
    /// 字段所属地块的网格坐标。
    grid: IVec2,
    /// 字段下标。
    field: usize,
}

/// 绑定到依附对象字段的输入框。
#[derive(Component, Clone, Copy)]
pub(crate) struct AttachmentField {
    /// 字段所属地块的网格坐标。
    grid: IVec2,
    /// 字段所属的依附实例。
    attachment: Attachment,
    /// 字段下标。
    field: usize,
}

/// 构建检查器面板。
pub(super) fn inspector() -> impl Scene {
    bsn! {
        Node {
            width: px(INSPECTOR_WIDTH),
            height: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            min_height: px(0),
            padding: px(SPACE_LG),
            border: UiRect::left(px(BORDER_WIDTH)),
            overflow: Overflow::clip(),
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        ThemeBorderColor(tokens::PANE_HEADER_BORDER)
        Children [
            @pane()
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_height: px(0),
                overflow: Overflow::clip(),
            }
            Children [
                @pane_header()
                Node {
                    border: UiRect::all(px(BORDER_WIDTH)),
                    border_radius: BorderRadius::ZERO,
                }
                Children [
                    @caption("Inspector")
                ]
                --
                @pane_body()
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_height: px(0),
                    border_radius: BorderRadius::ZERO,
                    padding: px(SPACE_MD),
                    row_gap: px(SPACE_MD),
                    overflow: Overflow {
                        x: OverflowAxis::Clip,
                        y: OverflowAxis::Scroll,
                    },
                }
                ScrollArea
                Children [
                    Node {
                        width: percent(100),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 0.0,
                        row_gap: px(SPACE_MD),
                    }
                    Children [
                        @tile_group()
                        --
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.0,
                            row_gap: px(SPACE_SM),
                        }
                        AttachmentGroups
                    ]
                ]
            ]
        ]
    }
}

/// 构建地块信息控件组。
fn tile_group() -> impl Scene {
    bsn! {
        @group()
        Children [
            @group_header()
            Children [
                @caption("Tile")
            ]
            --
            @group_body()
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: px(SPACE_SM),
            }
            TileDetails
        ]
    }
}

/// 鼠标按键按下期间不重建面板，避免拖拽过程中销毁正在交互的控件。
pub(crate) fn pointer_released(buttons: Res<ButtonInput<MouseButton>>) -> bool {
    buttons.get_pressed().next().is_none()
}

/// 根据选中地块重建地块信息与依附对象分组。
pub(crate) fn update_tile_details(
    selected: Res<SelectedTile>,
    btl: Option<Res<Btl>>,
    bin: Option<Res<Bin>>,
    details: Query<Entity, With<TileDetails>>,
    groups: Query<Entity, With<AttachmentGroups>>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let (Ok(details), Ok(groups)) = (details.single(), groups.single()) else {
        return;
    };

    // 面板即将重建，先释放输入焦点，避免焦点系统在后续阶段引用将被销毁的控件。
    if focus.get().is_some() {
        focus.clear();
    }

    let grid = selected.get();
    let (summary, attachments) = match (btl.as_deref(), bin.as_deref(), grid) {
        (Some(btl), Some(bin), Some(grid)) => {
            (tile_summary(btl, bin, grid), attachments_at(btl, bin, grid))
        }
        _ => (None, Vec::new()),
    };

    rebuild_details(&mut commands, details, summary.as_ref(), btl.as_deref(), bin.as_deref());
    rebuild_groups(&mut commands, groups, grid, btl.as_deref(), &attachments);
}

/// 将地块字段的编辑结果写回地图数据并同步输入框。
fn on_tile_field(
    change: On<ValueChange<i32>>,
    fields: Query<&TileField>,
    btl: Option<ResMut<Btl>>,
    bin: Option<ResMut<Bin>>,
    mut dirty: ResMut<DirtyTiles>,
    mut commands: Commands,
) {
    let Ok(field) = fields.get(change.event_target()) else {
        return;
    };
    let (Some(mut btl), Some(mut bin)) = (btl, bin) else {
        return;
    };

    let stored = apply_tile_field(&mut btl, &mut bin, &mut dirty, field, change.value);
    sync_input(&mut commands, change.event_target(), stored);
}

/// 将依附字段的编辑结果写回场景数据并同步输入框。
fn on_attachment_field(
    change: On<ValueChange<i32>>,
    fields: Query<&AttachmentField>,
    btl: Option<ResMut<Btl>>,
    mut dirty: ResMut<DirtyTiles>,
    mut commands: Commands,
) {
    let Ok(field) = fields.get(change.event_target()) else {
        return;
    };
    let Some(mut btl) = btl else {
        return;
    };

    let stored = apply_attachment_field(&mut btl, &mut dirty, field, change.value);
    sync_input(&mut commands, change.event_target(), stored);
}

/// 文本编辑过程中即时写回字段，无需等待回车或失焦。
pub(crate) fn write_fields_from_text(
    changed: Query<(&EditableText, &ChildOf), Changed<EditableText>>,
    tile_fields: Query<&TileField>,
    attachment_fields: Query<&AttachmentField>,
    btl: Option<ResMut<Btl>>,
    bin: Option<ResMut<Bin>>,
    mut dirty: ResMut<DirtyTiles>,
) {
    let (Some(mut btl), Some(mut bin)) = (btl, bin) else {
        return;
    };

    for (text, child_of) in &changed {
        let text = text.value().to_string();
        let Some(value) = text.trim().parse::<i32>().ok() else {
            continue;
        };
        let root = child_of.parent();

        if let Ok(field) = tile_fields.get(root) {
            apply_tile_field(&mut btl, &mut bin, &mut dirty, field, value);
        } else if let Ok(field) = attachment_fields.get(root) {
            apply_attachment_field(&mut btl, &mut dirty, field, value);
        }
    }
}

/// 写入地块字段，返回实际存储值。
fn apply_tile_field(
    btl: &mut Btl,
    bin: &mut Bin,
    dirty: &mut DirtyTiles,
    field: &TileField,
    value: i32,
) -> Option<i32> {
    let before = tile_field_value(btl, bin, field.grid, field.field);
    if !set_tile_field(btl, bin, field.grid, field.field, value) {
        return None;
    }

    let stored = tile_field_value(btl, bin, field.grid, field.field);
    if stored != before {
        dirty.mark(field.grid);
    }

    stored
}

/// 写入依附字段，返回实际存储值。
fn apply_attachment_field(
    btl: &mut Btl,
    dirty: &mut DirtyTiles,
    field: &AttachmentField,
    value: i32,
) -> Option<i32> {
    let before = field_value(btl, field.attachment, field.field);
    if !set_field_value(btl, field.attachment, field.field, value) {
        return None;
    }

    let stored = field_value(btl, field.attachment, field.field);
    if stored != before {
        dirty.mark(field.grid);
    }

    stored
}

/// 将数据中的实际值同步到输入框，保证拖拽与截断后显示一致。
///
/// 输入框可能在事件排队的同一帧内随面板重建而销毁，因此使用容错写入。
fn sync_input(commands: &mut Commands, input: Entity, stored: Option<i32>) {
    if let Some(stored) = stored {
        commands.entity(input).try_insert(NumberInputValue::I32(stored));
    }
}

/// 重建地块信息列表。
fn rebuild_details(
    commands: &mut Commands,
    details: Entity,
    summary: Option<&TileSummary>,
    btl: Option<&Btl>,
    bin: Option<&Bin>,
) {
    commands.entity(details).despawn_related::<Children>();

    let Some(summary) = summary else {
        let note = commands.spawn_scene(label_small("No tile selected")).id();
        commands.entity(details).add_child(note);
        return;
    };

    let mut rows = detail_rows(summary)
        .into_iter()
        .map(|(label, value)| spawn_detail_row(commands, label, &value))
        .collect::<Vec<_>>();

    if let (Some(btl), Some(bin)) = (btl, bin) {
        for (field, name) in tile_field_names().iter().enumerate() {
            let Some(value) = tile_field_value(btl, bin, summary.grid, field) else {
                continue;
            };

            let (row, input) = spawn_field_row(commands, name, tile_input(value));
            commands.entity(input).try_insert(TileField {
                grid: summary.grid,
                field,
            });
            rows.push(row);
        }
    }

    commands.entity(details).add_children(&rows);
}

/// 重建依附对象分组：每个实例对应一个分组。
fn rebuild_groups(
    commands: &mut Commands,
    container: Entity,
    grid: Option<IVec2>,
    btl: Option<&Btl>,
    attachments: &[Attachment],
) {
    commands.entity(container).despawn_related::<Children>();

    let (Some(grid), Some(btl)) = (grid, btl) else {
        return;
    };

    if attachments.is_empty() {
        let note = commands.spawn_scene(label_small("No attached objects")).id();
        commands.entity(container).add_child(note);
        return;
    }

    let groups = attachments
        .iter()
        .map(|&attachment| spawn_attachment_group(commands, btl, grid, attachment))
        .collect::<Vec<_>>();

    commands.entity(container).add_children(&groups);
}

/// 构建单个依附实例的分组。
fn spawn_attachment_group(
    commands: &mut Commands,
    btl: &Btl,
    grid: IVec2,
    attachment: Attachment,
) -> Entity {
    let title = format!("{} #{}", attachment.kind.label(), attachment.ordinal());
    let header = commands.spawn_scene(group_header_title(title)).id();

    let body = commands.spawn_scene(group_body()).id();
    let mut rows = Vec::new();

    for (field, name) in field_names(attachment.kind).iter().enumerate() {
        let Some(value) = field_value(btl, attachment, field) else {
            continue;
        };

        let (row, input) = spawn_field_row(commands, name, attachment_input(value));
        commands.entity(input).try_insert(AttachmentField {
            grid,
            attachment,
            field,
        });
        rows.push(row);
    }

    if rows.is_empty() {
        rows.push(commands.spawn_scene(label_small("No editable fields")).id());
    }

    commands.entity(body).add_children(&rows);

    let group = commands.spawn_scene(group()).id();
    commands.entity(group).add_children(&[header, body]);

    group
}

/// 构建名称与数字输入框组成的字段行，返回行实体与输入框实体。
fn spawn_field_row(
    commands: &mut Commands,
    name: &'static str,
    input: impl Scene,
) -> (Entity, Entity) {
    let label = commands.spawn_scene(label_small(name)).id();
    commands.entity(label).try_insert(Node {
        flex_grow: 1.0,
        min_width: px(0),
        ..default()
    });

    let input = commands.spawn_scene(input).id();

    let row = commands
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(SPACE_MD),
            min_height: px(FIELD_ROW_HEIGHT),
            ..default()
        })
        .id();
    commands.entity(row).add_children(&[label, input]);

    (row, input)
}

/// 构建绑定到地块字段的输入框。
fn tile_input(value: i32) -> impl Scene {
    bsn! {
        @FeathersNumberInput
        NumberInputValue::I32(value)
        Node { width: px(FIELD_INPUT_WIDTH) }
        on(on_tile_field)
    }
}

/// 构建绑定到依附对象字段的输入框。
fn attachment_input(value: i32) -> impl Scene {
    bsn! {
        @FeathersNumberInput
        NumberInputValue::I32(value)
        Node { width: px(FIELD_INPUT_WIDTH) }
        on(on_attachment_field)
    }
}

/// 构建分组标题栏。
fn group_header_title(title: String) -> impl Scene {
    bsn! {
        @group_header()
        Children [
            @caption(title)
        ]
    }
}

/// 将地块摘要格式化为只读字段行。
fn detail_rows(summary: &TileSummary) -> [(&'static str, String); 3] {
    let features = [(summary.water, "Water"), (summary.road, "Road"), (summary.coast, "Coast")]
        .into_iter()
        .filter_map(|(enabled, label)| enabled.then_some(label))
        .collect::<Vec<_>>();

    [
        ("Grid", format!("{}, {}", summary.grid.x, summary.grid.y)),
        ("Map", format!("{}, {}", summary.map.x, summary.map.y)),
        (
            "Features",
            if features.is_empty() {
                "—".to_owned()
            } else {
                features.join(", ")
            },
        ),
    ]
}

/// 构建单个只读字段行。
fn spawn_detail_row(commands: &mut Commands, label: &'static str, value: &str) -> Entity {
    let label = commands.spawn_scene(label_small(label)).id();
    let value = commands.spawn_scene(label_small(value)).id();

    let row = commands
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(SPACE_MD),
            min_height: px(FIELD_ROW_HEIGHT),
            ..default()
        })
        .id();
    commands.entity(row).add_children(&[label, value]);

    row
}

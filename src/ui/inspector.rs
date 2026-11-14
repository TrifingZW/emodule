//! 检查器面板场景。

use std::collections::HashSet;

use bevy::{
    feathers::{
        containers::{group, group_body, group_header, pane, pane_body, pane_header},
        controls::{FeathersNumberInput, NumberInputStep, NumberInputValue},
        display::{caption, label_small},
        theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor},
        tokens,
    },
    math::Rot2,
    picking::{Pickable, events::PointerClick},
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
const FIELD_INPUT_WIDTH: f32 = 96.0;
/// 字段行高度（逻辑像素），与 Feathers 控件行高一致。
const FIELD_ROW_HEIGHT: f32 = 32.0;
/// 字段行的水平内边距（逻辑像素）。
const FIELD_ROW_PADDING: f32 = 4.0;
/// 字段行之间的间距（逻辑像素）。
const FIELD_ROW_GAP: f32 = 0.0;
/// 字段行圆角（逻辑像素），与 Feathers 输入框模板保持一致。
const FIELD_ROW_RADIUS: f32 = 0.0;
/// 分组标题栏高度（逻辑像素）。
const GROUP_HEADER_HEIGHT: f32 = 28.0;
/// 数字输入框的拖拽步长。
///
/// Feathers 以 `步长 × 0.01` 换算每像素的数值增量，默认整数步长需要拖动
/// 100 像素才改变 1；此处提高到 20，使 5 像素对应 1 个数值。
const FIELD_DRAG_STEP: f64 = 20.0;

/// 标记地块信息分组容器。
#[derive(Component, Clone, Default)]
pub(crate) struct TileGroup;

/// 标记地块信息列表容器。
#[derive(Component, Clone, Default)]
pub(crate) struct TileDetails;

/// 标记依附对象分组容器。
#[derive(Component, Clone, Default)]
pub(crate) struct AttachmentGroups;

/// 记录检查器分组折叠状态的持久容器。
#[derive(Component, Clone, Default)]
pub(crate) struct GroupCollapseState {
    /// 已折叠的分组标题。
    collapsed: HashSet<String>,
}

impl GroupCollapseState {
    /// 判断指定分组是否处于折叠状态。
    fn is_collapsed(&self, key: &str) -> bool {
        self.collapsed.contains(key)
    }
}

/// 分组标题栏的折叠交互信息。
#[derive(Component)]
struct GroupHeader {
    /// 持有折叠状态的持久容器。
    owner: Entity,
    /// 分组标题，用作状态键。
    key: String,
    /// 折叠主体的实体。
    body: Entity,
    /// 折叠指示符实体。
    marker: Entity,
}

/// 标记分组折叠指示符。
#[derive(Component, Clone, Default)]
struct GroupMarker;

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
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.0,
                            row_gap: px(SPACE_MD),
                        }
                        TileGroup
                        GroupCollapseState::default()
                        --
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.0,
                            row_gap: px(SPACE_MD),
                        }
                        AttachmentGroups
                        GroupCollapseState::default()
                    ]
                ]
            ]
        ]
    }
}

/// 构建字段分组：统一分组边框、标题栏与主体容器。
///
/// 返回分组根与主体容器，主体内容由调用方填充。
fn spawn_field_group(
    commands: &mut Commands,
    owner: Entity,
    title: impl Into<String>,
    collapsed: bool,
) -> (Entity, Entity) {
    let title = title.into();
    let display = if collapsed {
        Display::None
    } else {
        Display::Flex
    };
    let rotation = if collapsed {
        Rot2::turn_fraction(0.75)
    } else {
        Rot2::IDENTITY
    };

    let body = commands.spawn_scene(field_group_body(display)).id();
    let header = commands.spawn_scene(group_title(title.clone())).id();
    let marker = commands.spawn_scene(group_marker(rotation)).id();
    commands.entity(header).add_child(marker);
    commands.entity(header).insert(GroupHeader {
        owner,
        key: title,
        body,
        marker,
    });

    let group = commands.spawn_scene(field_group()).id();
    commands.entity(group).add_children(&[header, body]);

    (group, body)
}

/// 构建分组外壳：统一的分组边框与底色。
fn field_group() -> impl Scene {
    bsn! {
        @group()
        Node {
            border: px(2),
        }
    }
}

/// 构建分组标题栏：深色标题带、底部细分隔线与圆角顶边。
fn group_title(title: impl Into<String>) -> impl Scene {
    bsn! {
        @group_header()
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(SPACE_SM),
            min_height: px(GROUP_HEADER_HEIGHT),
            padding: UiRect::horizontal(px(SPACE_MD + FIELD_ROW_PADDING)),
            border: UiRect::all(px(0)),
            border_radius: BorderRadius::top(px(3)),
        }
        ThemeBackgroundColor(tokens::PANE_BODY_BG)
        ThemeBorderColor(tokens::GROUP_BORDER)
        Pickable::default()
        on(toggle_group)
        Children [
            @caption(title)
            Pickable::IGNORE
        ]
    }
}

/// 构建折叠指示符：以 UI 边框绘制矢量式 chevron，折叠时旋转为右向。
fn group_marker(rotation: Rot2) -> impl Scene {
    bsn! {
        Node {
            width: px(12),
            height: px(12),
            flex_shrink: 0.0,
            display: Display::Flex,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        GroupMarker
        Pickable::IGNORE
        UiTransform::from_rotation(rotation)
        Children [
            Node {
                width: px(6),
                height: px(6),
                border: UiRect::new(px(0), px(2), px(0), px(2)),
            }
            Pickable::IGNORE
            UiTransform::from_rotation(Rot2::turn_fraction(0.125))
            ThemeBorderColor(tokens::TEXT_DIM)
        ]
    }
}

/// 构建分组主体：等宽字段行、紧凑行距与统一内边距。
fn field_group_body(display: Display) -> impl Scene {
    bsn! {
        @group_body()
        Node {
            display: display,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(FIELD_ROW_GAP),
            padding: UiRect::all(px(0)),
        }
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
    tile_group: Query<(Entity, &GroupCollapseState), With<TileGroup>>,
    details: Query<Entity, With<TileDetails>>,
    groups: Query<(Entity, &GroupCollapseState), With<AttachmentGroups>>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let (Ok((tile_group, tile_state)), Ok((groups, group_state))) =
        (tile_group.single(), groups.single())
    else {
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

    // 分组外壳只在首次构建，之后仅重建主体内容，避免整块 UI 重建造成闪烁。
    let details = match details.single() {
        Ok(body) => body,
        Err(_) => {
            let (group, body) = spawn_field_group(
                &mut commands,
                tile_group,
                "Tile",
                tile_state.is_collapsed("Tile"),
            );
            commands.entity(tile_group).add_child(group);
            commands.entity(body).insert(TileDetails);
            body
        }
    };

    rebuild_details(&mut commands, details, summary.as_ref(), btl.as_deref(), bin.as_deref());
    rebuild_groups(&mut commands, groups, group_state, grid, btl.as_deref(), &attachments);
}

/// 切换分组主体的显示状态，并持久化折叠状态。
fn toggle_group(
    click: On<PointerClick>,
    headers: Query<&GroupHeader>,
    mut bodies: Query<&mut Node>,
    mut markers: Query<&mut UiTransform, With<GroupMarker>>,
    mut owners: Query<&mut GroupCollapseState>,
    mut focus: ResMut<InputFocus>,
) {
    let Ok(header) = headers.get(click.event_target()) else {
        return;
    };
    let Ok(mut body) = bodies.get_mut(header.body) else {
        return;
    };
    let Ok(mut state) = owners.get_mut(header.owner) else {
        return;
    };

    let collapsed = if state.collapsed.remove(&header.key) {
        false
    } else {
        state.collapsed.insert(header.key.clone());
        true
    };

    body.display = if collapsed {
        Display::None
    } else {
        Display::Flex
    };

    if collapsed && focus.get().is_some() {
        focus.clear();
    }

    if let Ok(mut marker) = markers.get_mut(header.marker) {
        marker.rotation = if collapsed {
            Rot2::turn_fraction(0.75)
        } else {
            Rot2::IDENTITY
        };
    }
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
    body: Entity,
    summary: Option<&TileSummary>,
    btl: Option<&Btl>,
    bin: Option<&Bin>,
) {
    commands.entity(body).despawn_related::<Children>();

    let Some(summary) = summary else {
        let note = commands.spawn_scene(placeholder_label("No tile selected")).id();
        commands.entity(body).add_child(note);
        return;
    };

    let mut rows = detail_rows(summary)
        .into_iter()
        .enumerate()
        .map(|(index, (label, value))| spawn_detail_row(commands, label, value, index % 2 == 1))
        .collect::<Vec<_>>();

    if let (Some(btl), Some(bin)) = (btl, bin) {
        for (field, name) in tile_field_names().iter().enumerate() {
            let Some(value) = tile_field_value(btl, bin, summary.grid, field) else {
                continue;
            };

            let striped = rows.len() % 2 == 1;
            let (row, input) = spawn_field_row(commands, name, tile_input(value), striped);
            commands.entity(input).try_insert(TileField {
                grid: summary.grid,
                field,
            });
            rows.push(row);
        }
    }

    commands.entity(body).add_children(&rows);
}

/// 重建依附对象分组：每个实例对应一个分组。
fn rebuild_groups(
    commands: &mut Commands,
    container: Entity,
    collapse: &GroupCollapseState,
    grid: Option<IVec2>,
    btl: Option<&Btl>,
    attachments: &[Attachment],
) {
    commands.entity(container).despawn_related::<Children>();

    let (Some(grid), Some(btl)) = (grid, btl) else {
        return;
    };

    if attachments.is_empty() {
        let note = commands.spawn_scene(placeholder_label("No attached objects")).id();
        commands.entity(container).add_child(note);
        return;
    }

    let groups = attachments
        .iter()
        .map(|&attachment| {
            let title = format!("{} #{}", attachment.kind.label(), attachment.ordinal());
            let collapsed = collapse.is_collapsed(&title);
            spawn_attachment_group(commands, container, title, collapsed, btl, grid, attachment)
        })
        .collect::<Vec<_>>();

    commands.entity(container).add_children(&groups);
}

/// 构建单个依附实例的分组。
fn spawn_attachment_group(
    commands: &mut Commands,
    owner: Entity,
    title: String,
    collapsed: bool,
    btl: &Btl,
    grid: IVec2,
    attachment: Attachment,
) -> Entity {
    let (group, body) = spawn_field_group(commands, owner, title, collapsed);

    let mut rows = Vec::new();

    for (field, name) in field_names(attachment.kind).iter().enumerate() {
        let Some(value) = field_value(btl, attachment, field) else {
            continue;
        };

        let striped = rows.len() % 2 == 1;
        let (row, input) = spawn_field_row(commands, name, attachment_input(value), striped);
        commands.entity(input).try_insert(AttachmentField {
            grid,
            attachment,
            field,
        });
        rows.push(row);
    }

    if rows.is_empty() {
        rows.push(commands.spawn_scene(placeholder_label("No editable fields")).id());
    }

    commands.entity(body).add_children(&rows);

    group
}

/// 构建名称与数字输入框组成的字段行，返回行实体与输入框实体。
fn spawn_field_row(
    commands: &mut Commands,
    name: &'static str,
    input: impl Scene,
    striped: bool,
) -> (Entity, Entity) {
    let label = commands.spawn_scene(field_label(name)).id();
    let input = commands.spawn_scene(input).id();

    let row = spawn_row(commands, &[label, input], striped);

    (row, input)
}

/// 构建行容器，交替行使用弱化底色区分相邻条目。
fn spawn_row(commands: &mut Commands, children: &[Entity], striped: bool) -> Entity {
    let row = commands
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(SPACE_MD),
            min_height: px(FIELD_ROW_HEIGHT),
            border_radius: BorderRadius::all(px(FIELD_ROW_RADIUS)),
            padding: UiRect::all(px(4)),
            ..default()
        })
        .id();

    if striped {
        commands.entity(row).insert(ThemeBackgroundColor(tokens::PANE_BODY_BG));
    }

    commands.entity(row).add_children(children);

    row
}

/// 构建绑定到地块字段的输入框。
///
/// `Node` 在 bsn 中只覆盖声明的字段，因此需显式关闭模板的 `flex_grow`，
/// 否则输入框宽度会随标签长度变化而失去竖向对齐。
fn tile_input(value: i32) -> impl Scene {
    bsn! {
        @FeathersNumberInput
        NumberInputValue::I32(value)
        NumberInputStep(FIELD_DRAG_STEP)
        Node {
            width: px(FIELD_INPUT_WIDTH),
            flex_grow: 0.0,
            flex_shrink: 0.0,
        }
        on(on_tile_field)
    }
}

/// 构建绑定到依附对象字段的输入框。
fn attachment_input(value: i32) -> impl Scene {
    bsn! {
        @FeathersNumberInput
        NumberInputValue::I32(value)
        NumberInputStep(FIELD_DRAG_STEP)
        Node {
            width: px(FIELD_INPUT_WIDTH),
            flex_grow: 0.0,
            flex_shrink: 0.0,
        }
        on(on_attachment_field)
    }
}

/// 构建字段标签：弱化颜色、单行显示并在超出时裁剪。
fn field_label(text: &'static str) -> impl Scene {
    bsn! {
        @label_small(text)
        ThemeTextColor(tokens::TEXT_DIM)
        TextLayout::no_wrap()
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            padding: UiRect::left(px(FIELD_ROW_PADDING)),
            overflow: Overflow::clip(),
        }
    }
}

/// 构建只读字段值：单行显示并在超出时裁剪。
fn value_label(text: String) -> impl Scene {
    bsn! {
        @label_small(text)
        TextLayout::no_wrap()
        Node {
            min_width: px(0),
            padding: UiRect::right(px(FIELD_ROW_PADDING)),
            overflow: Overflow::clip(),
        }
    }
}

/// 构建空状态提示。
fn placeholder_label(text: &'static str) -> impl Scene {
    bsn! {
        @label_small(text)
        ThemeTextColor(tokens::TEXT_DIM)
        Node {
            padding: UiRect::axes(px(FIELD_ROW_PADDING), px(SPACE_SM)),
        }
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
fn spawn_detail_row(
    commands: &mut Commands,
    label: &'static str,
    value: String,
    striped: bool,
) -> Entity {
    let label = commands.spawn_scene(field_label(label)).id();
    let value = commands.spawn_scene(value_label(value)).id();

    spawn_row(commands, &[label, value], striped)
}

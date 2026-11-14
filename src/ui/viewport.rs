//! 地图视口、相机输入层与浮动工具栏。

use bevy::{
    feathers::{
        containers::pane_header_divider,
        controls::{ButtonVariant, FeathersCheckbox, FeathersToolButton},
        display::{caption, label_dim},
        theme::{ThemeBackgroundColor, ThemeBorderColor},
        tokens,
    },
    prelude::*,
    ui_widgets::checkbox_self_update,
};

use crate::{
    camera::{MapViewport, pan_camera, zoom_camera},
    state::{Tool, ToolButton, on_grid_change, on_tool_activate},
    ui::BORDER_RADIUS,
    wc4::selection::select_tile_on_click,
};

use super::{BORDER_WIDTH, CONTROL_GAP, SPACE_LG, SPACE_MD, SPACE_SM, VIEWPORT_INSET};

/// 构建视口区域及其浮动覆盖层。
pub(super) fn viewport() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
        }
        MapViewport
        Children [
            // 覆盖层后方的输入层：左键点选地块，中键拖拽平移，滚轮缩放。
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
            }
            on(select_tile_on_click)
            on(pan_camera)
            on(zoom_camera)
            --
            @viewport_toolbar()
            --
            Node {
                position_type: PositionType::Absolute,
                left: px(VIEWPORT_INSET),
                bottom: px(SPACE_LG),
                padding: UiRect::axes(px(SPACE_MD), px(3)),
                border_radius: BorderRadius::all(px(SPACE_SM)),
            }
            ThemeBackgroundColor(tokens::PANE_HEADER_BG)
            ThemeBorderColor(tokens::PANE_HEADER_BORDER)
            Children [
                @label_dim("MMB drag to pan / wheel to zoom")
            ]
        ]
    }
}

/// 构建包含工具选择与网格可见性的浮动工具栏。
fn viewport_toolbar() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(VIEWPORT_INSET),
            top: px(VIEWPORT_INSET),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(CONTROL_GAP),
            padding: UiRect::axes(px(SPACE_SM), px(SPACE_SM)),
            border: px(BORDER_WIDTH),
            border_radius: BorderRadius::all(px(BORDER_RADIUS)),
        }
        ThemeBackgroundColor(tokens::PANE_HEADER_BG)
        ThemeBorderColor(tokens::PANE_HEADER_BORDER)
        Children [
            @tool_button("选择", Tool::Select, ButtonVariant::Primary)
            --
            @tool_button("移动", Tool::Move, ButtonVariant::Normal)
            --
            @tool_button("旋转", Tool::Rotate, ButtonVariant::Normal)
            --
            @tool_button("缩放", Tool::Scale, ButtonVariant::Normal)
            --
            @pane_header_divider()
            --
            @FeathersCheckbox {
                @caption: bsn! { @caption("网格") },
            }
            on(checkbox_self_update)
            on(on_grid_change)
        ]
    }
}

/// 构建激活 `tool` 的工具栏按钮。
fn tool_button(text: &'static str, tool: Tool, variant: ButtonVariant) -> impl Scene {
    bsn! {
        @FeathersToolButton {
            @variant: {variant},
            @caption: bsn! { @caption(text) },
        }
        ToolButton(tool)
        on(on_tool_activate)
    }
}

//! 编辑器外壳场景与共享布局常量。

mod dialogs;
mod inspector;
mod menu;
mod status;
mod viewport;

use bevy::{feathers::palette, input_focus::tab_navigation::TabGroup, prelude::*};

use crate::camera::MapCamera;

pub(crate) use inspector::{pointer_released, update_tile_details, write_fields_from_text};
pub(crate) use status::update_status;

/// 检查器面板宽度（逻辑像素）。
pub(crate) const INSPECTOR_WIDTH: f32 = 350.0;

/// 视口清屏颜色。
pub(crate) const VIEWPORT_CLEAR_COLOR: Color = palette::BLACK;

/// 状态栏高度（逻辑像素）。
pub(crate) const STATUS_BAR_HEIGHT: f32 = 24.0;

/// 视口内浮动覆盖层的内边距。
pub(crate) const VIEWPORT_INSET: f32 = 8.0;

/// 统一的边框宽度（逻辑像素）。
pub(crate) const BORDER_WIDTH: f32 = 2.0;

pub(crate) const BORDER_RADIUS: f32 = 6.0;

/// 相关控件之间的小间距。
pub(crate) const SPACE_SM: f32 = 4.0;

/// 同一行控件之间的间距。
pub(crate) const CONTROL_GAP: f32 = 6.0;

/// 紧凑表面的中等间距。
pub(crate) const SPACE_MD: f32 = 6.0;

/// 面板主体的较大间距。
pub(crate) const SPACE_LG: f32 = 8.0;

/// 包含地图相机与编辑器 UI 的场景列表。
pub(crate) fn root_scene() -> impl SceneList {
    bsn_list! {
        Camera2d
        MapCamera
        --
        @editor_root()
    }
}

/// 构建编辑器布局：菜单栏、主区域与状态栏。
fn editor_root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
        }
        TabGroup
        Children [
            @menu::menu_bar()
            --
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_height: px(0),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
            }
            Children [
                @viewport::viewport()
                --
                @inspector::inspector()
            ]
            --
            @status::status_bar()
        ]
    }
}

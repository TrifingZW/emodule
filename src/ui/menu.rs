//! 菜单栏场景与菜单动作。

use bevy::{
    feathers::{
        containers::flex_spacer,
        controls::{
            FeathersMenu, FeathersMenuButton, FeathersMenuDivider, FeathersMenuItem,
            FeathersMenuPopup,
        },
        display::caption,
        theme::{ThemeBackgroundColor, ThemeBorderColor},
        tokens,
    },
    prelude::*,
    ui_widgets::Activate,
};

use crate::{
    camera::reset_camera,
    ui::{
        BORDER_WIDTH, CONTROL_GAP, SPACE_MD,
        dialogs::{show_about, spawn_quit_dialog},
    },
};

/// 构建应用菜单栏。
pub(super) fn menu_bar() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(SPACE_MD), px(SPACE_MD)),
            column_gap: px(CONTROL_GAP),
            border: UiRect::bottom(px(BORDER_WIDTH)),
        }
        ThemeBackgroundColor(tokens::PANE_HEADER_BG)
        ThemeBorderColor(tokens::PANE_HEADER_BORDER)
        Children [
            // 每个菜单只拥有一个弹出层，因此各菜单需要独立容器。
            @FeathersMenu
            Children [
                @FeathersMenuButton {
                    @caption: bsn! { @caption("File") }
                }
                --
                @FeathersMenuPopup
                Children [
                    @FeathersMenuItem {
                        @caption: bsn! { @caption("New") }
                    }
                    on(new_map)
                    --
                    @FeathersMenuItem {
                        @caption: bsn! { @caption("Open...") }
                    }
                    on(open_map)
                    --
                    @FeathersMenuItem {
                        @caption: bsn! { @caption("Save") }
                    }
                    on(save_map)
                    --
                    @FeathersMenuDivider
                    --
                    @FeathersMenuItem {
                        @caption: bsn! { @caption("Quit") }
                    }
                    on(spawn_quit_dialog)
                ]
            ]
            --
            @FeathersMenu
            Children [
                @FeathersMenuButton {
                    @caption: bsn! { @caption("View") }
                }
                --
                @FeathersMenuPopup
                Children [
                    @FeathersMenuItem {
                        @caption: bsn! { @caption("Reset Camera") }
                    }
                    on(reset_camera)
                ]
            ]
            --
            @FeathersMenu
            Children [
                @FeathersMenuButton {
                    @caption: bsn! { @caption("Help") }
                }
                --
                @FeathersMenuPopup
                Children [
                    @FeathersMenuItem {
                        @caption: bsn! { @caption("About") }
                    }
                    on(show_about)
                ]
            ]
            --
            @flex_spacer()
        ]
    }
}

/// 处理 `File > New` 动作。
fn new_map(_: On<Activate>) {
    info!("New map");
}

/// 处理 `File > Open...` 动作。
fn open_map(_: On<Activate>) {
    info!("Open map");
}

/// 处理 `File > Save` 动作。
fn save_map(_: On<Activate>) {
    info!("Save map");
}

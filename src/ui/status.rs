//! 状态栏场景与状态显示。

use bevy::{
    feathers::{
        containers::flex_spacer,
        display::{label_dim, label_small},
        theme::{ThemeBackgroundColor, ThemeBorderColor},
        tokens,
    },
    prelude::*,
};

use crate::state::{EditorState, StatusText};

use super::{BORDER_WIDTH, SPACE_LG, STATUS_BAR_HEIGHT};

/// 构建底部状态栏。
pub(super) fn status_bar() -> impl Scene {
    bsn! {
        Node {
            height: px(STATUS_BAR_HEIGHT),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Start,
            column_gap: px(SPACE_LG),
            padding: UiRect::horizontal(px(SPACE_LG)),
            border: UiRect::top(px(BORDER_WIDTH)),
        }
        ThemeBackgroundColor(tokens::PANE_HEADER_BG)
        ThemeBorderColor(tokens::PANE_HEADER_BORDER)
        Children [
            @label_dim("Ready") StatusText
            --
            @flex_spacer()
            --
            @label_small("bevy_ui + feathers")
        ]
    }
}

/// 当编辑器状态变化时刷新状态文本。
pub(crate) fn update_status(
    state: Res<EditorState>,
    mut status: Single<&mut Text, With<StatusText>>,
) {
    status.0 = state.status_line();
}

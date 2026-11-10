//! 检查器面板场景。

use bevy::{
    feathers::{
        containers::{flex_spacer, group, group_body, group_header, pane, pane_body, pane_header},
        controls::{
            ColorInputValue, FeathersCheckbox, FeathersColorInput, FeathersNumberInput,
            FeathersSlider, NumberInputPrecision, NumberInputValue, color_input_self_update,
        },
        display::{caption, label_small},
        theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeToken},
        tokens,
    },
    prelude::*,
    ui::Checked,
    ui_widgets::{SliderPrecision, SliderStep, SliderValue},
};

use crate::state::{
    DEFAULT_COLOR, DEFAULT_POSITION, DEFAULT_ROTATION, DEFAULT_SCALE, PositionAxis, ROTATION_MAX,
    ROTATION_MIN, SCALE_MAX, SCALE_MIN, on_color, on_position_axis, on_rotation, on_scale,
    on_visibility_change,
};

use super::{BORDER_WIDTH, CONTROL_GAP, INSPECTOR_WIDTH, SPACE_LG, SPACE_SM};

/// 构建检查器面板。
pub(super) fn inspector() -> impl Scene {
    bsn! {
        Node {
            width: px(INSPECTOR_WIDTH),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            padding: px(SPACE_LG),
            border: UiRect::left(px(BORDER_WIDTH)),
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        ThemeBorderColor(tokens::PANE_HEADER_BORDER)
        Children [
            @pane()
            Node { flex_grow: 1.0 }
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
                    border_radius: BorderRadius::ZERO,
                    padding: px(SPACE_SM),
                }
                Children [
                    @transform_group()
                    --
                    @display_group()
                    --
                    @flex_spacer()
                ]
            ]
        ]
    }
}

/// 构建变换控件组。
fn transform_group() -> impl Scene {
    bsn! {
        @group()
        Children [
            @group_header()
            Children [
                @caption("Transform")
            ]
            --
            @group_body()
            Children [
                @label_small("Position")
                --
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(CONTROL_GAP),
                    align_items: AlignItems::Center,
                }
                Children [
                    @position_input(PositionAxis::X, DEFAULT_POSITION.x, "X", tokens::TEXT_INPUT_X_AXIS)
                    --
                    @position_input(PositionAxis::Y, DEFAULT_POSITION.y, "Y", tokens::TEXT_INPUT_Y_AXIS)
                    --
                    @position_input(PositionAxis::Z, DEFAULT_POSITION.z, "Z", tokens::TEXT_INPUT_Z_AXIS)
                ]
                --
                @label_small("Rotation")
                --
                @FeathersSlider {
                    @min: {ROTATION_MIN},
                    @max: {ROTATION_MAX},
                }
                SliderValue(DEFAULT_ROTATION)
                SliderPrecision(0)
                SliderStep(1.0)
                Node { flex_grow: 1.0 }
                on(on_rotation)
                --
                @label_small("Scale")
                --
                @FeathersSlider {
                    @min: {SCALE_MIN},
                    @max: {SCALE_MAX},
                }
                SliderValue(DEFAULT_SCALE)
                SliderPrecision(2)
                Node { flex_grow: 1.0 }
                on(on_scale)
            ]
        ]
    }
}

/// 构建显示控件组。
fn display_group() -> impl Scene {
    bsn! {
        @group()
        Children [
            @group_header()
            Children [
                @caption("Display")
            ]
            --
            @group_body()
            Children [
                @FeathersCheckbox {
                    @caption: bsn! { @caption("Visible") },
                }
                Checked
                on(on_visibility_change)
                --
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(SPACE_SM),
                }
                Children [
                    @label_small("Color")
                    --
                    @FeathersColorInput
                    ColorInputValue(DEFAULT_COLOR)
                    on(color_input_self_update)
                    on(on_color)
                ]
            ]
        ]
    }
}

/// 构建绑定到单个位置轴的数字输入框。
fn position_input(
    axis: PositionAxis,
    value: f32,
    label: &'static str,
    sigil_color: ThemeToken,
) -> impl Scene {
    bsn! {
        @FeathersNumberInput {
            @sigil_color: {sigil_color},
            @label_text: {label},
        }
        NumberInputValue::F32(value)
        NumberInputPrecision(1)
        Node { flex_grow: 1.0 }
        on(on_position_axis)
        axis
    }
}

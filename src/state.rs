//! 编辑器状态，以及保持控件与状态同步的观察者。

use bevy::{
    color::{Color, palettes},
    feathers::controls::ButtonVariant,
    prelude::*,
    ui_widgets::{Activate, ValueChange},
};

/// 视口工具栏中选中的编辑工具。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Tool {
    /// 选择地图实体。
    #[default]
    Select,
    /// 移动选中对象。
    Move,
    /// 旋转选中对象。
    Rotate,
    /// 缩放选中对象。
    Scale,
}

impl Tool {
    /// 返回工具面向用户的名称。
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
        }
    }
}

/// 位置数字输入框所编辑的坐标轴。
#[derive(Component, Clone, Copy)]
pub(crate) enum PositionAxis {
    /// 水平轴。
    X,
    /// 垂直轴。
    Y,
    /// 深度轴。
    Z,
}

/// 默认对象位置。
pub(crate) const DEFAULT_POSITION: Vec3 = Vec3::ZERO;

/// 默认对象旋转角度（度）。
pub(crate) const DEFAULT_ROTATION: f32 = 0.0;

/// 默认对象缩放。
pub(crate) const DEFAULT_SCALE: f32 = 1.0;

/// 默认对象颜色。
pub(crate) const DEFAULT_COLOR: Color = Color::Srgba(palettes::tailwind::EMERALD_600);

/// 最小旋转角度（度）。
pub(crate) const ROTATION_MIN: f32 = 0.0;

/// 最大旋转角度（度）。
pub(crate) const ROTATION_MAX: f32 = 360.0;

/// 最小对象缩放。
pub(crate) const SCALE_MIN: f32 = 0.1;

/// 最大对象缩放。
pub(crate) const SCALE_MAX: f32 = 4.0;

/// 被编辑对象及视图设置的检查器数值。
#[derive(Resource, Clone, Copy)]
pub(crate) struct EditorState {
    tool: Tool,
    show_grid: bool,
    position: Vec3,
    rotation_degrees: f32,
    scale: f32,
    color: Color,
    object_visible: bool,
}

impl Default for EditorState {
    /// 创建初始编辑器状态。
    fn default() -> Self {
        Self {
            tool: Tool::default(),
            show_grid: false,
            position: DEFAULT_POSITION,
            rotation_degrees: DEFAULT_ROTATION,
            scale: DEFAULT_SCALE,
            color: DEFAULT_COLOR,
            object_visible: true,
        }
    }
}

impl EditorState {
    /// 返回视口网格是否启用。
    pub(crate) const fn grid_enabled(&self) -> bool {
        self.show_grid
    }

    /// 将当前状态格式化为状态栏文本。
    pub(crate) fn status_line(&self) -> String {
        let grid = if self.show_grid {
            "On"
        } else {
            "Off"
        };
        let visible = if self.object_visible {
            "Yes"
        } else {
            "No"
        };
        let color = self.color.to_srgba();
        let red = (color.red * 255.0).round() as u8;
        let green = (color.green * 255.0).round() as u8;
        let blue = (color.blue * 255.0).round() as u8;

        format!(
            "Tool: {} | Pos: {:.1}, {:.1}, {:.1} | Rot: {:.0} | Scale: {:.2} | Color: #{red:02X}{green:02X}{blue:02X} | Grid: {grid} | Visible: {visible}",
            self.tool.label(),
            self.position.x,
            self.position.y,
            self.position.z,
            self.rotation_degrees,
            self.scale,
        )
    }

    /// 写入对象位置的单个分量。
    fn set_position_axis(&mut self, axis: PositionAxis, value: f32) {
        match axis {
            PositionAxis::X => self.position.x = value,
            PositionAxis::Y => self.position.y = value,
            PositionAxis::Z => self.position.z = value,
        }
    }
}

/// 标记激活某个工具的视口工具栏按钮。
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct ToolButton(pub(crate) Tool);

/// 标记状态栏文本。
#[derive(Component, Clone, Default)]
pub(crate) struct StatusText;

/// 激活被按下工具栏按钮所关联的工具。
pub(crate) fn on_tool_activate(
    activate: On<Activate>,
    buttons: Query<(Entity, &ToolButton)>,
    mut state: ResMut<EditorState>,
    mut commands: Commands,
) {
    let Ok((_, button)) = buttons.get(activate.event_target()) else {
        return;
    };

    let tool = button.0;
    state.tool = tool;

    for (entity, button) in &buttons {
        commands.entity(entity).insert(ButtonVariant::selected(button.0 == tool));
    }
}

/// 更新网格可见性。
pub(crate) fn on_grid_change(change: On<ValueChange<bool>>, mut state: ResMut<EditorState>) {
    state.show_grid = change.value;
}

/// 更新对象可见性。
pub(crate) fn on_visibility_change(change: On<ValueChange<bool>>, mut state: ResMut<EditorState>) {
    state.object_visible = change.value;
}

/// 更新对象位置的单个分量。
pub(crate) fn on_position_axis(
    change: On<ValueChange<f32>>,
    axes: Query<&PositionAxis>,
    mut state: ResMut<EditorState>,
) {
    let Ok(&axis) = axes.get(change.event_target()) else {
        return;
    };
    let Some(value) = finite(change.value) else {
        return;
    };

    state.set_position_axis(axis, value);
}

/// 更新对象旋转角度（度）。
pub(crate) fn on_rotation(change: On<ValueChange<f32>>, mut state: ResMut<EditorState>) {
    let Some(value) = finite(change.value) else {
        return;
    };

    state.rotation_degrees = value.clamp(ROTATION_MIN, ROTATION_MAX);
}

/// 更新对象缩放。
pub(crate) fn on_scale(change: On<ValueChange<f32>>, mut state: ResMut<EditorState>) {
    let Some(value) = finite(change.value) else {
        return;
    };

    state.scale = value.clamp(SCALE_MIN, SCALE_MAX);
}

/// 更新对象颜色。
pub(crate) fn on_color(change: On<ValueChange<Color>>, mut state: ResMut<EditorState>) {
    state.color = change.value;
}

/// 当数值为有限值时返回该值。
fn finite(value: f32) -> Option<f32> {
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_labels_match_their_variants() {
        assert_eq!(Tool::Select.label(), "Select");
        assert_eq!(Tool::Move.label(), "Move");
        assert_eq!(Tool::Rotate.label(), "Rotate");
        assert_eq!(Tool::Scale.label(), "Scale");
    }

    #[test]
    fn status_line_reports_the_current_state() {
        let state = EditorState {
            tool: Tool::Rotate,
            show_grid: false,
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation_degrees: 90.0,
            scale: 2.0,
            color: DEFAULT_COLOR,
            object_visible: false,
        };

        assert_eq!(
            state.status_line(),
            "Tool: Rotate | Pos: 1.0, 2.0, 3.0 | Rot: 90 | Scale: 2.00 | Color: #059669 | Grid: Off | Visible: No"
        );
    }

    #[test]
    fn finite_rejects_non_finite_values() {
        assert_eq!(finite(1.5), Some(1.5));
        assert_eq!(finite(f32::NAN), None);
        assert_eq!(finite(f32::INFINITY), None);
        assert_eq!(finite(f32::NEG_INFINITY), None);
    }
}

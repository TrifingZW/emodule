//! 编辑器状态，以及保持控件与状态同步的观察者。

use bevy::{
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

/// 编辑器与视图设置。
#[derive(Resource, Clone, Copy)]
pub(crate) struct EditorState {
    tool: Tool,
    show_grid: bool,
    fixed_line_width: bool,
    show_provinces: bool,
    show_countries: bool,
}

impl Default for EditorState {
    /// 创建初始编辑器状态。
    fn default() -> Self {
        Self {
            tool: Tool::default(),
            show_grid: false,
            fixed_line_width: false,
            show_provinces: false,
            show_countries: false,
        }
    }
}

impl EditorState {
    /// 返回视口网格是否启用。
    pub(crate) const fn grid_enabled(&self) -> bool {
        self.show_grid
    }

    /// 返回覆盖层线宽是否固定为屏幕像素。
    ///
    /// `true` 时线宽不随视角缩放，`false` 时按世界尺寸固定并与地图一同缩放，
    /// 对网格、省界与国界同时生效。
    pub(crate) const fn line_width_fixed(&self) -> bool {
        self.fixed_line_width
    }

    /// 返回省界是否启用。
    pub(crate) const fn province_boundaries_enabled(&self) -> bool {
        self.show_provinces
    }

    /// 返回国界是否启用。
    pub(crate) const fn country_boundaries_enabled(&self) -> bool {
        self.show_countries
    }

    /// 将当前状态格式化为状态栏文本。
    pub(crate) fn status_line(&self) -> String {
        let grid = if self.show_grid {
            "On"
        } else {
            "Off"
        };

        format!("Tool: {} | Grid: {grid}", self.tool.label())
    }
}

/// 当前选中的地块。
///
/// 坐标为地图网格坐标（列、行），与网格换算模块的输出保持一致；
/// `None` 表示未选中任何地块。
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SelectedTile(Option<IVec2>);

impl SelectedTile {
    /// 创建选中指定地块的状态。
    pub(crate) const fn at(tile: IVec2) -> Self {
        Self(Some(tile))
    }

    /// 返回选中的网格坐标，未选中时返回 `None`。
    pub(crate) const fn get(self) -> Option<IVec2> {
        self.0
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

/// 更新覆盖层线宽是否固定。
pub(crate) fn on_line_width_change(change: On<ValueChange<bool>>, mut state: ResMut<EditorState>) {
    state.fixed_line_width = change.value;
}

/// 更新省界可见性。
pub(crate) fn on_province_change(change: On<ValueChange<bool>>, mut state: ResMut<EditorState>) {
    state.show_provinces = change.value;
}

/// 更新国界可见性。
pub(crate) fn on_country_change(change: On<ValueChange<bool>>, mut state: ResMut<EditorState>) {
    state.show_countries = change.value;
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
            show_grid: true,
            fixed_line_width: false,
            show_provinces: false,
            show_countries: false,
        };

        assert_eq!(state.status_line(), "Tool: Rotate | Grid: On");
    }
}

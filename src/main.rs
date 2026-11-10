//! 基于 Bevy UI 与 Bevy Feathers 的极简地图编辑器外壳。
//!
//! 编辑器由菜单栏、带浮动工具栏的地图视口、检查器和状态栏组成。

mod camera;
mod map;
mod state;
mod ui;
mod wc4;

use bevy::{
    feathers::{FeathersPlugins, dark_theme::create_dark_theme, theme::UiTheme},
    prelude::*,
};

use crate::state::EditorState;
use crate::ui::VIEWPORT_CLEAR_COLOR;
use crate::wc4::Wc4Plugin;

/// 主窗口标题。
const WINDOW_TITLE: &str = "EModule";

/// 启动编辑器应用。
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: WINDOW_TITLE.into(),
                    ..default()
                }),
                ..default()
            }),
            FeathersPlugins,
            Wc4Plugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(ClearColor(VIEWPORT_CLEAR_COLOR))
        .init_resource::<EditorState>()
        .add_systems(Startup, ui::root_scene.spawn())
        .add_systems(
            Update,
            (map::draw_grid, ui::update_status.run_if(resource_changed::<EditorState>)),
        )
        .run();
}

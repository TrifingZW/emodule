//! 基于 Bevy UI 与 Bevy Feathers 的极简地图编辑器外壳。
//!
//! 编辑器由菜单栏、带浮动工具栏的地图视口、检查器和状态栏组成。

mod camera;
mod font;
mod map;
mod state;
mod ui;
mod wc4;

use bevy::{
    ecs::error::{BevyError, ErrorContext, FallbackErrorHandler},
    feathers::{FeathersPlugins, dark_theme::create_dark_theme, theme::UiTheme},
    prelude::*,
};

use crate::font::EditorFontPlugin;
use crate::state::{EditorState, SelectedTile};
use crate::ui::VIEWPORT_CLEAR_COLOR;
use crate::wc4::Wc4Plugin;

/// 主窗口标题。
const WINDOW_TITLE: &str = "EModule";

/// 处理未被单独处理的 ECS 错误。
///
/// 命令错误通常来自延迟执行时目标实体已被销毁，记录命令名与完整回溯后继续运行；
/// 其余错误保持默认的崩溃行为，避免掩盖真正的逻辑问题。
fn handle_error(error: BevyError, context: ErrorContext) {
    if matches!(context, ErrorContext::Command { .. }) {
        let backtrace = std::backtrace::Backtrace::force_capture();
        error!("{context} 失败: {error}\n{backtrace}");
        return;
    }

    bevy::ecs::error::panic(error, context);
}

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
            EditorFontPlugin,
            Wc4Plugin,
        ))
        .insert_resource(FallbackErrorHandler(handle_error))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(ClearColor(VIEWPORT_CLEAR_COLOR))
        .init_resource::<EditorState>()
        .init_resource::<SelectedTile>()
        .add_systems(Startup, ui::root_scene.spawn())
        .add_systems(
            Update,
            (
                map::draw_grid,
                ui::update_status.run_if(resource_changed::<EditorState>),
                ui::update_tile_details
                    .run_if(resource_changed::<SelectedTile>)
                    .run_if(ui::pointer_released),
                ui::write_fields_from_text,
                camera::smooth_zoom,
            ),
        )
        .run();
}

//! 编辑器全局字体。
//!
//! 字体资产只加载一次，由 2D 世界文字与 UI 文字共享。

use bevy::{
    prelude::*,
    text::{Font, FontSize, FontSource},
    ui::{Node, UiSystems},
};

/// 全局字体资产路径。
const FONT_PATH: &str = "SarasaUiSC-SemiBold.ttf";

/// 编辑器统一字体。
#[derive(Resource)]
pub(crate) struct EditorFont {
    /// 字体资产句柄。
    pub(crate) handle: Handle<Font>,
}

impl EditorFont {
    /// 构造使用该字体的文字样式。
    pub(crate) fn text_font(&self, font_size: impl Into<FontSize>) -> TextFont {
        TextFont::from_font_size(font_size).with_font(self.handle.clone())
    }
}

/// 标记 UI 文本使用自身字体，跳过全局字体统一。
#[derive(Component, Default)]
pub(crate) struct CustomFont;

/// 注册字体加载与 UI 字体统一。
pub(crate) struct EditorFontPlugin;

impl Plugin for EditorFontPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_editor_font).add_systems(
            PostUpdate,
            apply_editor_font_to_ui.after(UiSystems::Propagate).before(UiSystems::Content),
        );
    }
}

/// 加载编辑器字体。
fn load_editor_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(EditorFont {
        handle: asset_server.load(FONT_PATH),
    });
}

/// 将编辑器字体应用到所有 UI 文本。
///
/// Feathers 控件在场景中硬编码 Fira Sans，因此在文本布局前统一替换字体来源，
/// 保留控件自身的字号与字重。需要独立字体的文本可添加 [`CustomFont`]。
fn apply_editor_font_to_ui(
    font: Res<EditorFont>,
    mut texts: Query<&mut TextFont, (With<Node>, Without<CustomFont>, Changed<TextFont>)>,
) {
    let editor_font = FontSource::Handle(font.handle.clone());

    for mut text_font in &mut texts {
        if text_font.font != editor_font {
            text_font.font = editor_font.clone();
        }
    }
}

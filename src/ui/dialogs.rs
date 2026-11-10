//! 编辑器对话框与对话框动作。

use bevy::{
    feathers::{
        controls::{
            ButtonVariant, FeathersButton, FeathersDialog, FeathersDialogBody, FeathersDialogClose,
            FeathersDialogFooter, FeathersDialogHeader, FeathersFloatingDialog,
        },
        display::{caption, label_dim},
    },
    prelude::*,
    ui_widgets::{Activate, RequestClose},
};

/// 标记 About 对话框。
#[derive(Component, Clone, Default)]
pub(crate) struct AboutDialog;

/// 标记退出确认对话框。
#[derive(Component, Clone, Default)]
pub(crate) struct QuitDialog;

/// 打开 About 对话框。
pub(crate) fn show_about(
    _: On<Activate>,
    dialogs: Query<(), With<AboutDialog>>,
    mut commands: Commands,
) {
    if !dialogs.is_empty() {
        return;
    }

    commands.spawn_scene(bsn! {
        @FeathersFloatingDialog {
            @title: {"About".to_owned()},
            @width: px(280),
            @contents: bsn_list! {
                @caption("emodule: a minimal map editor shell.")
                --
                @label_dim("Built with bevy_ui and Bevy Feathers.")
            }
        }
        AboutDialog
    });
}

/// 打开退出确认对话框。
pub(crate) fn spawn_quit_dialog(
    _: On<Activate>,
    dialogs: Query<(), With<QuitDialog>>,
    mut commands: Commands,
) {
    if !dialogs.is_empty() {
        return;
    }

    commands.spawn_scene(bsn! {
        @FeathersDialog {
            @width: px(320),
            @contents: bsn_list! {
                @FeathersDialogHeader
                Children [
                    @caption("Quit")
                    --
                    @FeathersDialogClose
                ]
                --
                @FeathersDialogBody
                Children [
                    @caption("Quit the editor?")
                ]
                --
                @FeathersDialogFooter
                Children [
                    @FeathersButton {
                        @caption: bsn! { @caption("Cancel") },
                    }
                    on(request_close)
                    --
                    @FeathersButton {
                        @variant: ButtonVariant::Primary,
                        @caption: bsn! { @caption("Quit") },
                    }
                    on(quit_app)
                ]
            }
        }
        QuitDialog
        on(close_dialog)
    });
}

/// 请求关闭包含被激活按钮的对话框。
fn request_close(activate: On<Activate>, mut commands: Commands) {
    commands.trigger(RequestClose {
        source: activate.event_target(),
    });
}

/// 退出应用。
fn quit_app(_: On<Activate>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}

/// 关闭收到请求的对话框。
fn close_dialog(close: On<RequestClose>, mut commands: Commands) {
    commands.entity(close.event_target()).try_despawn();
}

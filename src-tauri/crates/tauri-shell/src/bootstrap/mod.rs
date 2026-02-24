mod invoke;
mod setup;

use crate::constants::{
    MAIN_WINDOW_LABEL, SHORTCUT_CLIPBOARD_WINDOW, SHORTCUT_CLIPBOARD_WINDOW_COMPACT,
    SHORTCUT_LAUNCHER_FALLBACK, SHORTCUT_LAUNCHER_PRIMARY,
};
use crate::platform::native_ui::shortcuts;
use app_launcher_app::launcher::index::stop_background_indexer;
use setup::{log_error_fallback, log_warn_fallback};
use tauri_plugin_global_shortcut::ShortcutState;

pub(crate) struct AppBootstrap;

impl AppBootstrap {
    pub(crate) fn run(context: tauri::Context<tauri::Wry>) {
        let clipboard_shortcut_id = SHORTCUT_CLIPBOARD_WINDOW
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .ok()
            .map(|shortcut| shortcut.id());
        let clipboard_compact_shortcut_id = SHORTCUT_CLIPBOARD_WINDOW_COMPACT
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .ok()
            .map(|shortcut| shortcut.id());

        let shortcut_builder = tauri_plugin_global_shortcut::Builder::new()
            .with_shortcuts([
                SHORTCUT_LAUNCHER_PRIMARY,
                SHORTCUT_LAUNCHER_FALLBACK,
                SHORTCUT_CLIPBOARD_WINDOW,
                SHORTCUT_CLIPBOARD_WINDOW_COMPACT,
            ])
            .or_else(|error| {
                log_warn_fallback(&format!(
                    "failed to register global shortcuts with fallback: {}",
                    error
                ));
                tauri_plugin_global_shortcut::Builder::new().with_shortcuts([
                    SHORTCUT_LAUNCHER_PRIMARY,
                    SHORTCUT_CLIPBOARD_WINDOW,
                    SHORTCUT_CLIPBOARD_WINDOW_COMPACT,
                ])
            })
            .or_else(|error| {
                log_warn_fallback(&format!(
                    "failed to register compact clipboard shortcut, fallback to basic clipboard shortcut: {}",
                    error
                ));
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([SHORTCUT_LAUNCHER_PRIMARY, SHORTCUT_CLIPBOARD_WINDOW])
            })
            .or_else(|error| {
                log_warn_fallback(&format!(
                    "failed to register clipboard shortcut, fallback to launcher only: {}",
                    error
                ));
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([SHORTCUT_LAUNCHER_PRIMARY])
            })
            .unwrap_or_else(|error| {
                log_error_fallback(&format!("failed to register global shortcuts: {}", error));
                tauri_plugin_global_shortcut::Builder::new()
            });

        let shortcut_plugin = shortcut_builder
            .with_handler(move |app, shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    shortcuts::handle_shortcut(
                        app,
                        shortcut,
                        clipboard_shortcut_id,
                        clipboard_compact_shortcut_id,
                    );
                }
            })
            .build();

        let builder = tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_clipboard::init())
            .plugin(shortcut_plugin)
            .setup(setup::setup)
            .on_window_event(|window, event| {
                if window.label() != MAIN_WINDOW_LABEL {
                    return;
                }

                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        tracing::warn!(
                            event = "window_hide_failed",
                            window = MAIN_WINDOW_LABEL,
                            error = error.to_string()
                        );
                    }
                }
            });

        let app = invoke::with_invoke_handler(builder)
            .build(context)
            .unwrap_or_else(|error| {
                log_error_fallback(&format!(
                    "error while building tauri application: {}",
                    error
                ));
                std::process::exit(1);
            });

        app.run(|_, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                stop_background_indexer();
            }
        });
    }
}

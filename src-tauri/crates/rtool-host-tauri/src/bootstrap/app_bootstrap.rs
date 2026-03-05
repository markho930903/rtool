use crate::bootstrap::app_setup;
use crate::bootstrap::command_registry;
use crate::constants::RUNTIME_WORKER_LAUNCHER;
use crate::constants::{
    MAIN_WINDOW_LABEL, SHORTCUT_CLIPBOARD_WINDOW, SHORTCUT_CLIPBOARD_WINDOW_COMPACT,
    SHORTCUT_LAUNCHER_FALLBACK, SHORTCUT_LAUNCHER_PRIMARY, SHORTCUT_SCREENSHOT_DEFAULT,
};
use crate::platform::native_ui::shortcuts;
use rtool_contracts::models::SettingsDto;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

pub(crate) struct AppBootstrap;

fn resolve_screenshot_shortcut(startup_settings: Option<&SettingsDto>) -> String {
    let loaded = startup_settings
        .map(|settings| settings.screenshot.shortcut.clone())
        .unwrap_or_else(|| SHORTCUT_SCREENSHOT_DEFAULT.to_string());

    let normalized = loaded.trim().to_string();
    if normalized.is_empty() {
        return SHORTCUT_SCREENSHOT_DEFAULT.to_string();
    }
    if normalized
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .is_err()
    {
        app_setup::log_warn_fallback(&format!(
            "invalid screenshot shortcut '{}', fallback to default",
            normalized
        ));
        return SHORTCUT_SCREENSHOT_DEFAULT.to_string();
    }
    normalized
}

impl AppBootstrap {
    pub(crate) fn run(context: tauri::Context<tauri::Wry>) {
        let startup_settings = app_setup::try_load_startup_settings();
        let screenshot_shortcut = resolve_screenshot_shortcut(startup_settings.as_ref());
        let clipboard_shortcut_id = SHORTCUT_CLIPBOARD_WINDOW
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .ok()
            .map(|shortcut| shortcut.id());
        let clipboard_compact_shortcut_id = SHORTCUT_CLIPBOARD_WINDOW_COMPACT
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .ok()
            .map(|shortcut| shortcut.id());
        let screenshot_shortcut_id = screenshot_shortcut
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .ok()
            .map(|shortcut| shortcut.id());

        let shortcut_builder = tauri_plugin_global_shortcut::Builder::new()
            .with_shortcuts([
                SHORTCUT_LAUNCHER_PRIMARY,
                SHORTCUT_LAUNCHER_FALLBACK,
                SHORTCUT_CLIPBOARD_WINDOW,
                SHORTCUT_CLIPBOARD_WINDOW_COMPACT,
                screenshot_shortcut.as_str(),
            ])
            .or_else(|error| {
                app_setup::log_warn_fallback(&format!(
                    "failed to register global shortcuts with fallback: {}",
                    error
                ));
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([
                        SHORTCUT_LAUNCHER_PRIMARY,
                        SHORTCUT_CLIPBOARD_WINDOW,
                        SHORTCUT_CLIPBOARD_WINDOW_COMPACT,
                        screenshot_shortcut.as_str(),
                    ])
            })
            .or_else(|error| {
                app_setup::log_warn_fallback(&format!(
                    "failed to register compact clipboard/screenshot shortcut, fallback to basic clipboard shortcut: {}",
                    error
                ));
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([SHORTCUT_LAUNCHER_PRIMARY, SHORTCUT_CLIPBOARD_WINDOW])
            })
            .or_else(|error| {
                app_setup::log_warn_fallback(&format!(
                    "failed to register clipboard shortcut, fallback to launcher only: {}",
                    error
                ));
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([SHORTCUT_LAUNCHER_PRIMARY])
            })
            .unwrap_or_else(|error| {
                app_setup::log_error_fallback(&format!(
                    "failed to register global shortcuts: {}",
                    error
                ));
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
            .plugin(tauri_plugin_dialog::init())
            .plugin(shortcut_plugin)
            .setup(move |app| {
                app_setup::setup(app, startup_settings.clone(), screenshot_shortcut_id)
            })
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

        let app = command_registry::with_invoke_handler(builder)
            .build(context)
            .unwrap_or_else(|error| {
                app_setup::log_error_fallback(&format!(
                    "error while building tauri application: {}",
                    error
                ));
                std::process::exit(1);
            });

        app.run(|app_handle, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) && let Some(state) = app_handle.try_state::<crate::app::state::AppState>()
            {
                state.app_services.shutdown();
                state
                    .runtime_orchestrator
                    .mark_stopped(RUNTIME_WORKER_LAUNCHER);
            }
        });
    }
}

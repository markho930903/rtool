use crate::constants::{
    CLIPBOARD_WINDOW_LABEL, LAUNCHER_OPENED_EVENT, LAUNCHER_WINDOW_LABEL, MAIN_WINDOW_LABEL,
};
use crate::core::i18n::t;
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub(crate) fn refresh_window_titles<R: Runtime>(app: &AppHandle<R>, locale: &str) {
    for (label, title_key) in [
        (MAIN_WINDOW_LABEL, "window.main.title"),
        (CLIPBOARD_WINDOW_LABEL, "window.clipboard.title"),
        (LAUNCHER_WINDOW_LABEL, "window.launcher.title"),
    ] {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };

        let title = t(locale, title_key);
        if let Err(error) = window.set_title(&title) {
            tracing::warn!(
                event = "window_title_update_failed",
                window = label,
                error = error.to_string()
            );
        }
    }
}

pub(crate) fn toggle_launcher_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LAUNCHER_WINDOW_LABEL) {
        if window.is_visible().unwrap_or(false) {
            if let Err(error) = window.hide() {
                tracing::warn!(
                    event = "window_hide_failed",
                    window = LAUNCHER_WINDOW_LABEL,
                    error = error.to_string()
                );
            }
            return;
        }

        if let Err(error) = window.show() {
            tracing::warn!(
                event = "window_show_failed",
                window = LAUNCHER_WINDOW_LABEL,
                error = error.to_string()
            );
        }
        if let Err(error) = window.set_focus() {
            tracing::warn!(
                event = "window_focus_failed",
                window = LAUNCHER_WINDOW_LABEL,
                error = error.to_string()
            );
        }
        if let Err(error) = app.emit(LAUNCHER_OPENED_EVENT, ()) {
            tracing::warn!(
                event = "window_event_emit_failed",
                event_name = LAUNCHER_OPENED_EVENT,
                error = error.to_string()
            );
        }
    }
}

pub(crate) fn focus_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        tracing::warn!(
            event = "window_focus_failed",
            window = MAIN_WINDOW_LABEL,
            error = "window_not_found"
        );
        return;
    };

    if let Err(error) = window.show() {
        tracing::warn!(
            event = "window_show_failed",
            window = MAIN_WINDOW_LABEL,
            error = error.to_string()
        );
    }
    if let Err(error) = window.set_focus() {
        tracing::warn!(
            event = "window_focus_failed",
            window = MAIN_WINDOW_LABEL,
            error = error.to_string()
        );
    }
}

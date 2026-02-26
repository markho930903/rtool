use crate::app::state::AppState;
use crate::constants::{CLIPBOARD_WINDOW_LABEL, CLIPBOARD_WINDOW_OPENED_EVENT};
use crate::platform::native_ui::clipboard_window::{
    apply_clipboard_window_mode, set_clipboard_window_compact_state,
};
use crate::platform::native_ui::windows::toggle_launcher_window;
use foundation::models::ClipboardWindowOpenedPayload;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::Shortcut;

fn emit_clipboard_window_opened(app: &AppHandle, compact: bool) {
    if let Err(error) = app.emit(
        CLIPBOARD_WINDOW_OPENED_EVENT,
        ClipboardWindowOpenedPayload { compact },
    ) {
        tracing::warn!(
            event = "window_event_emit_failed",
            event_name = CLIPBOARD_WINDOW_OPENED_EVENT,
            compact,
            error = error.to_string()
        );
    }
}

fn handle_clipboard_window_shortcut(app: &AppHandle, requested_compact: bool) {
    let Some(window) = app.get_webview_window(CLIPBOARD_WINDOW_LABEL) else {
        return;
    };

    let is_visible = window.is_visible().unwrap_or(false);
    if !is_visible {
        tracing::info!(
            event = "clipboard_window_shortcut_action",
            action = "show",
            requested_compact = requested_compact
        );
        if let Err(error) = window.show() {
            tracing::warn!(
                event = "window_show_failed",
                window = CLIPBOARD_WINDOW_LABEL,
                error = error.to_string()
            );
        }
        if let Err(error) = window.set_focus() {
            tracing::warn!(
                event = "window_focus_failed",
                window = CLIPBOARD_WINDOW_LABEL,
                error = error.to_string()
            );
        }
        if let Err(error) = apply_clipboard_window_mode(app, requested_compact, "shortcut_show") {
            tracing::warn!(
                event = "clipboard_window_mode_apply_failed",
                action = "show",
                requested_compact = requested_compact,
                error = error.to_string()
            );
        }
        set_clipboard_window_compact_state(app, requested_compact);
        emit_clipboard_window_opened(app, requested_compact);
        return;
    }

    let should_hide = app
        .try_state::<AppState>()
        .map(|state| state.clipboard_window_compact() == requested_compact)
        .unwrap_or(false);
    if should_hide {
        tracing::info!(
            event = "clipboard_window_shortcut_action",
            action = "hide",
            requested_compact = requested_compact
        );
        if let Err(error) = window.hide() {
            tracing::warn!(
                event = "window_hide_failed",
                window = CLIPBOARD_WINDOW_LABEL,
                error = error.to_string()
            );
        }
        return;
    }

    tracing::info!(
        event = "clipboard_window_shortcut_action",
        action = "switch",
        requested_compact = requested_compact
    );
    if let Err(error) = window.set_focus() {
        tracing::warn!(
            event = "window_focus_failed",
            window = CLIPBOARD_WINDOW_LABEL,
            error = error.to_string()
        );
    }
    if let Err(error) = apply_clipboard_window_mode(app, requested_compact, "shortcut_switch") {
        tracing::warn!(
            event = "clipboard_window_mode_apply_failed",
            action = "switch",
            requested_compact = requested_compact,
            error = error.to_string()
        );
    }
    set_clipboard_window_compact_state(app, requested_compact);
    emit_clipboard_window_opened(app, requested_compact);
}

pub(crate) fn handle_shortcut(
    app: &AppHandle,
    shortcut: &Shortcut,
    clipboard_shortcut_id: Option<u32>,
    clipboard_compact_shortcut_id: Option<u32>,
) {
    if Some(shortcut.id()) == clipboard_compact_shortcut_id {
        handle_clipboard_window_shortcut(app, true);
        return;
    }

    if Some(shortcut.id()) == clipboard_shortcut_id {
        handle_clipboard_window_shortcut(app, false);
        return;
    }

    toggle_launcher_window(app);
}

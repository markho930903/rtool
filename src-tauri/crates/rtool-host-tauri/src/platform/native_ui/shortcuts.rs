use crate::app::state::AppState;
use crate::constants::{
    CLIPBOARD_WINDOW_LABEL, CLIPBOARD_WINDOW_OPENED_EVENT, SCREENSHOT_WINDOW_LABEL,
    SCREENSHOT_WINDOW_OPENED_EVENT, SHORTCUT_SCREENSHOT_DEFAULT,
};
use crate::platform::native_ui::clipboard_window::{
    apply_clipboard_window_mode, set_clipboard_window_compact_state,
};
use crate::platform::native_ui::window_factory::ensure_webview_window;
use crate::platform::native_ui::windows::toggle_launcher_window;
use rtool_app::ScreenshotApplicationService;
use rtool_contracts::models::{ClipboardWindowOpenedPayload, ScreenshotWindowOpenedPayload};
use rtool_contracts::{AppError, AppResult};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

fn current_screenshot_shortcut_id(app: &AppHandle) -> Option<u32> {
    app.try_state::<AppState>()
        .map(|state| state.screenshot_shortcut_id())
        .unwrap_or(None)
}

fn set_screenshot_shortcut_id(app: &AppHandle, shortcut_id: Option<u32>) {
    if let Some(state) = app.try_state::<AppState>() {
        state.set_screenshot_shortcut_id(shortcut_id);
        return;
    }

    tracing::warn!(
        event = "screenshot_shortcut_state_unavailable",
        shortcut_id,
        detail = "app_state_unavailable"
    );
}

pub(crate) fn normalize_screenshot_shortcut(value: &str) -> AppResult<String> {
    let normalized = value.trim();
    let candidate = if normalized.is_empty() {
        SHORTCUT_SCREENSHOT_DEFAULT
    } else {
        normalized
    };
    candidate.parse::<Shortcut>().map_err(|error| {
        AppError::new("screenshot_shortcut_invalid", "截图快捷键格式无效")
            .with_context("shortcut", candidate.to_string())
            .with_source(error)
    })?;
    Ok(candidate.to_string())
}

pub(crate) fn rebind_screenshot_shortcut(
    app: &AppHandle,
    previous_shortcut: &str,
    next_shortcut: &str,
) -> AppResult<()> {
    let next_normalized = normalize_screenshot_shortcut(next_shortcut)?;
    let previous_normalized = previous_shortcut.trim().to_string();
    let previous = previous_normalized.parse::<Shortcut>().ok();
    let next = next_normalized.parse::<Shortcut>().map_err(|error| {
        AppError::new("screenshot_shortcut_invalid", "截图快捷键格式无效")
            .with_context("shortcut", next_normalized.clone())
            .with_source(error)
    })?;

    if previous.as_ref().map(Shortcut::id) == Some(next.id()) {
        set_screenshot_shortcut_id(app, Some(next.id()));
        return Ok(());
    }

    if app.global_shortcut().is_registered(next) {
        return Err(
            AppError::new("screenshot_shortcut_conflict", "截图快捷键与现有快捷键冲突")
                .with_context("shortcut", next_normalized),
        );
    }

    app.global_shortcut().register(next).map_err(|error| {
        AppError::new("screenshot_shortcut_register_failed", "注册截图快捷键失败")
            .with_context("shortcut", next_normalized.clone())
            .with_source(error)
    })?;

    if let Some(previous_shortcut) = previous
        && app.global_shortcut().is_registered(previous_shortcut)
        && let Err(error) = app.global_shortcut().unregister(previous_shortcut)
    {
        tracing::warn!(
            event = "screenshot_shortcut_unregister_failed",
            shortcut = previous_normalized,
            error = error.to_string()
        );
    }

    set_screenshot_shortcut_id(app, Some(next.id()));
    Ok(())
}

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
    let window = match ensure_webview_window(app, CLIPBOARD_WINDOW_LABEL) {
        Ok(window) => window,
        Err(error) => {
            tracing::warn!(
                event = "window_create_failed",
                window = CLIPBOARD_WINDOW_LABEL,
                code = error.code.as_str(),
                message = error.message.as_str()
            );
            return;
        }
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

fn emit_screenshot_window_opened(app: &AppHandle, payload: ScreenshotWindowOpenedPayload) {
    if let Err(error) = app.emit(SCREENSHOT_WINDOW_OPENED_EVENT, payload) {
        tracing::warn!(
            event = "window_event_emit_failed",
            event_name = SCREENSHOT_WINDOW_OPENED_EVENT,
            error = error.to_string()
        );
    }
}

fn handle_screenshot_shortcut(app: &AppHandle) {
    let session = match ScreenshotApplicationService.start_session(None) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(
                event = "screenshot_session_start_failed",
                error_code = error.code.as_str(),
                error_detail = error.causes.first().map(String::as_str).unwrap_or_default()
            );
            return;
        }
    };

    let window = match ensure_webview_window(app, SCREENSHOT_WINDOW_LABEL) {
        Ok(window) => window,
        Err(error) => {
            tracing::warn!(
                event = "window_create_failed",
                window = SCREENSHOT_WINDOW_LABEL,
                code = error.code.as_str(),
                message = error.message.as_str()
            );
            return;
        }
    };

    if let Some(display) = session
        .displays
        .iter()
        .find(|item| item.id == session.active_display_id)
        .or_else(|| session.displays.first())
    {
        if let Err(error) = window.set_position(LogicalPosition::new(
            f64::from(display.x),
            f64::from(display.y),
        )) {
            tracing::warn!(
                event = "window_set_position_failed",
                window = SCREENSHOT_WINDOW_LABEL,
                error = error.to_string()
            );
        }
        if let Err(error) = window.set_size(LogicalSize::new(
            f64::from(display.width),
            f64::from(display.height),
        )) {
            tracing::warn!(
                event = "window_set_size_failed",
                window = SCREENSHOT_WINDOW_LABEL,
                error = error.to_string()
            );
        }
    }

    if let Err(error) = window.show() {
        tracing::warn!(
            event = "window_show_failed",
            window = SCREENSHOT_WINDOW_LABEL,
            error = error.to_string()
        );
    }
    if let Err(error) = window.set_focus() {
        tracing::warn!(
            event = "window_focus_failed",
            window = SCREENSHOT_WINDOW_LABEL,
            error = error.to_string()
        );
    }

    emit_screenshot_window_opened(app, ScreenshotWindowOpenedPayload { session });
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

    if Some(shortcut.id()) == current_screenshot_shortcut_id(app) {
        handle_screenshot_shortcut(app);
        return;
    }

    toggle_launcher_window(app);
}

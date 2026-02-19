mod app;
mod bootstrap;
mod clipboard_watcher;
mod commands;
mod constants;
mod core;
mod infrastructure;
mod native_ui;

use tauri::{AppHandle, Runtime};

pub(crate) fn apply_locale_to_native_ui<R: Runtime>(app: &AppHandle<R>, locale: &str) {
    native_ui::apply_locale_to_native_ui(app, locale);
}

pub(crate) fn apply_clipboard_window_mode(
    app: &tauri::AppHandle,
    compact: bool,
    source: &str,
) -> core::AppResult<core::models::ClipboardWindowModeAppliedDto> {
    native_ui::clipboard_window::apply_clipboard_window_mode(app, compact, source)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    bootstrap::AppBootstrap::run();
}

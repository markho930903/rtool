pub mod app;
mod bootstrap;
mod command_runtime;
mod constants;
mod features;
mod host;
mod platform;

use foundation::{AppResult, models};
use tauri::{AppHandle, Runtime};

pub(crate) fn apply_locale_to_native_ui<R: Runtime>(app: &AppHandle<R>, locale: &str) {
    platform::native_ui::apply_locale_to_native_ui(app, locale);
}

pub(crate) fn apply_clipboard_window_mode(
    app: &tauri::AppHandle,
    compact: bool,
    source: &str,
) -> AppResult<models::ClipboardWindowModeAppliedDto> {
    platform::native_ui::clipboard_window::apply_clipboard_window_mode(app, compact, source)
}

pub fn run(context: tauri::Context<tauri::Wry>) {
    bootstrap::AppBootstrap::run(context);
}

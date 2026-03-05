pub(crate) mod clipboard_window;
pub(crate) mod shortcuts;
pub(crate) mod tray;
pub(crate) mod window_chrome;
pub(crate) mod window_factory;
pub(crate) mod windows;

use tauri::{AppHandle, Runtime};

pub(crate) fn apply_locale_to_native_ui<R: Runtime>(app: &AppHandle<R>, locale: &str) {
    tray::refresh_tray_menu(app, locale);
    windows::refresh_window_titles(app, locale);
}

pub(crate) fn apply_window_chrome<R: Runtime>(
    app: &AppHandle<R>,
    transparent_window_background: bool,
) {
    window_chrome::apply_window_chrome(app, transparent_window_background);
}

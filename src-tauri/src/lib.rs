mod app;
mod commands;
mod core;
mod infrastructure;

use active_win_pos_rs::get_active_window;
use app::clipboard_service::ClipboardService;
use app::launcher_service::execute_launcher_action;
use app::state::AppState;
use app::transfer_service::TransferService;
use core::i18n::{
    APP_LOCALE_PREFERENCE_KEY, AppLocaleState, init_i18n_catalog, normalize_locale_preference,
    resolve_locale, t,
};
use core::models::{
    ClipboardSyncPayload, ClipboardWindowModeAppliedDto, ClipboardWindowOpenedPayload,
    LauncherActionDto,
};
use image::ImageReader;
use std::error::Error;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Listener, LogicalSize, Manager, PhysicalPosition, Runtime};
use tauri_plugin_global_shortcut::ShortcutState;

const CLIPBOARD_SYNC_EVENT: &str = "rtool://clipboard/sync";
const SHORTCUT_LAUNCHER_PRIMARY: &str = "CommandOrControl+K";
const SHORTCUT_LAUNCHER_FALLBACK: &str = "Alt+Space";
const SHORTCUT_CLIPBOARD_WINDOW: &str = "Alt+V";
const SHORTCUT_CLIPBOARD_WINDOW_COMPACT: &str = "Alt+Shift+V";
const CLIPBOARD_WINDOW_LABEL: &str = "clipboard_history";
const CLIPBOARD_COMPACT_WIDTH_LOGICAL: f64 = 560.0;
const CLIPBOARD_REGULAR_WIDTH_LOGICAL: f64 = 960.0;
const CLIPBOARD_MIN_HEIGHT_LOGICAL: f64 = 520.0;
const TRAY_ICON_ID: &str = "main-tray";
const TRAY_MENU_ID_DASHBOARD: &str = "tray.dashboard";
const TRAY_MENU_ID_TOOLS: &str = "tray.tools";
const TRAY_MENU_ID_CLIPBOARD: &str = "tray.clipboard";
const TRAY_MENU_ID_QUIT: &str = "tray.quit";

fn read_initial_locale_state(
    db_pool: &infrastructure::db::DbPool,
) -> Result<AppLocaleState, Box<dyn Error>> {
    let preference = infrastructure::db::get_app_setting(db_pool, APP_LOCALE_PREFERENCE_KEY)?
        .as_deref()
        .and_then(normalize_locale_preference)
        .unwrap_or_else(|| "system".to_string());
    Ok(AppLocaleState::new(
        preference.clone(),
        resolve_locale(&preference),
    ))
}

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>, locale: &str) -> tauri::Result<Menu<R>> {
    let dashboard_label = t(locale, "tray.dashboard");
    let dashboard_item = MenuItem::with_id(
        app,
        TRAY_MENU_ID_DASHBOARD,
        &dashboard_label,
        true,
        None::<&str>,
    )?;
    let tools_label = t(locale, "tray.tools");
    let tools_item = MenuItem::with_id(app, TRAY_MENU_ID_TOOLS, &tools_label, true, None::<&str>)?;
    let clipboard_label = t(locale, "tray.clipboard");
    let clipboard_item = MenuItem::with_id(
        app,
        TRAY_MENU_ID_CLIPBOARD,
        &clipboard_label,
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_label = t(locale, "tray.quit");
    let quit_item = MenuItem::with_id(app, TRAY_MENU_ID_QUIT, &quit_label, true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &dashboard_item,
            &tools_item,
            &clipboard_item,
            &separator,
            &quit_item,
        ],
    )
}

fn refresh_window_titles<R: Runtime>(app: &AppHandle<R>, locale: &str) {
    for (label, title_key) in [
        ("main", "window.main.title"),
        ("clipboard_history", "window.clipboard.title"),
        ("launcher", "window.launcher.title"),
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

fn refresh_tray_menu<R: Runtime>(app: &AppHandle<R>, locale: &str) {
    let Some(tray) = app.tray_by_id(TRAY_ICON_ID) else {
        return;
    };

    match build_tray_menu(app, locale) {
        Ok(menu) => {
            if let Err(error) = tray.set_menu(Some(menu)) {
                tracing::warn!(event = "tray_menu_update_failed", error = error.to_string());
            }
        }
        Err(error) => {
            tracing::warn!(event = "tray_menu_build_failed", error = error.to_string());
        }
    }

    if let Err(error) = tray.set_tooltip(Some(t(locale, "tray.tooltip"))) {
        tracing::warn!(
            event = "tray_tooltip_update_failed",
            error = error.to_string()
        );
    }

    if let Err(error) = tray.set_title(Option::<&str>::None) {
        tracing::warn!(event = "tray_title_clear_failed", error = error.to_string());
    }
}

pub(crate) fn apply_locale_to_native_ui<R: Runtime>(app: &AppHandle<R>, locale: &str) {
    refresh_tray_menu(app, locale);
    refresh_window_titles(app, locale);
}

fn log_warn_fallback(message: &str) {
    if tracing::dispatcher::has_been_set() {
        tracing::warn!(event = "bootstrap_warning", message = message);
        return;
    }

    eprintln!("{message}");
}

fn log_error_fallback(message: &str) {
    if tracing::dispatcher::has_been_set() {
        tracing::error!(event = "bootstrap_error", message = message);
        return;
    }

    eprintln!("{message}");
}

fn toggle_launcher_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("launcher") {
        if window.is_visible().unwrap_or(false) {
            if let Err(error) = window.hide() {
                tracing::warn!(
                    event = "window_hide_failed",
                    window = "launcher",
                    error = error.to_string()
                );
            }
            return;
        }

        if let Err(error) = window.show() {
            tracing::warn!(
                event = "window_show_failed",
                window = "launcher",
                error = error.to_string()
            );
        }
        if let Err(error) = window.set_focus() {
            tracing::warn!(
                event = "window_focus_failed",
                window = "launcher",
                error = error.to_string()
            );
        }
        if let Err(error) = app.emit("rtool://launcher/opened", ()) {
            tracing::warn!(
                event = "window_event_emit_failed",
                event_name = "rtool://launcher/opened",
                error = error.to_string()
            );
        }
    }
}

fn emit_clipboard_window_opened(app: &tauri::AppHandle, compact: bool) {
    if let Err(error) = app.emit(
        "rtool://clipboard-window/opened",
        ClipboardWindowOpenedPayload { compact },
    ) {
        tracing::warn!(
            event = "window_event_emit_failed",
            event_name = "rtool://clipboard-window/opened",
            compact,
            error = error.to_string()
        );
    }
}

fn set_clipboard_window_compact_state(app: &tauri::AppHandle, compact: bool) {
    if let Some(state) = app.try_state::<AppState>() {
        state.set_clipboard_window_compact(compact);
    }
}

fn clamp_clipboard_window_position(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    monitor: &tauri::Monitor,
) -> (i32, i32) {
    let work_area = monitor.work_area();
    let min_x = work_area.position.x;
    let min_y = work_area.position.y;
    let max_x = min_x + work_area.size.width.saturating_sub(width) as i32;
    let max_y = min_y + work_area.size.height.saturating_sub(height) as i32;
    let clamped_x = x.clamp(min_x, max_x.max(min_x));
    let clamped_y = y.clamp(min_y, max_y.max(min_y));
    (clamped_x, clamped_y)
}

pub(crate) fn apply_clipboard_window_mode(
    app: &tauri::AppHandle,
    compact: bool,
    source: &str,
) -> Result<ClipboardWindowModeAppliedDto, String> {
    let window = app
        .get_webview_window(CLIPBOARD_WINDOW_LABEL)
        .ok_or_else(|| "window_not_found:clipboard_history".to_string())?;

    let scale_factor = window
        .scale_factor()
        .map_err(|error| format!("scale_factor_failed:{error}"))?
        .max(0.1);
    let before_size = window
        .outer_size()
        .map_err(|error| format!("outer_size_failed:{error}"))?;
    let before_position = window
        .outer_position()
        .map_err(|error| format!("outer_position_failed:{error}"))?;

    let target_width_logical = if compact {
        CLIPBOARD_COMPACT_WIDTH_LOGICAL
    } else {
        CLIPBOARD_REGULAR_WIDTH_LOGICAL
    };
    let target_height_logical =
        (before_size.height as f64 / scale_factor).max(CLIPBOARD_MIN_HEIGHT_LOGICAL);
    window
        .set_size(LogicalSize::new(
            target_width_logical,
            target_height_logical,
        ))
        .map_err(|error| format!("set_size_failed:{error}"))?;

    let target_width_px = (target_width_logical * scale_factor).round().max(1.0) as u32;
    let target_height_px = (target_height_logical * scale_factor).round().max(1.0) as u32;
    let mut next_x = before_position.x;
    let mut next_y = before_position.y;
    match window.current_monitor() {
        Ok(Some(monitor)) => {
            let (x, y) = clamp_clipboard_window_position(
                next_x,
                next_y,
                target_width_px,
                target_height_px,
                &monitor,
            );
            next_x = x;
            next_y = y;
        }
        Ok(None) => {
            tracing::debug!(
                event = "clipboard_window_monitor_missing",
                source = source,
                compact = compact
            );
        }
        Err(error) => {
            tracing::warn!(
                event = "clipboard_window_monitor_read_failed",
                source = source,
                compact = compact,
                error = error.to_string()
            );
        }
    }
    if (next_x, next_y) != (before_position.x, before_position.y) {
        window
            .set_position(PhysicalPosition::new(next_x, next_y))
            .map_err(|error| format!("set_position_failed:{error}"))?;
    }

    let after_size = match window.outer_size() {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(
                event = "clipboard_window_after_size_read_failed",
                source = source,
                compact = compact,
                error = error.to_string()
            );
            before_size
        }
    };
    let applied_width_logical = after_size.width as f64 / scale_factor;
    let applied_height_logical = after_size.height as f64 / scale_factor;

    tracing::info!(
        event = "clipboard_window_mode_applied",
        source = source,
        compact = compact,
        scale_factor = scale_factor,
        before_width_px = before_size.width,
        before_height_px = before_size.height,
        target_width_logical = target_width_logical,
        target_height_logical = target_height_logical,
        after_width_px = after_size.width,
        after_height_px = after_size.height,
        position_x = next_x,
        position_y = next_y
    );

    Ok(ClipboardWindowModeAppliedDto {
        compact,
        applied_width_logical,
        applied_height_logical,
        scale_factor,
    })
}

fn handle_clipboard_window_shortcut(app: &tauri::AppHandle, requested_compact: bool) {
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
                error = error
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
            error = error
        );
    }
    set_clipboard_window_compact_state(app, requested_compact);
    emit_clipboard_window_opened(app, requested_compact);
}

fn handle_shortcut(
    app: &tauri::AppHandle,
    shortcut: &tauri_plugin_global_shortcut::Shortcut,
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

fn run_tray_action(app: &tauri::AppHandle, action: LauncherActionDto, action_name: &str) {
    let result = execute_launcher_action(app, &action);
    if !result.ok {
        tracing::warn!(
            event = "tray_action_failed",
            action = action_name,
            message = result.message
        );
    }
}

fn handle_tray_menu(app: &tauri::AppHandle, menu_id: &str) {
    match menu_id {
        TRAY_MENU_ID_DASHBOARD => run_tray_action(
            app,
            LauncherActionDto::OpenBuiltinRoute {
                route: "/".to_string(),
            },
            "dashboard",
        ),
        TRAY_MENU_ID_TOOLS => run_tray_action(
            app,
            LauncherActionDto::OpenBuiltinRoute {
                route: "/tools".to_string(),
            },
            "tools",
        ),
        TRAY_MENU_ID_CLIPBOARD => run_tray_action(
            app,
            LauncherActionDto::OpenBuiltinWindow {
                window_label: "clipboard_history".to_string(),
            },
            "clipboard",
        ),
        TRAY_MENU_ID_QUIT => app.exit(0),
        _ => {}
    }
}

fn handle_tray_icon_event(app: &tauri::AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        handle_tray_menu(app, TRAY_MENU_ID_DASHBOARD);
    }
}

fn init_database(
    app: &tauri::App,
) -> Result<(PathBuf, infrastructure::db::DbPool), Box<dyn Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    let db_path = app_data_dir.join("rtool.db");
    infrastructure::db::init_db(&db_path)?;
    let db_pool = infrastructure::db::new_db_pool(&db_path)?;
    Ok((db_path, db_pool))
}

const CLIPBOARD_PLUGIN_UPDATE_EVENT: &str = "plugin:clipboard://clipboard-monitor/update";

fn current_source_app() -> Option<String> {
    let active_window = get_active_window().ok()?;
    let app_name = active_window.app_name.trim();
    if app_name.is_empty() {
        return None;
    }

    Some(app_name.to_string())
}

fn build_image_signature(width: usize, height: usize, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(width as u64).to_le_bytes());
    hasher.update(&(height as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

struct ClipboardProcessor<R: Runtime> {
    app_handle: AppHandle<R>,
    service: ClipboardService,
    preview_dir: Option<PathBuf>,
    last_seen: String,
    last_image_signature: String,
}

impl<R: Runtime> ClipboardProcessor<R> {
    fn new(app_handle: AppHandle<R>, service: ClipboardService) -> Self {
        let preview_dir = match app_handle.path().app_data_dir() {
            Ok(value) => Some(value.join("clipboard_previews")),
            Err(error) => {
                tracing::warn!(
                    event = "clipboard_preview_dir_resolve_failed",
                    error = error.to_string()
                );
                None
            }
        };

        Self {
            app_handle,
            service,
            preview_dir,
            last_seen: String::new(),
            last_image_signature: String::new(),
        }
    }

    fn emit_sync(&self, payload: ClipboardSyncPayload) {
        if let Err(error) = self.app_handle.emit(CLIPBOARD_SYNC_EVENT, payload) {
            tracing::warn!(
                event = "clipboard_event_emit_failed",
                event_name = CLIPBOARD_SYNC_EVENT,
                error = error.to_string()
            );
        }
    }

    fn handle_text(&mut self, text: String, source_app: Option<String>) -> bool {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() || trimmed == self.last_seen {
            return true;
        }

        self.last_seen = trimmed.clone();
        self.last_image_signature.clear();

        match self.service.save_text(trimmed, source_app) {
            Ok(result) => {
                self.emit_sync(ClipboardSyncPayload {
                    upsert: vec![result.item],
                    removed_ids: result.removed_ids,
                    clear_all: false,
                    reason: Some("watcher_save_text".to_string()),
                });
            }
            Err(error) => {
                tracing::warn!(
                    event = "clipboard_text_save_failed",
                    error_code = error.code.as_str(),
                    error_detail = error
                        .detail
                        .as_deref()
                        .map(infrastructure::logging::sanitize_for_log)
                        .unwrap_or_default()
                );
            }
        }
        true
    }

    fn handle_files(&mut self, files_uris: Vec<String>, source_app: Option<String>) -> bool {
        let normalized_files: Vec<String> = files_uris
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if normalized_files.is_empty() {
            return false;
        }

        let serialized = normalized_files.join("\n");
        if serialized == self.last_seen {
            return true;
        }

        self.last_seen = serialized.clone();
        self.last_image_signature.clear();

        match self.service.save_text(serialized, source_app) {
            Ok(result) => {
                self.emit_sync(ClipboardSyncPayload {
                    upsert: vec![result.item],
                    removed_ids: result.removed_ids,
                    clear_all: false,
                    reason: Some("watcher_save_files".to_string()),
                });
            }
            Err(error) => {
                tracing::warn!(
                    event = "clipboard_files_save_failed",
                    error_code = error.code.as_str(),
                    error_detail = error
                        .detail
                        .as_deref()
                        .map(infrastructure::logging::sanitize_for_log)
                        .unwrap_or_default()
                );
            }
        }
        true
    }

    fn handle_image(&mut self, png_bytes: &[u8], source_app: Option<String>) {
        let (width_u32, height_u32) =
            if let Some(dimensions) = read_image_dimensions_from_header(png_bytes) {
                dimensions
            } else {
                match image::load_from_memory(png_bytes) {
                    Ok(decoded) => (decoded.width(), decoded.height()),
                    Err(error) => {
                        tracing::warn!(
                            event = "clipboard_image_decode_failed",
                            error = error.to_string()
                        );
                        return;
                    }
                }
            };
        let width = width_u32 as usize;
        let height = height_u32 as usize;
        let signature = build_image_signature(width, height, png_bytes);
        if signature == self.last_image_signature {
            return;
        }

        if let Err(error) = self.service.ensure_disk_space_for_new_item() {
            tracing::warn!(
                event = "clipboard_image_skip_low_disk",
                error_code = error.code.as_str(),
                error_detail = error
                    .detail
                    .as_deref()
                    .map(infrastructure::logging::sanitize_for_log)
                    .unwrap_or_default()
            );
            return;
        }

        self.last_image_signature = signature.clone();
        self.last_seen.clear();

        let preview_path = self.preview_dir.as_ref().and_then(|dir| {
            match save_clipboard_image_preview(dir, &signature, png_bytes) {
                Ok(path) => Some(path),
                Err(error) => {
                    tracing::warn!(
                        event = "clipboard_preview_save_failed",
                        signature = %signature,
                        error = error.to_string()
                    );
                    None
                }
            }
        });

        let item = infrastructure::clipboard::build_image_clipboard_item(
            width,
            height,
            &signature,
            preview_path,
            None,
            source_app,
        );

        match self.service.save_item(item) {
            Ok(result) => {
                self.emit_sync(ClipboardSyncPayload {
                    upsert: vec![result.item],
                    removed_ids: result.removed_ids,
                    clear_all: false,
                    reason: Some("watcher_save_image".to_string()),
                });
            }
            Err(error) => {
                tracing::warn!(
                    event = "clipboard_image_save_failed",
                    error_code = error.code.as_str(),
                    error_detail = error
                        .detail
                        .as_deref()
                        .map(infrastructure::logging::sanitize_for_log)
                        .unwrap_or_default()
                );
            }
        }
    }

    fn handle_update_event(&mut self) {
        let source_app = current_source_app();
        let files_uris_result = {
            let clipboard = self.app_handle.state::<tauri_plugin_clipboard::Clipboard>();
            clipboard.read_files_uris()
        };
        if let Ok(files_uris) = files_uris_result {
            if self.handle_files(files_uris, source_app.clone()) {
                return;
            }
        }

        let image_binary_result = {
            let clipboard = self.app_handle.state::<tauri_plugin_clipboard::Clipboard>();
            clipboard.read_image_binary()
        };
        if let Ok(image_binary) = image_binary_result {
            self.handle_image(&image_binary, source_app.clone());
            return;
        }

        let text_result = {
            let clipboard = self.app_handle.state::<tauri_plugin_clipboard::Clipboard>();
            clipboard.read_text()
        };
        if let Ok(text) = text_result {
            self.handle_text(text, source_app);
        }
    }
}

fn start_clipboard_watcher<R: Runtime>(app_handle: AppHandle<R>, service: ClipboardService) {
    let clipboard = app_handle.state::<tauri_plugin_clipboard::Clipboard>();
    if let Err(error) = clipboard.start_monitor(app_handle.clone()) {
        tracing::error!(event = "clipboard_watcher_start_failed", error = error);
        return;
    }

    let processor = std::sync::Arc::new(Mutex::new(ClipboardProcessor::new(
        app_handle.clone(),
        service,
    )));

    let processor_ref = std::sync::Arc::clone(&processor);
    let _listener_id = app_handle.listen_any(CLIPBOARD_PLUGIN_UPDATE_EVENT, move |_| {
        if let Ok(mut guard) = processor_ref.lock() {
            guard.handle_update_event();
        }
    });
}

fn read_image_dimensions_from_header(bytes: &[u8]) -> Option<(u32, u32)> {
    let cursor = Cursor::new(bytes);
    let reader = ImageReader::new(cursor).with_guessed_format().ok()?;
    reader.into_dimensions().ok()
}

fn save_clipboard_image_preview(
    preview_dir: &Path,
    signature: &str,
    bytes: &[u8],
) -> Result<String, Box<dyn Error>> {
    std::fs::create_dir_all(preview_dir)?;

    let preview_path = preview_dir.join(format!("{}.png", signature));
    std::fs::write(&preview_path, bytes)?;

    Ok(preview_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([
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
            tauri_plugin_global_shortcut::Builder::new().with_shortcuts([SHORTCUT_LAUNCHER_PRIMARY])
        })
        .unwrap_or_else(|error| {
            log_error_fallback(&format!("failed to register global shortcuts: {}", error));
            tauri_plugin_global_shortcut::Builder::new()
        });

    let shortcut_plugin = shortcut_builder
        .with_handler(move |app, shortcut, event| {
            if event.state == ShortcutState::Pressed {
                handle_shortcut(
                    app,
                    shortcut,
                    clipboard_shortcut_id,
                    clipboard_compact_shortcut_id,
                );
            }
        })
        .build();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard::init())
        .plugin(shortcut_plugin)
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let logging_guard = infrastructure::logging::init_logging(&app.handle())?;
            let log_dir = logging_guard.log_dir().to_path_buf();
            tracing::info!(
                event = "logging_initialized",
                level = logging_guard.level(),
                log_dir = %logging_guard.log_dir().to_string_lossy()
            );

            let app_data_dir = app.path().app_data_dir()?;
            init_i18n_catalog(&app_data_dir).map_err(std::io::Error::other)?;

            let (db_path, db_pool) = init_database(app)?;
            let initial_locale_state = read_initial_locale_state(&db_pool)?;
            let app_handle = app.handle().clone();
            let tray_menu = build_tray_menu(&app_handle, &initial_locale_state.resolved)?;

            let mut tray_builder = TrayIconBuilder::with_id(TRAY_ICON_ID)
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .tooltip(t(&initial_locale_state.resolved, "tray.tooltip"))
                .on_menu_event(|app, event| {
                    handle_tray_menu(app, event.id().as_ref());
                })
                .on_tray_icon_event(|tray, event| {
                    handle_tray_icon_event(tray.app_handle(), event);
                });
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            tray_builder.build(app)?;

            let logging_config =
                infrastructure::logging::init_log_center(&app.handle(), db_pool.clone(), log_dir)?;
            tracing::info!(
                event = "log_center_initialized",
                min_level = logging_config.min_level,
                keep_days = logging_config.keep_days,
                realtime_enabled = logging_config.realtime_enabled,
                high_freq_window_ms = logging_config.high_freq_window_ms,
                high_freq_max_per_key = logging_config.high_freq_max_per_key
            );
            let clipboard_service = ClipboardService::new(db_pool.clone(), db_path.clone())?;
            let transfer_service =
                TransferService::new(app_handle.clone(), db_pool.clone(), app_data_dir.as_path())?;
            start_clipboard_watcher(app_handle.clone(), clipboard_service.clone());
            let initial_resolved_locale = initial_locale_state.resolved.clone();
            let locale_state = Arc::new(Mutex::new(initial_locale_state));

            app.manage(AppState {
                db_path,
                db_pool,
                clipboard_service,
                transfer_service,
                locale_state,
                clipboard_window_compact: Arc::new(Mutex::new(false)),
                started_at: Instant::now(),
            });
            apply_locale_to_native_ui(&app_handle, &initial_resolved_locale);

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    tracing::warn!(
                        event = "window_hide_failed",
                        window = "main",
                        error = error.to_string()
                    );
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_manager::app_manager_list,
            commands::app_manager::app_manager_get_detail,
            commands::app_manager::app_manager_scan_residue,
            commands::app_manager::app_manager_cleanup,
            commands::app_manager::app_manager_export_scan_result,
            commands::app_manager::app_manager_refresh_index,
            commands::app_manager::app_manager_set_startup,
            commands::app_manager::app_manager_uninstall,
            commands::app_manager::app_manager_open_uninstall_help,
            commands::app_manager::app_manager_reveal_path,
            commands::launcher::launcher_search,
            commands::launcher::launcher_execute,
            commands::palette::palette_search,
            commands::palette::palette_execute,
            commands::locale::app_get_locale,
            commands::locale::app_set_locale,
            commands::i18n_import::app_list_locales,
            commands::i18n_import::app_reload_locales,
            commands::i18n_import::app_import_locale_file,
            commands::clipboard::clipboard_list,
            commands::clipboard::clipboard_pin,
            commands::clipboard::clipboard_delete,
            commands::clipboard::clipboard_clear_all,
            commands::clipboard::clipboard_save_text,
            commands::clipboard::clipboard_copy_back,
            commands::clipboard::clipboard_copy_file_paths,
            commands::clipboard::clipboard_copy_image_back,
            commands::clipboard::clipboard_get_settings,
            commands::clipboard::clipboard_update_settings,
            commands::clipboard::clipboard_window_set_mode,
            commands::clipboard::clipboard_window_apply_mode,
            commands::dashboard::dashboard_snapshot,
            commands::logging::client_log,
            commands::logging::logging_query,
            commands::logging::logging_get_config,
            commands::logging::logging_update_config,
            commands::logging::logging_export_jsonl,
            commands::transfer::transfer_get_settings,
            commands::transfer::transfer_update_settings,
            commands::transfer::transfer_generate_pairing_code,
            commands::transfer::transfer_start_discovery,
            commands::transfer::transfer_stop_discovery,
            commands::transfer::transfer_list_peers,
            commands::transfer::transfer_send_files,
            commands::transfer::transfer_pause_session,
            commands::transfer::transfer_resume_session,
            commands::transfer::transfer_cancel_session,
            commands::transfer::transfer_retry_session,
            commands::transfer::transfer_list_history,
            commands::transfer::transfer_clear_history,
            commands::transfer::transfer_open_download_dir,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log_error_fallback(&format!("error while running tauri application: {}", error));
            panic!("error while running tauri application: {}", error);
        });
}

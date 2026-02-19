use crate::app::clipboard_service::ClipboardService;
use crate::app::state::AppState;
use crate::app::transfer_service::TransferService;
use crate::clipboard_watcher::start_clipboard_watcher;
use crate::constants::TRAY_ICON_ID;
use crate::core::i18n::{
    APP_LOCALE_PREFERENCE_KEY, AppLocaleState, init_i18n_catalog, normalize_locale_preference,
    resolve_locale, t,
};
use crate::infrastructure;
use crate::native_ui::{apply_locale_to_native_ui, tray};
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Manager;
use tauri::tray::TrayIconBuilder;

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

pub(crate) fn log_warn_fallback(message: &str) {
    if tracing::dispatcher::has_been_set() {
        tracing::warn!(event = "bootstrap_warning", message = message);
        return;
    }

    eprintln!("{message}");
}

pub(crate) fn log_error_fallback(message: &str) {
    if tracing::dispatcher::has_been_set() {
        tracing::error!(event = "bootstrap_error", message = message);
        return;
    }

    eprintln!("{message}");
}

pub(crate) fn setup(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
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
    let tray_menu = tray::build_tray_menu(&app_handle, &initial_locale_state.resolved)?;

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .tooltip(t(&initial_locale_state.resolved, "tray.tooltip"))
        .on_menu_event(|app, event| {
            tray::handle_tray_menu(app, event.id().as_ref());
        })
        .on_tray_icon_event(|tray_icon, event| {
            tray::handle_tray_icon_event(tray_icon.app_handle(), event);
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
}

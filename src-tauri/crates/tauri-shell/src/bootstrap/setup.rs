use crate::app::state::AppState;
use crate::constants::TRAY_ICON_ID;
use crate::platform::clipboard_watcher::start_clipboard_watcher;
use crate::platform::native_ui::{apply_locale_to_native_ui, tray};
use app_clipboard::service::ClipboardService;
use app_core::{
    AppResult,
    i18n::{
        APP_LOCALE_PREFERENCE_KEY, AppLocaleState, init_i18n_catalog, normalize_locale_preference,
        resolve_locale, t,
    },
};
use app_infra::{db, logging};
use app_launcher_app::launcher::index::start_background_indexer;
use app_transfer::service::{TransferService, TransferTask, TransferTaskSpawner};
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Manager;
use tauri::tray::TrayIconBuilder;
use tokio::task::JoinHandle;

#[derive(Default)]
struct TauriTransferTaskSpawner;

impl TransferTaskSpawner for TauriTransferTaskSpawner {
    fn spawn(&self, _task_name: &'static str, task: TransferTask) -> AppResult<JoinHandle<()>> {
        match tauri::async_runtime::spawn(task) {
            tauri::async_runtime::JoinHandle::Tokio(handle) => Ok(handle),
        }
    }
}

async fn read_initial_locale_state(db_conn: &db::DbConn) -> Result<AppLocaleState, Box<dyn Error>> {
    let preference = db::get_app_setting(db_conn, APP_LOCALE_PREFERENCE_KEY)
        .await?
        .as_deref()
        .and_then(normalize_locale_preference)
        .unwrap_or_else(|| "system".to_string());
    Ok(AppLocaleState::new(
        preference.clone(),
        resolve_locale(&preference),
    ))
}

fn cleanup_legacy_db_files(legacy_db_path: &Path) {
    let mut paths = vec![legacy_db_path.to_path_buf()];
    paths.push(PathBuf::from(format!("{}-wal", legacy_db_path.display())));
    paths.push(PathBuf::from(format!("{}-shm", legacy_db_path.display())));

    for path in paths {
        if let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            log_warn_fallback(&format!(
                "legacy_db_cleanup_failed: path={}, error={}",
                path.display(),
                error
            ));
        }
    }
}

async fn init_database(app: &tauri::App) -> Result<(PathBuf, db::DbConn), Box<dyn Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    let legacy_db_path = app_data_dir.join("rtool.db");
    let db_path = app_data_dir.join("rtool-turso.db");
    let db_conn = db::open_db(&db_path).await?;
    db::init_db(&db_conn).await?;
    cleanup_legacy_db_files(&legacy_db_path);
    Ok((db_path, db_conn))
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

fn log_setup_stage(stage: &str, started_at: Instant, ok: bool) {
    tracing::info!(
        event = "setup_stage_done",
        stage = stage,
        duration_ms = started_at.elapsed().as_millis() as u64,
        ok = ok
    );
}

pub(crate) fn setup(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
    let setup_started_at = Instant::now();
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let app_data_dir = app.path().app_data_dir()?;
    let logging_guard = logging::init_logging(app_data_dir.as_path())?;
    let log_dir = logging_guard.log_dir().to_path_buf();
    tracing::info!(
        event = "logging_initialized",
        level = logging_guard.level(),
        log_dir = %logging_guard.log_dir().to_string_lossy()
    );

    let i18n_stage_started_at = Instant::now();
    let i18n_result = init_i18n_catalog(&app_data_dir).map_err(std::io::Error::other);
    log_setup_stage("i18n_init", i18n_stage_started_at, i18n_result.is_ok());
    i18n_result?;

    let db_stage_started_at = Instant::now();
    let db_result = tauri::async_runtime::block_on(init_database(app));
    log_setup_stage("db_init", db_stage_started_at, db_result.is_ok());
    let (db_path, db_conn) = db_result?;

    let locale_stage_started_at = Instant::now();
    let locale_result = tauri::async_runtime::block_on(read_initial_locale_state(&db_conn));
    log_setup_stage(
        "locale_read",
        locale_stage_started_at,
        locale_result.is_ok(),
    );
    let initial_locale_state = locale_result?;
    let app_handle = app.handle().clone();

    let tray_stage_started_at = Instant::now();
    let tray_result: Result<(), Box<dyn Error>> = {
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
        Ok(())
    };
    log_setup_stage("tray_build", tray_stage_started_at, tray_result.is_ok());
    tray_result?;

    let log_center_stage_started_at = Instant::now();
    let log_center_result: Result<_, Box<dyn Error>> = {
        let logging_event_sink = Arc::new(
            crate::features::logging::events::TauriLoggingEventSink::new(app_handle.clone()),
        );
        tauri::async_runtime::block_on(logging::init_log_center(
            db_conn.clone(),
            log_dir,
            Some(logging_event_sink),
        ))
        .map_err(Into::into)
    };
    log_setup_stage(
        "log_center_init",
        log_center_stage_started_at,
        log_center_result.is_ok(),
    );
    let logging_config = log_center_result?;
    tracing::info!(
        event = "log_center_initialized",
        min_level = logging_config.min_level,
        keep_days = logging_config.keep_days,
        realtime_enabled = logging_config.realtime_enabled,
        high_freq_window_ms = logging_config.high_freq_window_ms,
        high_freq_max_per_key = logging_config.high_freq_max_per_key
    );

    let clipboard_stage_started_at = Instant::now();
    let clipboard_result =
        tauri::async_runtime::block_on(ClipboardService::new(db_conn.clone(), db_path.clone()));
    log_setup_stage(
        "clipboard_init",
        clipboard_stage_started_at,
        clipboard_result.is_ok(),
    );
    let clipboard_service = clipboard_result?;

    let transfer_stage_started_at = Instant::now();
    let transfer_event_sink = Arc::new(
        crate::features::transfer::events::TauriTransferEventSink::new(app_handle.clone()),
    );
    let transfer_task_spawner = Arc::new(TauriTransferTaskSpawner);
    let transfer_service_result = tauri::async_runtime::block_on(TransferService::new(
        transfer_event_sink,
        transfer_task_spawner,
        db_conn.clone(),
        app_data_dir.as_path(),
    ));
    if transfer_service_result.is_err() {
        log_setup_stage("transfer_init", transfer_stage_started_at, false);
    }
    let mut transfer_stage_ok = transfer_service_result.is_ok();
    let transfer_service = transfer_service_result?;
    if let Err(error) = transfer_service.bootstrap_background_tasks() {
        transfer_stage_ok = false;
        tracing::error!(
            event = "transfer_background_bootstrap_failed",
            error_code = error.code,
            error_detail = error.causes.first().map(String::as_str).unwrap_or_default()
        );
    }
    log_setup_stage(
        "transfer_init",
        transfer_stage_started_at,
        transfer_stage_ok,
    );

    start_clipboard_watcher(app_handle.clone(), clipboard_service.clone());
    let initial_resolved_locale = initial_locale_state.resolved.clone();
    let locale_state = Arc::new(Mutex::new(initial_locale_state));

    app.manage(AppState {
        db_path,
        db_conn: db_conn.clone(),
        clipboard_service,
        transfer_service,
        locale_state,
        clipboard_window_compact: Arc::new(Mutex::new(false)),
        started_at: Instant::now(),
    });
    start_background_indexer(db_conn);
    apply_locale_to_native_ui(&app_handle, &initial_resolved_locale);
    log_setup_stage("setup_total", setup_started_at, transfer_stage_ok);

    Ok(())
}

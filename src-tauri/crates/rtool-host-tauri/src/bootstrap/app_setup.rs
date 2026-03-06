use crate::app::state::AppState;
use crate::constants::{
    RUNTIME_WORKER_APP_MANAGER, RUNTIME_WORKER_CLIPBOARD, RUNTIME_WORKER_LAUNCHER,
    RUNTIME_WORKER_SCREENSHOT, SHORTCUT_SCREENSHOT_DEFAULT, TRAY_ICON_ID,
};
use crate::platform::clipboard_watcher::start_clipboard_watcher;
use crate::platform::native_ui::{apply_locale_to_native_ui, apply_window_chrome, shortcuts, tray};
use rtool_app::{
    AppLocaleState, ApplicationServices, BootstrapApplicationService, LocaleApplicationService,
    ScreenshotApplicationService, SettingsApplicationService,
};
use rtool_contracts::models::SettingsDto;
use rtool_kernel::{RuntimeOrchestrator, RuntimeState};
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tauri::tray::TrayIconBuilder;

fn locale_state_from_settings(settings: &SettingsDto) -> AppLocaleState {
    LocaleApplicationService.state_from_settings(settings)
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

fn run_setup_stage<T, E, F>(stage: &str, task: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E>,
{
    let started_at = Instant::now();
    let result = task();
    log_setup_stage(stage, started_at, result.is_ok());
    result
}

pub(crate) fn try_load_startup_settings() -> Option<SettingsDto> {
    // Settings are DB-backed after refactor and the DB connection is initialized in setup.
    // Bootstrap prefetch falls back to runtime defaults before setup completes.
    None
}

fn start_screenshot_session_sweeper(orchestrator: &RuntimeOrchestrator) {
    const SWEEP_INTERVAL: Duration = Duration::from_secs(5);
    orchestrator.mark_running(RUNTIME_WORKER_SCREENSHOT);
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(SWEEP_INTERVAL).await;
            let removed = ScreenshotApplicationService.sweep_expired_sessions_now();
            if removed > 0 {
                tracing::debug!(event = "screenshot_session_sweep", removed);
            }
        }
    });
}

pub(crate) fn setup(
    app: &mut tauri::App,
    startup_settings: Option<SettingsDto>,
    screenshot_shortcut_id: Option<u32>,
) -> Result<(), Box<dyn Error>> {
    let setup_started_at = Instant::now();
    let bootstrap_service = BootstrapApplicationService;
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let app_data_dir = app.path().app_data_dir()?;
    let logging_guard = bootstrap_service.init_logging(app_data_dir.as_path())?;
    let log_dir = logging_guard.log_dir().to_path_buf();
    tracing::info!(
        event = "logging_initialized",
        level = logging_guard.level(),
        log_dir = %logging_guard.log_dir().to_string_lossy()
    );

    run_setup_stage("i18n_init", || {
        LocaleApplicationService.init_catalog(&app_data_dir)
    })?;

    let (db_path, db_conn) = run_setup_stage("db_init", || {
        tauri::async_runtime::block_on(bootstrap_service.init_database(&app_data_dir))
            .map_err(|error| -> Box<dyn Error> { Box::new(error) })
    })?;
    let settings_service = SettingsApplicationService::new(db_conn.clone());

    let settings = run_setup_stage("settings_ready", || match startup_settings {
        Some(settings) => Ok(settings),
        None => tauri::async_runtime::block_on(settings_service.load_or_init())
            .map_err(|error| -> Box<dyn Error> { Box::new(error) }),
    })?;

    let initial_locale_state = run_setup_stage("locale_read", || {
        Ok::<AppLocaleState, Box<dyn Error>>(locale_state_from_settings(&settings))
    })?;
    let app_handle = app.handle().clone();
    let runtime_orchestrator = RuntimeOrchestrator::new();
    runtime_orchestrator.register_workers(&[
        RUNTIME_WORKER_CLIPBOARD,
        RUNTIME_WORKER_APP_MANAGER,
        RUNTIME_WORKER_SCREENSHOT,
        RUNTIME_WORKER_LAUNCHER,
    ]);
    apply_window_chrome(&app_handle, settings.theme.transparent_window_background);

    run_setup_stage("tray_build", || {
        let locale_service = LocaleApplicationService;
        let tray_menu = tray::build_tray_menu(&app_handle, &initial_locale_state.resolved)?;
        let mut tray_builder = TrayIconBuilder::with_id(TRAY_ICON_ID)
            .menu(&tray_menu)
            .show_menu_on_left_click(false)
            .tooltip(locale_service.translate(&initial_locale_state.resolved, "tray.tooltip"))
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
        Ok::<(), Box<dyn Error>>(())
    })?;

    let logging_config = run_setup_stage("log_center_init", || {
        let logging_event_sink: Arc<dyn rtool_app::LoggingEventSink> = Arc::new(
            crate::features::logging::events::TauriLoggingEventSink::new(app_handle.clone()),
        );
        tauri::async_runtime::block_on(bootstrap_service.init_log_center(
            db_conn.clone(),
            log_dir,
            Some(logging_event_sink),
        ))
        .map_err(|error| -> Box<dyn Error> { Box::new(error) })
    })?;
    tracing::info!(
        event = "log_center_initialized",
        min_level = logging_config.min_level,
        keep_days = logging_config.keep_days,
        realtime_enabled = logging_config.realtime_enabled,
        high_freq_window_ms = logging_config.high_freq_window_ms,
        high_freq_max_per_key = logging_config.high_freq_max_per_key
    );

    let clipboard_service = run_setup_stage("clipboard_init", || {
        tauri::async_runtime::block_on(bootstrap_service.init_clipboard_service(
            db_conn.clone(),
            db_path.clone(),
            settings.clipboard.clone(),
        ))
        .map_err(|error| -> Box<dyn Error> { Box::new(error) })
    })?;

    let initial_resolved_locale = initial_locale_state.resolved.clone();
    let runtime_state =
        RuntimeState::new(initial_locale_state, Instant::now(), screenshot_shortcut_id);
    let app_services = ApplicationServices::new(db_conn.clone(), clipboard_service);

    app.manage(crate::platform::native_ui::window_factory::WindowWarmupState::default());

    match start_clipboard_watcher(app_handle.clone(), app_services.clipboard.clone()) {
        Ok(()) => runtime_orchestrator.mark_running(RUNTIME_WORKER_CLIPBOARD),
        Err(error) => {
            runtime_orchestrator.mark_error(
                RUNTIME_WORKER_CLIPBOARD,
                format!("{}: {}", error.code, error.message),
            );
            tracing::error!(
                event = "clipboard_watcher_start_failed",
                error_code = error.code,
                error_detail = error.causes.first().map(String::as_str).unwrap_or_default()
            );
        }
    }
    app_services.start_background_workers();

    app.manage(AppState {
        db_path,
        app_services,
        runtime_state,
        runtime_orchestrator: runtime_orchestrator.clone(),
    });

    crate::platform::native_ui::window_factory::warmup_secondary_windows(app_handle.clone());

    if settings.screenshot.shortcut != SHORTCUT_SCREENSHOT_DEFAULT
        && let Err(error) = shortcuts::rebind_screenshot_shortcut(
            &app_handle,
            SHORTCUT_SCREENSHOT_DEFAULT,
            settings.screenshot.shortcut.as_str(),
        )
    {
        tracing::warn!(
            event = "screenshot_shortcut_bootstrap_rebind_failed",
            configured_shortcut = settings.screenshot.shortcut,
            error = error.to_string()
        );
    }

    runtime_orchestrator.mark_running(RUNTIME_WORKER_LAUNCHER);
    start_screenshot_session_sweeper(&runtime_orchestrator);
    apply_locale_to_native_ui(&app_handle, &initial_resolved_locale);
    log_setup_stage("setup_total", setup_started_at, true);

    Ok(())
}

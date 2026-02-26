use super::{run_blocking_command, run_command_sync};
use crate::app::state::AppState;
use crate::host::launcher::TauriLauncherHost;
use anyhow::Context;
use app_application::AppManagerApplicationService;
use app_core::models::{
    AppManagerActionResultDto, AppManagerCleanupInputDto, AppManagerCleanupResultDto,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerPageDto, AppManagerQueryDto, AppManagerResidueScanInputDto,
    AppManagerResidueScanResultDto, AppManagerResolveSizesInputDto,
    AppManagerResolveSizesResultDto, AppManagerSnapshotMetaDto, AppManagerStartupUpdateInputDto,
    AppManagerUninstallInputDto, ManagedAppDetailDto,
};
use app_core::{AppError, AppResult, InvokeError, ResultExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{Emitter, State};
use tokio::sync::Notify;
use tokio::time::sleep;

fn app_manager_watcher_started() -> &'static AtomicBool {
    static STARTED: OnceLock<AtomicBool> = OnceLock::new();
    STARTED.get_or_init(|| AtomicBool::new(false))
}

fn app_manager_watcher_notify() -> Arc<Notify> {
    static SIGNAL: OnceLock<Arc<Notify>> = OnceLock::new();
    SIGNAL.get_or_init(|| Arc::new(Notify::new())).clone()
}

fn trigger_app_manager_watcher_refresh() {
    app_manager_watcher_notify().notify_one();
}

const APP_MANAGER_AUTO_REFRESH_BASE_INTERVAL_SECS: u64 = 20;
const APP_MANAGER_AUTO_REFRESH_MIN_INTERVAL_SECS: u64 = 5;
const APP_MANAGER_AUTO_REFRESH_MAX_INTERVAL_SECS: u64 = 120;

fn next_poll_interval(current: Duration) -> Duration {
    let doubled = current.as_secs().saturating_mul(2);
    Duration::from_secs(doubled.clamp(
        APP_MANAGER_AUTO_REFRESH_MIN_INTERVAL_SECS,
        APP_MANAGER_AUTO_REFRESH_MAX_INTERVAL_SECS,
    ))
}

fn ensure_app_manager_watcher_started(
    app: &tauri::AppHandle,
    service: AppManagerApplicationService,
) {
    let started = app_manager_watcher_started();
    if started
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let app_handle = app.clone();
    let wake_signal = app_manager_watcher_notify();
    tauri::async_runtime::spawn(async move {
        let mut wait_for = Duration::from_secs(APP_MANAGER_AUTO_REFRESH_BASE_INTERVAL_SECS);
        let mut run_immediately = true;
        loop {
            if run_immediately {
                run_immediately = false;
            } else {
                tokio::select! {
                    _ = sleep(wait_for) => {}
                    _ = wake_signal.notified() => {
                        wait_for = Duration::from_secs(APP_MANAGER_AUTO_REFRESH_MIN_INTERVAL_SECS);
                    }
                }
            }
            let host = TauriLauncherHost::new(app_handle.clone());
            let poll_result = run_blocking_command(
                "app_manager_auto_refresh_poll",
                Some("app_manager_watcher".to_string()),
                Some("main".to_string()),
                "app_manager_auto_refresh_poll",
                move || service.poll_auto_refresh(&host),
            )
            .await;
            match poll_result {
                Ok(Some(payload)) => {
                    let _ = app_handle.emit("rtool://app-manager/index-updated", payload);
                    wait_for = Duration::from_secs(APP_MANAGER_AUTO_REFRESH_MIN_INTERVAL_SECS);
                }
                Ok(None) => {
                    wait_for = next_poll_interval(wait_for);
                }
                Err(error) => {
                    tracing::debug!(
                        event = "app_manager_auto_refresh_poll_failed",
                        code = error.code.as_str(),
                        message = error.message.as_str(),
                        retry_in_secs = wait_for.as_secs()
                    );
                    wait_for = next_poll_interval(wait_for);
                }
            }
        }
    });
}

fn reveal_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(
            AppError::new("app_manager_reveal_not_found", "定位失败：目标路径不存在")
                .with_context("path", path.to_string_lossy().to_string()),
        );
    }

    let target = path.to_path_buf();
    let command_result = if cfg!(target_os = "macos") {
        Command::new("open").arg("-R").arg(&target).status()
    } else if cfg!(target_os = "windows") {
        Command::new("explorer")
            .arg(format!("/select,{}", target.to_string_lossy()))
            .status()
    } else {
        let fallback = if target.is_dir() {
            target.clone()
        } else {
            target
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| path.to_path_buf())
        };
        Command::new("xdg-open").arg(fallback).status()
    };

    let status = command_result
        .with_context(|| {
            format!(
                "failed to launch file manager for {}",
                target.to_string_lossy()
            )
        })
        .with_code(
            "app_manager_reveal_failed",
            "定位失败：无法启动系统文件管理器",
        )?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::new(
            "app_manager_reveal_failed",
            "定位失败：系统文件管理器调用异常",
        )
        .with_context("path", target.to_string_lossy().to_string())
        .with_context("status", status.to_string()))
    }
}

#[tauri::command]
pub async fn app_manager_list(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    query: Option<AppManagerQueryDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerPageDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let input_query = query.unwrap_or_default();
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_list",
        request_id,
        window_label,
        "app_manager_list",
        move || service.list(&host, input_query),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_get_detail(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    query: AppManagerDetailQueryDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ManagedAppDetailDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_get_detail",
        request_id,
        window_label,
        "app_manager_get_detail",
        move || service.get_detail(&host, query),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_list_snapshot_meta(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerSnapshotMetaDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_list_snapshot_meta",
        request_id,
        window_label,
        "app_manager_list_snapshot_meta",
        move || service.list_snapshot_meta(&host),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_resolve_sizes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerResolveSizesInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerResolveSizesResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_resolve_sizes",
        request_id,
        window_label,
        "app_manager_resolve_sizes",
        move || service.resolve_sizes(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_get_detail_core(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    query: AppManagerDetailQueryDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ManagedAppDetailDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_get_detail_core",
        request_id,
        window_label,
        "app_manager_get_detail_core",
        move || service.get_detail_core(&host, query),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_get_detail_heavy(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerResidueScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerResidueScanResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_get_detail_heavy",
        request_id,
        window_label,
        "app_manager_get_detail_heavy",
        move || service.get_detail_heavy(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_scan_residue(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerResidueScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerResidueScanResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_scan_residue",
        request_id,
        window_label,
        "app_manager_scan_residue",
        move || service.scan_residue(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_cleanup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerCleanupInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerCleanupResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    let result = run_blocking_command(
        "app_manager_cleanup",
        request_id,
        window_label,
        "app_manager_cleanup",
        move || service.cleanup(&host, input),
    )
    .await;
    if result.is_ok() {
        trigger_app_manager_watcher_refresh();
    }
    result
}

#[tauri::command]
pub async fn app_manager_export_scan_result(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerExportScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerExportScanResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_export_scan_result",
        request_id,
        window_label,
        "app_manager_export_scan_result",
        move || service.export_scan_result(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_refresh_index(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    let result = run_blocking_command(
        "app_manager_refresh_index",
        request_id,
        window_label,
        "app_manager_refresh_index",
        move || service.refresh_index(&host),
    )
    .await;
    if result.is_ok() {
        trigger_app_manager_watcher_refresh();
    }
    result
}

#[tauri::command]
pub async fn app_manager_set_startup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerStartupUpdateInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    let result = run_blocking_command(
        "app_manager_set_startup",
        request_id,
        window_label,
        "app_manager_set_startup",
        move || service.set_startup(&host, input),
    )
    .await;
    if result.is_ok() {
        trigger_app_manager_watcher_refresh();
    }
    result
}

#[tauri::command]
pub async fn app_manager_uninstall(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerUninstallInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    let result = run_blocking_command(
        "app_manager_uninstall",
        request_id,
        window_label,
        "app_manager_uninstall",
        move || service.uninstall(&host, input),
    )
    .await;
    if result.is_ok() {
        trigger_app_manager_watcher_refresh();
    }
    result
}

#[tauri::command]
pub async fn app_manager_open_uninstall_help(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    app_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    let service = state.app_services.app_manager;
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_open_uninstall_help",
        request_id,
        window_label,
        "app_manager_open_uninstall_help",
        move || service.open_uninstall_help(&host, app_id),
    )
    .await
}

#[tauri::command]
pub fn app_manager_reveal_path(
    path: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_sync(
        "app_manager_reveal_path",
        request_id,
        window_label,
        move || {
            let trimmed = path.trim();
            if trimmed.is_empty() {
                return Err(AppError::new(
                    "app_manager_reveal_invalid",
                    "定位失败：路径不能为空",
                ));
            }

            reveal_path(Path::new(trimmed))
        },
    )
}

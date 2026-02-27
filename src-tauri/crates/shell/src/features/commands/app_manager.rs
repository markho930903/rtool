use super::{run_blocking_command, run_command_sync};
use crate::app::state::AppState;
use crate::features::command_payload::{
    CommandPayloadContext, CommandRequestDto,
};
use crate::host::launcher::TauriLauncherHost;
use anyhow::Context;
use protocol::models::{
    AppManagerActionResultDto, AppManagerCleanupInputDto, AppManagerCleanupResultDto,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerPageDto, AppManagerQueryDto, AppManagerResidueScanInputDto,
    AppManagerResidueScanResultDto, AppManagerResolveSizesInputDto,
    AppManagerResolveSizesResultDto, AppManagerSnapshotMetaDto, AppManagerStartupUpdateInputDto,
    AppManagerUninstallInputDto, ManagedAppDetailDto,
};
use protocol::{AppError, AppResult, InvokeError, ResultExt};
use rtool_core::AppManagerApplicationService;
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{Emitter, State};
use tokio::sync::Notify;
use tokio::time::sleep;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct AppManagerListPayload {
    query: Option<AppManagerQueryDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerDetailPayload {
    query: AppManagerDetailQueryDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerResolveSizesPayload {
    input: AppManagerResolveSizesInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerResidueInputPayload {
    input: AppManagerResidueScanInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerCleanupPayload {
    input: AppManagerCleanupInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerExportPayload {
    input: AppManagerExportScanInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerStartupPayload {
    input: AppManagerStartupUpdateInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerUninstallPayload {
    input: AppManagerUninstallInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerHelpPayload {
    app_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppManagerRevealPayload {
    path: String,
}

const APP_MANAGER_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "app_manager",
    "应用管理命令参数无效",
    "应用管理命令返回序列化失败",
    "未知应用管理命令",
);

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

async fn run_app_manager_command<T, F>(
    app: tauri::AppHandle,
    service: AppManagerApplicationService,
    request_id: Option<String>,
    window_label: Option<String>,
    command_name: &'static str,
    operation: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    F: FnOnce(AppManagerApplicationService, TauriLauncherHost) -> AppResult<T> + Send + 'static,
{
    ensure_app_manager_watcher_started(&app, service);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        command_name,
        request_id,
        window_label,
        command_name,
        move || operation(service, host),
    )
    .await
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
    let input_query = query.unwrap_or_default();
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_list",
        move |service, host| service.list(&host, input_query),
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
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_get_detail",
        move |service, host| service.get_detail(&host, query),
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
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_list_snapshot_meta",
        move |service, host| service.list_snapshot_meta(&host),
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
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_resolve_sizes",
        move |service, host| service.resolve_sizes(&host, input),
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
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_get_detail_core",
        move |service, host| service.get_detail_core(&host, query),
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
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_get_detail_heavy",
        move |service, host| service.get_detail_heavy(&host, input),
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
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_scan_residue",
        move |service, host| service.scan_residue(&host, input),
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
    let result = run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_cleanup",
        move |service, host| service.cleanup(&host, input),
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
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_export_scan_result",
        move |service, host| service.export_scan_result(&host, input),
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
    let result = run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_refresh_index",
        move |service, host| service.refresh_index(&host),
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
    let result = run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_set_startup",
        move |service, host| service.set_startup(&host, input),
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
    let result = run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_uninstall",
        move |service, host| service.uninstall(&host, input),
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
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        request_id,
        window_label,
        "app_manager_open_uninstall_help",
        move |service, host| service.open_uninstall_help(&host, app_id),
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

#[tauri::command]
pub async fn app_manager_handle(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "list" => {
            let payload: AppManagerListPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("list", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "list",
                app_manager_list(app, state, payload.query, request_id, window_label).await?,
            )
        }
        "get_detail" => {
            let payload: AppManagerDetailPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("get_detail", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "get_detail",
                app_manager_get_detail(app, state, payload.query, request_id, window_label).await?,
            )
        }
        "list_snapshot_meta" => APP_MANAGER_COMMAND_CONTEXT.serialize(
            "list_snapshot_meta",
            app_manager_list_snapshot_meta(app, state, request_id, window_label).await?,
        ),
        "resolve_sizes" => {
            let payload: AppManagerResolveSizesPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("resolve_sizes", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "resolve_sizes",
                app_manager_resolve_sizes(app, state, payload.input, request_id, window_label)
                    .await?,
            )
        }
        "get_detail_core" => {
            let payload: AppManagerDetailPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("get_detail_core", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "get_detail_core",
                app_manager_get_detail_core(app, state, payload.query, request_id, window_label)
                    .await?,
            )
        }
        "get_detail_heavy" => {
            let payload: AppManagerResidueInputPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("get_detail_heavy", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "get_detail_heavy",
                app_manager_get_detail_heavy(app, state, payload.input, request_id, window_label)
                    .await?,
            )
        }
        "scan_residue" => {
            let payload: AppManagerResidueInputPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("scan_residue", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "scan_residue",
                app_manager_scan_residue(app, state, payload.input, request_id, window_label).await?,
            )
        }
        "cleanup" => {
            let payload: AppManagerCleanupPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("cleanup", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "cleanup",
                app_manager_cleanup(app, state, payload.input, request_id, window_label).await?,
            )
        }
        "export_scan_result" => {
            let payload: AppManagerExportPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("export_scan_result", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "export_scan_result",
                app_manager_export_scan_result(
                    app,
                    state,
                    payload.input,
                    request_id,
                    window_label,
                )
                .await?,
            )
        }
        "refresh_index" => APP_MANAGER_COMMAND_CONTEXT.serialize(
            "refresh_index",
            app_manager_refresh_index(app, state, request_id, window_label).await?,
        ),
        "set_startup" => {
            let payload: AppManagerStartupPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("set_startup", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "set_startup",
                app_manager_set_startup(app, state, payload.input, request_id, window_label).await?,
            )
        }
        "uninstall" => {
            let payload: AppManagerUninstallPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("uninstall", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "uninstall",
                app_manager_uninstall(app, state, payload.input, request_id, window_label).await?,
            )
        }
        "open_uninstall_help" => {
            let payload: AppManagerHelpPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("open_uninstall_help", request.payload)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize(
                "open_uninstall_help",
                app_manager_open_uninstall_help(
                    app,
                    state,
                    payload.app_id,
                    request_id,
                    window_label,
                )
                .await?,
            )
        }
        "reveal_path" => {
            let payload: AppManagerRevealPayload =
                APP_MANAGER_COMMAND_CONTEXT.parse("reveal_path", request.payload)?;
            app_manager_reveal_path(payload.path, request_id, window_label)?;
            Ok(Value::Null)
        }
        _ => Err(APP_MANAGER_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

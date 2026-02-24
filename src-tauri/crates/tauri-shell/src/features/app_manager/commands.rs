use super::{run_blocking_command, run_command_sync};
use crate::host::launcher::TauriLauncherHost;
use anyhow::Context;
use app_core::models::{
    AppManagerActionResultDto, AppManagerCleanupInputDto, AppManagerCleanupResultDto,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerPageDto, AppManagerQueryDto, AppManagerResidueScanInputDto,
    AppManagerResidueScanResultDto, AppManagerStartupUpdateInputDto, AppManagerUninstallInputDto,
    ManagedAppDetailDto,
};
use app_core::{AppError, AppResult, InvokeError, ResultExt};
use app_launcher_app::app_manager::{
    cleanup_managed_app_residue, export_managed_app_scan_result, get_managed_app_detail,
    list_managed_apps, open_uninstall_help, poll_managed_apps_auto_refresh,
    refresh_managed_apps_index, scan_managed_app_residue, set_managed_app_startup,
    uninstall_managed_app,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Emitter;
use tokio::time::sleep;

fn app_manager_watcher_started() -> &'static AtomicBool {
    static STARTED: OnceLock<AtomicBool> = OnceLock::new();
    STARTED.get_or_init(|| AtomicBool::new(false))
}

fn ensure_app_manager_watcher_started(app: &tauri::AppHandle) {
    let started = app_manager_watcher_started();
    if started
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            sleep(Duration::from_secs(20)).await;
            let host = TauriLauncherHost::new(app_handle.clone());
            let poll_result = run_blocking_command(
                "app_manager_auto_refresh_poll",
                Some("app_manager_watcher".to_string()),
                Some("main".to_string()),
                "app_manager_auto_refresh_poll",
                move || poll_managed_apps_auto_refresh(&host),
            )
            .await;
            match poll_result {
                Ok(Some(payload)) => {
                    let _ = app_handle.emit("rtool://app-manager/index-updated", payload);
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::debug!(
                        event = "app_manager_auto_refresh_poll_failed",
                        code = error.code.as_str(),
                        message = error.message.as_str()
                    );
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
    query: Option<AppManagerQueryDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerPageDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let input_query = query.unwrap_or_default();
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_list",
        request_id,
        window_label,
        "app_manager_list",
        move || list_managed_apps(&host, input_query),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_get_detail(
    app: tauri::AppHandle,
    query: AppManagerDetailQueryDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ManagedAppDetailDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_get_detail",
        request_id,
        window_label,
        "app_manager_get_detail",
        move || get_managed_app_detail(&host, query),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_scan_residue(
    app: tauri::AppHandle,
    input: AppManagerResidueScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerResidueScanResultDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_scan_residue",
        request_id,
        window_label,
        "app_manager_scan_residue",
        move || scan_managed_app_residue(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_cleanup(
    app: tauri::AppHandle,
    input: AppManagerCleanupInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerCleanupResultDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_cleanup",
        request_id,
        window_label,
        "app_manager_cleanup",
        move || cleanup_managed_app_residue(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_export_scan_result(
    app: tauri::AppHandle,
    input: AppManagerExportScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerExportScanResultDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_export_scan_result",
        request_id,
        window_label,
        "app_manager_export_scan_result",
        move || export_managed_app_scan_result(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_refresh_index(
    app: tauri::AppHandle,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_refresh_index",
        request_id,
        window_label,
        "app_manager_refresh_index",
        move || refresh_managed_apps_index(&host),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_set_startup(
    app: tauri::AppHandle,
    input: AppManagerStartupUpdateInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_set_startup",
        request_id,
        window_label,
        "app_manager_set_startup",
        move || set_managed_app_startup(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_uninstall(
    app: tauri::AppHandle,
    input: AppManagerUninstallInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_uninstall",
        request_id,
        window_label,
        "app_manager_uninstall",
        move || uninstall_managed_app(&host, input),
    )
    .await
}

#[tauri::command]
pub async fn app_manager_open_uninstall_help(
    app: tauri::AppHandle,
    app_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    ensure_app_manager_watcher_started(&app);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "app_manager_open_uninstall_help",
        request_id,
        window_label,
        "app_manager_open_uninstall_help",
        move || open_uninstall_help(&host, app_id),
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

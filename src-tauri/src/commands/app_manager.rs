use super::{command_end_error, command_end_ok, command_start, normalize_request_id};
use crate::app::app_manager_service::{
    cleanup_managed_app_residue, export_managed_app_scan_result, get_managed_app_detail,
    list_managed_apps, open_uninstall_help, refresh_managed_apps_index, scan_managed_app_residue,
    set_managed_app_startup, uninstall_managed_app,
};
use crate::core::models::{
    AppManagerActionResultDto, AppManagerCleanupInputDto, AppManagerCleanupResultDto,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerPageDto, AppManagerQueryDto, AppManagerResidueScanInputDto,
    AppManagerResidueScanResultDto, AppManagerStartupUpdateInputDto, AppManagerUninstallInputDto,
    ManagedAppDetailDto,
};
use crate::core::{AppError, AppResult};
use crate::infrastructure::runtime::blocking::run_blocking;
use std::path::{Path, PathBuf};
use std::process::Command;

fn reveal_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(
            AppError::new("app_manager_reveal_not_found", "定位失败：目标路径不存在")
                .with_detail(path.to_string_lossy().to_string()),
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
            target
        } else {
            target
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| path.to_path_buf())
        };
        Command::new("xdg-open").arg(fallback).status()
    };

    let status = command_result.map_err(|error| {
        AppError::new(
            "app_manager_reveal_failed",
            "定位失败：无法启动系统文件管理器",
        )
        .with_detail(error.to_string())
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::new(
            "app_manager_reveal_failed",
            "定位失败：系统文件管理器调用异常",
        )
        .with_detail(format!("status={status}")))
    }
}

#[tauri::command]
pub async fn app_manager_list(
    app: tauri::AppHandle,
    query: Option<AppManagerQueryDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerPageDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("app_manager_list", &request_id, window_label.as_deref());
    let input_query = query.unwrap_or_default();
    let result = run_blocking("app_manager_list", move || {
        list_managed_apps(&app, input_query)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_list", &request_id, started_at),
        Err(error) => command_end_error("app_manager_list", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn app_manager_get_detail(
    app: tauri::AppHandle,
    query: AppManagerDetailQueryDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ManagedAppDetailDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_manager_get_detail",
        &request_id,
        window_label.as_deref(),
    );
    let result = run_blocking("app_manager_get_detail", move || {
        get_managed_app_detail(&app, query)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_get_detail", &request_id, started_at),
        Err(error) => command_end_error("app_manager_get_detail", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn app_manager_scan_residue(
    app: tauri::AppHandle,
    input: AppManagerResidueScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerResidueScanResultDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_manager_scan_residue",
        &request_id,
        window_label.as_deref(),
    );
    let result = run_blocking("app_manager_scan_residue", move || {
        scan_managed_app_residue(&app, input)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_scan_residue", &request_id, started_at),
        Err(error) => command_end_error("app_manager_scan_residue", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn app_manager_cleanup(
    app: tauri::AppHandle,
    input: AppManagerCleanupInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerCleanupResultDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("app_manager_cleanup", &request_id, window_label.as_deref());
    let result = run_blocking("app_manager_cleanup", move || {
        cleanup_managed_app_residue(&app, input)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_cleanup", &request_id, started_at),
        Err(error) => command_end_error("app_manager_cleanup", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn app_manager_export_scan_result(
    app: tauri::AppHandle,
    input: AppManagerExportScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerExportScanResultDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_manager_export_scan_result",
        &request_id,
        window_label.as_deref(),
    );
    let result = run_blocking("app_manager_export_scan_result", move || {
        export_managed_app_scan_result(&app, input)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_export_scan_result", &request_id, started_at),
        Err(error) => command_end_error(
            "app_manager_export_scan_result",
            &request_id,
            started_at,
            error,
        ),
    }
    result
}

#[tauri::command]
pub async fn app_manager_refresh_index(
    app: tauri::AppHandle,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_manager_refresh_index",
        &request_id,
        window_label.as_deref(),
    );
    let result = run_blocking("app_manager_refresh_index", move || {
        refresh_managed_apps_index(&app)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_refresh_index", &request_id, started_at),
        Err(error) => {
            command_end_error("app_manager_refresh_index", &request_id, started_at, error)
        }
    }
    result
}

#[tauri::command]
pub async fn app_manager_set_startup(
    app: tauri::AppHandle,
    input: AppManagerStartupUpdateInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_manager_set_startup",
        &request_id,
        window_label.as_deref(),
    );
    let result = run_blocking("app_manager_set_startup", move || {
        set_managed_app_startup(&app, input)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_set_startup", &request_id, started_at),
        Err(error) => command_end_error("app_manager_set_startup", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn app_manager_uninstall(
    app: tauri::AppHandle,
    input: AppManagerUninstallInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_manager_uninstall",
        &request_id,
        window_label.as_deref(),
    );
    let result = run_blocking("app_manager_uninstall", move || {
        uninstall_managed_app(&app, input)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_uninstall", &request_id, started_at),
        Err(error) => command_end_error("app_manager_uninstall", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn app_manager_open_uninstall_help(
    app: tauri::AppHandle,
    app_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, crate::core::AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_manager_open_uninstall_help",
        &request_id,
        window_label.as_deref(),
    );
    let result = run_blocking("app_manager_open_uninstall_help", move || {
        open_uninstall_help(&app, app_id)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("app_manager_open_uninstall_help", &request_id, started_at),
        Err(error) => command_end_error(
            "app_manager_open_uninstall_help",
            &request_id,
            started_at,
            error,
        ),
    }
    result
}

#[tauri::command]
pub fn app_manager_reveal_path(
    path: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<()> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_manager_reveal_path",
        &request_id,
        window_label.as_deref(),
    );

    let trimmed = path.trim();
    if trimmed.is_empty() {
        let error = AppError::new("app_manager_reveal_invalid", "定位失败：路径不能为空");
        command_end_error("app_manager_reveal_path", &request_id, started_at, &error);
        return Err(error);
    }

    let result = reveal_path(Path::new(trimmed));
    match &result {
        Ok(_) => command_end_ok("app_manager_reveal_path", &request_id, started_at),
        Err(error) => command_end_error("app_manager_reveal_path", &request_id, started_at, error),
    }
    result
}

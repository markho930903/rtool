use crate::app::state::AppState;
use crate::host::launcher::TauriLauncherHost;
use crate::shared::command_runtime::run_command_sync;
use rtool_app::AppManagerApplicationService;
use rtool_contracts::models::{
    AppManagerActionResultDto, AppManagerCleanupInputDto, AppManagerCleanupResultDto,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerPageDto, AppManagerQueryDto, AppManagerResidueScanInputDto,
    AppManagerResidueScanResultDto, AppManagerResolveSizesInputDto,
    AppManagerResolveSizesResultDto, AppManagerSnapshotMetaDto, AppManagerStartupUpdateInputDto,
    AppManagerUninstallInputDto, ManagedAppDetailDto,
};
use rtool_contracts::{AppError, AppResult, InvokeError};
use std::path::Path;
use tauri::State;

use super::reveal::reveal_path;
use super::runtime::run_app_manager_command;
use super::watcher::trigger_app_manager_watcher_refresh;

async fn run_operation<T, F>(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
    command_name: &'static str,
    operation: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    F: FnOnce(AppManagerApplicationService, TauriLauncherHost) -> AppResult<T> + Send + 'static,
{
    run_app_manager_command(
        app,
        state.app_services.app_manager,
        state.runtime_orchestrator.clone(),
        request_id,
        window_label,
        command_name,
        operation,
    )
    .await
}

async fn run_operation_and_refresh<T, F>(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
    command_name: &'static str,
    operation: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    F: FnOnce(AppManagerApplicationService, TauriLauncherHost) -> AppResult<T> + Send + 'static,
{
    let result = run_operation(
        app,
        state,
        request_id,
        window_label,
        command_name,
        operation,
    )
    .await;
    if result.is_ok() {
        trigger_app_manager_watcher_refresh();
    }
    result
}

pub async fn app_manager_list(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    query: Option<AppManagerQueryDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerPageDto, InvokeError> {
    let input_query = query.unwrap_or_default();
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_list",
        move |service, host| service.list(&host, input_query),
    )
    .await
}

pub async fn app_manager_get_detail(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    query: AppManagerDetailQueryDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ManagedAppDetailDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_get_detail",
        move |service, host| service.get_detail(&host, query),
    )
    .await
}

pub async fn app_manager_list_snapshot_meta(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerSnapshotMetaDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_list_snapshot_meta",
        move |service, host| service.list_snapshot_meta(&host),
    )
    .await
}

pub async fn app_manager_resolve_sizes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerResolveSizesInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerResolveSizesResultDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_resolve_sizes",
        move |service, host| service.resolve_sizes(&host, input),
    )
    .await
}

pub async fn app_manager_get_detail_core(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    query: AppManagerDetailQueryDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ManagedAppDetailDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_get_detail_core",
        move |service, host| service.get_detail_core(&host, query),
    )
    .await
}

pub async fn app_manager_get_detail_heavy(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerResidueScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerResidueScanResultDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_get_detail_heavy",
        move |service, host| service.get_detail_heavy(&host, input),
    )
    .await
}

pub async fn app_manager_scan_residue(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerResidueScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerResidueScanResultDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_scan_residue",
        move |service, host| service.scan_residue(&host, input),
    )
    .await
}

pub async fn app_manager_cleanup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerCleanupInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerCleanupResultDto, InvokeError> {
    run_operation_and_refresh(
        app,
        state,
        request_id,
        window_label,
        "app_manager_cleanup",
        move |service, host| service.cleanup(&host, input),
    )
    .await
}

pub async fn app_manager_export_scan_result(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerExportScanInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerExportScanResultDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_export_scan_result",
        move |service, host| service.export_scan_result(&host, input),
    )
    .await
}

pub async fn app_manager_refresh_index(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    run_operation_and_refresh(
        app,
        state,
        request_id,
        window_label,
        "app_manager_refresh_index",
        move |service, host| service.refresh_index(&host),
    )
    .await
}

pub async fn app_manager_set_startup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerStartupUpdateInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    run_operation_and_refresh(
        app,
        state,
        request_id,
        window_label,
        "app_manager_set_startup",
        move |service, host| service.set_startup(&host, input),
    )
    .await
}

pub async fn app_manager_uninstall(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AppManagerUninstallInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    run_operation_and_refresh(
        app,
        state,
        request_id,
        window_label,
        "app_manager_uninstall",
        move |service, host| service.uninstall(&host, input),
    )
    .await
}

pub async fn app_manager_open_uninstall_help(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    app_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_open_uninstall_help",
        move |service, host| service.open_uninstall_help(&host, app_id),
    )
    .await
}

pub async fn app_manager_open_permission_help(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    app_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppManagerActionResultDto, InvokeError> {
    run_operation(
        app,
        state,
        request_id,
        window_label,
        "app_manager_open_permission_help",
        move |service, host| service.open_permission_help(&host, app_id),
    )
    .await
}

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

use super::run_blocking_command;
use crate::command_runtime::run_command_async;
use crate::host::launcher::TauriLauncherHost;
use app_core::InvokeError;
use app_core::models::{
    ActionResultDto, LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto,
    LauncherRebuildResultDto, LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto,
    ResourceModuleIdDto,
};
use app_launcher_app::launcher::index::{
    get_index_status_async, get_search_settings_async, rebuild_index_now_async,
    reset_search_settings_async, update_search_settings_async,
};
use app_launcher_app::launcher::service::{
    LauncherSearchDiagnostics, execute_launcher_action, search_launcher_async,
};
use tauri::State;

use crate::app::state::AppState;

fn record_launcher_diagnostics(diagnostics: &LauncherSearchDiagnostics) {
    if let Some(duration_ms) = diagnostics.index_query_duration_ms {
        app_resource_monitor::record_module_observation(
            ResourceModuleIdDto::LauncherIndex,
            !diagnostics.index_failed,
            duration_ms,
        );
    }
    if let Some(duration_ms) = diagnostics.fallback_scan_duration_ms {
        app_resource_monitor::record_module_observation(
            ResourceModuleIdDto::LauncherFallback,
            true,
            duration_ms,
        );
    }
    if let Some(duration_ms) = diagnostics.cache_refresh_duration_ms {
        app_resource_monitor::record_module_observation(
            ResourceModuleIdDto::LauncherCache,
            true,
            duration_ms,
        );
    }
}

#[tauri::command]
pub async fn launcher_search(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    query: String,
    limit: Option<u16>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<LauncherItemDto>, InvokeError> {
    let db_conn = state.db_conn.clone();
    let host = TauriLauncherHost::new(app);
    run_command_async(
        "launcher_search",
        request_id,
        window_label,
        move || async move {
            let (items, diagnostics) = search_launcher_async(&host, &db_conn, &query, limit).await;
            record_launcher_diagnostics(&diagnostics);
            Ok::<_, InvokeError>(items)
        },
    )
    .await
}

#[tauri::command]
pub async fn launcher_execute(
    app: tauri::AppHandle,
    action: LauncherActionDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ActionResultDto, InvokeError> {
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "launcher_execute",
        request_id,
        window_label,
        "launcher_execute",
        move || {
            let message = execute_launcher_action(&host, &action)?;
            Ok(ActionResultDto { ok: true, message })
        },
    )
    .await
}

#[tauri::command]
pub async fn launcher_get_search_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherSearchSettingsDto, InvokeError> {
    let db_conn = state.db_conn.clone();
    run_command_async(
        "launcher_get_search_settings",
        request_id,
        window_label,
        move || async move { get_search_settings_async(&db_conn).await },
    )
    .await
}

#[tauri::command]
pub async fn launcher_update_search_settings(
    state: State<'_, AppState>,
    input: LauncherUpdateSearchSettingsInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherSearchSettingsDto, InvokeError> {
    let db_conn = state.db_conn.clone();
    run_command_async(
        "launcher_update_search_settings",
        request_id,
        window_label,
        move || async move { update_search_settings_async(&db_conn, input).await },
    )
    .await
}

#[tauri::command]
pub async fn launcher_get_index_status(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherIndexStatusDto, InvokeError> {
    let db_conn = state.db_conn.clone();
    run_command_async(
        "launcher_get_index_status",
        request_id,
        window_label,
        move || async move { get_index_status_async(&db_conn).await },
    )
    .await
}

#[tauri::command]
pub async fn launcher_rebuild_index(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherRebuildResultDto, InvokeError> {
    let db_conn = state.db_conn.clone();
    run_command_async(
        "launcher_rebuild_index",
        request_id,
        window_label,
        move || async move { rebuild_index_now_async(&db_conn).await },
    )
    .await
}

#[tauri::command]
pub async fn launcher_reset_search_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherSearchSettingsDto, InvokeError> {
    let db_conn = state.db_conn.clone();
    run_command_async(
        "launcher_reset_search_settings",
        request_id,
        window_label,
        move || async move { reset_search_settings_async(&db_conn).await },
    )
    .await
}

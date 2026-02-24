use super::run_blocking_command;
use crate::command_runtime::run_command_async;
use crate::host::launcher::TauriLauncherHost;
use app_core::InvokeError;
use app_core::models::{
    ActionResultDto, LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto,
    LauncherRebuildResultDto, LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto,
};
use app_launcher_app::launcher::index::{
    get_index_status_async, get_search_settings_async, rebuild_index_now_async,
    update_search_settings_async,
};
use app_launcher_app::launcher::service::{execute_launcher_action, search_launcher};
use tauri::State;

use crate::app::state::AppState;

#[tauri::command]
pub async fn launcher_search(
    app: tauri::AppHandle,
    query: String,
    limit: Option<u16>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<LauncherItemDto>, InvokeError> {
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "launcher_search",
        request_id,
        window_label,
        "launcher_search",
        move || Ok(search_launcher(&host, &query, limit)),
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

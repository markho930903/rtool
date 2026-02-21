use super::run_blocking_command;
use crate::app::launcher_index_service::{
    get_index_status, get_search_settings, rebuild_index_now, update_search_settings,
};
use crate::app::launcher_service::{execute_launcher_action, search_launcher};
use crate::core::InvokeError;
use crate::core::models::{
    ActionResultDto, LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto,
    LauncherRebuildResultDto, LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto,
};
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
    run_blocking_command(
        "launcher_search",
        request_id,
        window_label,
        "launcher_search",
        move || Ok(search_launcher(&app, &query, limit)),
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
    run_blocking_command(
        "launcher_execute",
        request_id,
        window_label,
        "launcher_execute",
        move || {
            let message = execute_launcher_action(&app, &action)?;
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
    let db_pool = state.db_pool.clone();
    run_blocking_command(
        "launcher_get_search_settings",
        request_id,
        window_label,
        "launcher_get_search_settings",
        move || get_search_settings(&db_pool),
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
    let db_pool = state.db_pool.clone();
    run_blocking_command(
        "launcher_update_search_settings",
        request_id,
        window_label,
        "launcher_update_search_settings",
        move || update_search_settings(&db_pool, input),
    )
    .await
}

#[tauri::command]
pub async fn launcher_get_index_status(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherIndexStatusDto, InvokeError> {
    let db_pool = state.db_pool.clone();
    run_blocking_command(
        "launcher_get_index_status",
        request_id,
        window_label,
        "launcher_get_index_status",
        move || get_index_status(&db_pool),
    )
    .await
}

#[tauri::command]
pub async fn launcher_rebuild_index(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherRebuildResultDto, InvokeError> {
    let db_pool = state.db_pool.clone();
    run_blocking_command(
        "launcher_rebuild_index",
        request_id,
        window_label,
        "launcher_rebuild_index",
        move || rebuild_index_now(&db_pool),
    )
    .await
}

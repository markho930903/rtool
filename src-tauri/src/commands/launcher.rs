use super::{command_end_error, command_end_ok, command_start, normalize_request_id};
use crate::app::launcher_index_service::{
    get_index_status, get_search_settings, rebuild_index_now, update_search_settings,
};
use crate::app::launcher_service::{execute_launcher_action, search_launcher};
use crate::core::InvokeError;
use crate::core::models::{
    ActionResultDto, LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto,
    LauncherRebuildResultDto, LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto,
};
use crate::infrastructure::runtime::blocking::run_blocking;
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
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("launcher_search", &request_id, window_label.as_deref());
    let result = run_blocking("launcher_search", move || {
        Ok(search_launcher(&app, &query, limit))
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("launcher_search", &request_id, started_at),
        Err(error) => command_end_error("launcher_search", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn launcher_execute(
    app: tauri::AppHandle,
    action: LauncherActionDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ActionResultDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("launcher_execute", &request_id, window_label.as_deref());
    let result = run_blocking("launcher_execute", move || {
        let message = execute_launcher_action(&app, &action)?;
        Ok(ActionResultDto { ok: true, message })
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("launcher_execute", &request_id, started_at),
        Err(error) => command_end_error("launcher_execute", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn launcher_get_search_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherSearchSettingsDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "launcher_get_search_settings",
        &request_id,
        window_label.as_deref(),
    );
    let db_pool = state.db_pool.clone();
    let result = run_blocking("launcher_get_search_settings", move || {
        get_search_settings(&db_pool)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("launcher_get_search_settings", &request_id, started_at),
        Err(error) => command_end_error(
            "launcher_get_search_settings",
            &request_id,
            started_at,
            error,
        ),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn launcher_update_search_settings(
    state: State<'_, AppState>,
    input: LauncherUpdateSearchSettingsInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherSearchSettingsDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "launcher_update_search_settings",
        &request_id,
        window_label.as_deref(),
    );
    let db_pool = state.db_pool.clone();
    let result = run_blocking("launcher_update_search_settings", move || {
        update_search_settings(&db_pool, input)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("launcher_update_search_settings", &request_id, started_at),
        Err(error) => command_end_error(
            "launcher_update_search_settings",
            &request_id,
            started_at,
            error,
        ),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn launcher_get_index_status(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherIndexStatusDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "launcher_get_index_status",
        &request_id,
        window_label.as_deref(),
    );
    let db_pool = state.db_pool.clone();
    let result = run_blocking("launcher_get_index_status", move || {
        get_index_status(&db_pool)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("launcher_get_index_status", &request_id, started_at),
        Err(error) => {
            command_end_error("launcher_get_index_status", &request_id, started_at, error)
        }
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn launcher_rebuild_index(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherRebuildResultDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "launcher_rebuild_index",
        &request_id,
        window_label.as_deref(),
    );
    let db_pool = state.db_pool.clone();
    let result = run_blocking("launcher_rebuild_index", move || {
        rebuild_index_now(&db_pool)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("launcher_rebuild_index", &request_id, started_at),
        Err(error) => command_end_error("launcher_rebuild_index", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

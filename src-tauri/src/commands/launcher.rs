use super::{command_end_ok, command_end_status, command_start, normalize_request_id};
use crate::app::launcher_service::{execute_launcher_action, search_launcher};
use crate::core::models::{ActionResultDto, LauncherActionDto, LauncherItemDto};

#[tauri::command]
pub fn launcher_search(
    app: tauri::AppHandle,
    query: String,
    limit: Option<u16>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Vec<LauncherItemDto> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("launcher_search", &request_id, window_label.as_deref());
    let items = search_launcher(&app, &query, limit);
    command_end_ok("launcher_search", &request_id, started_at);
    items
}

#[tauri::command]
pub fn launcher_execute(
    app: tauri::AppHandle,
    action: LauncherActionDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> ActionResultDto {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("launcher_execute", &request_id, window_label.as_deref());
    let result = execute_launcher_action(&app, &action);
    command_end_status(
        "launcher_execute",
        &request_id,
        started_at,
        result.ok,
        Some("launcher_action_failed"),
        Some(&result.message),
    );
    result
}

use super::{command_end_ok, command_end_status, command_start, normalize_request_id};
use crate::app::launcher_service::{execute_launcher_action, search_launcher};
use crate::core::models::{ActionResultDto, LauncherActionDto, LauncherItemDto};
use crate::infrastructure::runtime::blocking::run_blocking;

#[tauri::command]
pub async fn launcher_search(
    app: tauri::AppHandle,
    query: String,
    limit: Option<u16>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Vec<LauncherItemDto> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("launcher_search", &request_id, window_label.as_deref());
    let result = run_blocking("launcher_search", move || {
        Ok(search_launcher(&app, &query, limit))
    })
    .await;
    match result {
        Ok(items) => {
            command_end_ok("launcher_search", &request_id, started_at);
            items
        }
        Err(error) => {
            command_end_status(
                "launcher_search",
                &request_id,
                started_at,
                false,
                Some(error.code.as_str()),
                Some(error.message.as_str()),
            );
            Vec::new()
        }
    }
}

#[tauri::command]
pub async fn launcher_execute(
    app: tauri::AppHandle,
    action: LauncherActionDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> ActionResultDto {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("launcher_execute", &request_id, window_label.as_deref());
    let result = run_blocking("launcher_execute", move || {
        Ok(execute_launcher_action(&app, &action))
    })
    .await
    .unwrap_or_else(|error| ActionResultDto {
        ok: false,
        message: error.message.clone(),
    });
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

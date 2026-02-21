use super::{command_end_error, command_end_ok, command_start, normalize_request_id};
use crate::app::palette_service::{execute_palette_action, search_palette};
use crate::core::InvokeError;
use crate::core::models::{ActionResultDto, PaletteItemDto};

#[tauri::command]
pub fn palette_search(
    app: tauri::AppHandle,
    query: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<PaletteItemDto>, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("palette_search", &request_id, window_label.as_deref());
    let result = Ok(search_palette(&app, &query));
    command_end_ok("palette_search", &request_id, started_at);
    result
}

#[tauri::command]
pub fn palette_execute(
    action_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ActionResultDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("palette_execute", &request_id, window_label.as_deref());
    let result =
        execute_palette_action(&action_id).map(|message| ActionResultDto { ok: true, message });
    match &result {
        Ok(_) => command_end_ok("palette_execute", &request_id, started_at),
        Err(error) => command_end_error("palette_execute", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

use super::run_command_sync;
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
    run_command_sync("palette_search", request_id, window_label, move || {
        Ok::<_, InvokeError>(search_palette(&app, &query))
    })
}

#[tauri::command]
pub fn palette_execute(
    action_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ActionResultDto, InvokeError> {
    run_command_sync("palette_execute", request_id, window_label, move || {
        execute_palette_action(&action_id).map(|message| ActionResultDto { ok: true, message })
    })
}

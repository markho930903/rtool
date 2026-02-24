use super::run_command_sync;
use crate::host::launcher::TauriLauncherHost;
use app_core::InvokeError;
use app_core::models::{ActionResultDto, PaletteItemDto};
use app_launcher_app::launcher::palette::{execute_palette_action, search_palette};

#[tauri::command]
pub fn palette_search(
    app: tauri::AppHandle,
    query: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<PaletteItemDto>, InvokeError> {
    let host = TauriLauncherHost::new(app);
    run_command_sync("palette_search", request_id, window_label, move || {
        Ok::<_, InvokeError>(search_palette(&host, &query))
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

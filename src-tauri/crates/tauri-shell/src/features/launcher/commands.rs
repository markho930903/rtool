use super::run_blocking_command;
use crate::command_runtime::run_command_async;
use crate::host::launcher::TauriLauncherHost;
use app_core::InvokeError;
use app_core::models::{
    ActionResultDto, LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto,
    LauncherRebuildResultDto, LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto,
};
use tauri::State;

use crate::app::state::AppState;

#[tauri::command]
pub async fn launcher_search(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    query: String,
    limit: Option<u16>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<LauncherItemDto>, InvokeError> {
    let launcher_service = state.app_services.launcher.clone();
    let host = TauriLauncherHost::new(app);
    run_command_async(
        "launcher_search",
        request_id,
        window_label,
        move || async move {
            Ok::<_, InvokeError>(launcher_service.search(&host, &query, limit).await)
        },
    )
    .await
}

#[tauri::command]
pub async fn launcher_execute(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    action: LauncherActionDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ActionResultDto, InvokeError> {
    let launcher_service = state.app_services.launcher.clone();
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        "launcher_execute",
        request_id,
        window_label,
        "launcher_execute",
        move || {
            let message = launcher_service.execute(&host, &action)?;
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
    let launcher_service = state.app_services.launcher.clone();
    run_command_async(
        "launcher_get_search_settings",
        request_id,
        window_label,
        move || async move { launcher_service.get_search_settings().await },
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
    let launcher_service = state.app_services.launcher.clone();
    run_command_async(
        "launcher_update_search_settings",
        request_id,
        window_label,
        move || async move { launcher_service.update_search_settings(input).await },
    )
    .await
}

#[tauri::command]
pub async fn launcher_get_index_status(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherIndexStatusDto, InvokeError> {
    let launcher_service = state.app_services.launcher.clone();
    run_command_async(
        "launcher_get_index_status",
        request_id,
        window_label,
        move || async move { launcher_service.get_index_status().await },
    )
    .await
}

#[tauri::command]
pub async fn launcher_rebuild_index(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherRebuildResultDto, InvokeError> {
    let launcher_service = state.app_services.launcher.clone();
    run_command_async(
        "launcher_rebuild_index",
        request_id,
        window_label,
        move || async move { launcher_service.rebuild_index().await },
    )
    .await
}

#[tauri::command]
pub async fn launcher_reset_search_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LauncherSearchSettingsDto, InvokeError> {
    let launcher_service = state.app_services.launcher.clone();
    run_command_async(
        "launcher_reset_search_settings",
        request_id,
        window_label,
        move || async move { launcher_service.reset_search_settings().await },
    )
    .await
}

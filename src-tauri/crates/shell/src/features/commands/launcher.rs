use super::run_blocking_command;
use crate::command_runtime::run_command_async;
use crate::features::command_payload::{
    CommandPayloadContext, CommandRequestDto,
};
use crate::host::launcher::TauriLauncherHost;
use protocol::InvokeError;
use protocol::models::{
    ActionResultDto, LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto,
    LauncherRebuildResultDto, LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::State;

use crate::app::state::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherSearchPayload {
    query: String,
    limit: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherExecutePayload {
    action: LauncherActionDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherUpdateSettingsPayload {
    input: LauncherUpdateSearchSettingsInputDto,
}

const LAUNCHER_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "launcher",
    "启动器命令参数无效",
    "启动器命令返回序列化失败",
    "未知启动器命令",
);

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

#[tauri::command]
pub async fn launcher_handle(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "search" => {
            let payload: LauncherSearchPayload =
                LAUNCHER_COMMAND_CONTEXT.parse("search", request.payload)?;
            LAUNCHER_COMMAND_CONTEXT.serialize(
                "search",
                launcher_search(
                    app,
                    state,
                    payload.query,
                    payload.limit,
                    request_id,
                    window_label,
                )
                .await?,
            )
        }
        "execute" => {
            let payload: LauncherExecutePayload =
                LAUNCHER_COMMAND_CONTEXT.parse("execute", request.payload)?;
            LAUNCHER_COMMAND_CONTEXT.serialize(
                "execute",
                launcher_execute(app, state, payload.action, request_id, window_label).await?,
            )
        }
        "get_search_settings" => LAUNCHER_COMMAND_CONTEXT.serialize(
            "get_search_settings",
            launcher_get_search_settings(state, request_id, window_label).await?,
        ),
        "update_search_settings" => {
            let payload: LauncherUpdateSettingsPayload =
                LAUNCHER_COMMAND_CONTEXT.parse("update_search_settings", request.payload)?;
            LAUNCHER_COMMAND_CONTEXT.serialize(
                "update_search_settings",
                launcher_update_search_settings(state, payload.input, request_id, window_label)
                    .await?,
            )
        }
        "get_index_status" => LAUNCHER_COMMAND_CONTEXT.serialize(
            "get_index_status",
            launcher_get_index_status(state, request_id, window_label).await?,
        ),
        "rebuild_index" => LAUNCHER_COMMAND_CONTEXT.serialize(
            "rebuild_index",
            launcher_rebuild_index(state, request_id, window_label).await?,
        ),
        "reset_search_settings" => LAUNCHER_COMMAND_CONTEXT.serialize(
            "reset_search_settings",
            launcher_reset_search_settings(state, request_id, window_label).await?,
        ),
        _ => Err(LAUNCHER_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

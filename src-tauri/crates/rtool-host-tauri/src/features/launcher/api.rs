use std::future::Future;

use crate::host::launcher::TauriLauncherHost;
use crate::shared::command_response::CommandPayloadContext;
use crate::shared::command_runtime::{run_blocking_command, run_command_async};
use crate::shared::request_context::InvokeMeta;
use rtool_contracts::models::{
    ActionResultDto, LauncherActionDto, LauncherUpdateSearchSettingsInputDto,
};
use rtool_contracts::{AppResult, InvokeError};
use serde::Deserialize;
use serde_json::Value;
use tauri::State;

use crate::app::state::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LauncherSearchPayload {
    query: String,
    limit: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LauncherExecutePayload {
    action: LauncherActionDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LauncherUpdateSettingsPayload {
    input: LauncherUpdateSearchSettingsInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub(crate) enum LauncherRequest {
    Search(LauncherSearchPayload),
    Execute(LauncherExecutePayload),
    GetSearchSettings,
    UpdateSearchSettings(LauncherUpdateSettingsPayload),
    GetStatus,
    RebuildIndex,
    ResetSearchSettings,
}

const LAUNCHER_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "launcher",
    "启动器命令参数无效",
    "启动器命令返回序列化失败",
    "未知启动器命令",
);

fn request_kind(request: &LauncherRequest) -> &'static str {
    match request {
        LauncherRequest::Search(_) => "search",
        LauncherRequest::Execute(_) => "execute",
        LauncherRequest::GetSearchSettings => "get_search_settings",
        LauncherRequest::UpdateSearchSettings(_) => "update_search_settings",
        LauncherRequest::GetStatus => "get_status",
        LauncherRequest::RebuildIndex => "rebuild_index",
        LauncherRequest::ResetSearchSettings => "reset_search_settings",
    }
}

fn request_command_name(request: &LauncherRequest) -> &'static str {
    match request {
        LauncherRequest::Search(_) => "launcher_search",
        LauncherRequest::Execute(_) => "launcher_execute",
        LauncherRequest::GetSearchSettings => "launcher_get_search_settings",
        LauncherRequest::UpdateSearchSettings(_) => "launcher_update_search_settings",
        LauncherRequest::GetStatus => "launcher_get_status",
        LauncherRequest::RebuildIndex => "launcher_rebuild_index",
        LauncherRequest::ResetSearchSettings => "launcher_reset_search_settings",
    }
}

async fn run_launcher_async<T, Fut, F>(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
    command_name: &'static str,
    operation: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    Fut: Future<Output = AppResult<T>>,
    F: FnOnce(rtool_app::LauncherApplicationService) -> Fut,
{
    let launcher_service = state.app_services.launcher.clone();
    run_command_async(command_name, request_id, window_label, move || {
        operation(launcher_service)
    })
    .await
}

async fn run_launcher_with_host_async<T, Fut, F>(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
    command_name: &'static str,
    operation: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    Fut: Future<Output = AppResult<T>>,
    F: FnOnce(rtool_app::LauncherApplicationService, TauriLauncherHost) -> Fut,
{
    let host = TauriLauncherHost::new(app);
    run_launcher_async(
        state,
        request_id,
        window_label,
        command_name,
        move |launcher_service| operation(launcher_service, host),
    )
    .await
}

async fn run_launcher_with_host_blocking<T, F>(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
    command_name: &'static str,
    operation: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    F: FnOnce(rtool_app::LauncherApplicationService, TauriLauncherHost) -> AppResult<T>
        + Send
        + 'static,
{
    let launcher_service = state.app_services.launcher.clone();
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        command_name,
        request_id,
        window_label,
        command_name,
        move || operation(launcher_service, host),
    )
    .await
}

pub(crate) async fn handle_launcher(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: LauncherRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    let (request_id, window_label) = meta.unwrap_or_default().split();
    let kind = request_kind(&request);
    let command_name = request_command_name(&request);

    match request {
        LauncherRequest::Search(payload) => LAUNCHER_COMMAND_CONTEXT.serialize(
            kind,
            run_launcher_with_host_async(
                app,
                state,
                request_id,
                window_label,
                command_name,
                move |launcher_service, host| async move {
                    Ok(launcher_service
                        .search(&host, &payload.query, payload.limit)
                        .await)
                },
            )
            .await?,
        ),
        LauncherRequest::Execute(payload) => LAUNCHER_COMMAND_CONTEXT.serialize(
            kind,
            run_launcher_with_host_blocking(
                app,
                state,
                request_id,
                window_label,
                command_name,
                move |launcher_service, host| {
                    let message = launcher_service.execute(&host, &payload.action)?;
                    Ok(ActionResultDto { ok: true, message })
                },
            )
            .await?,
        ),
        LauncherRequest::GetSearchSettings => LAUNCHER_COMMAND_CONTEXT.serialize(
            kind,
            run_launcher_async(
                state,
                request_id,
                window_label,
                command_name,
                move |launcher_service| async move { launcher_service.get_search_settings().await },
            )
            .await?,
        ),
        LauncherRequest::UpdateSearchSettings(payload) => LAUNCHER_COMMAND_CONTEXT.serialize(
            kind,
            run_launcher_async(
                state,
                request_id,
                window_label,
                command_name,
                move |launcher_service| async move {
                    launcher_service.update_search_settings(payload.input).await
                },
            )
            .await?,
        ),
        LauncherRequest::GetStatus => LAUNCHER_COMMAND_CONTEXT.serialize(
            kind,
            run_launcher_async(
                state,
                request_id,
                window_label,
                command_name,
                move |launcher_service| async move { launcher_service.get_status().await },
            )
            .await?,
        ),
        LauncherRequest::RebuildIndex => LAUNCHER_COMMAND_CONTEXT.serialize(
            kind,
            run_launcher_async(
                state,
                request_id,
                window_label,
                command_name,
                move |launcher_service| async move { launcher_service.rebuild_index().await },
            )
            .await?,
        ),
        LauncherRequest::ResetSearchSettings => {
            LAUNCHER_COMMAND_CONTEXT.serialize(
                kind,
                run_launcher_async(
                    state,
                    request_id,
                    window_label,
                    command_name,
                    move |launcher_service| async move {
                        launcher_service.reset_search_settings().await
                    },
                )
                .await?,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_kind_maps_every_variant() {
        assert_eq!(
            request_kind(&LauncherRequest::Search(LauncherSearchPayload {
                query: "a".to_string(),
                limit: Some(1),
            })),
            "search"
        );
        assert_eq!(
            request_kind(&LauncherRequest::Execute(LauncherExecutePayload {
                action: LauncherActionDto::OpenBuiltinRoute {
                    route: "/tools".to_string(),
                },
            })),
            "execute"
        );
        assert_eq!(
            request_kind(&LauncherRequest::GetSearchSettings),
            "get_search_settings"
        );
        assert_eq!(
            request_kind(&LauncherRequest::UpdateSearchSettings(
                LauncherUpdateSettingsPayload {
                    input: LauncherUpdateSearchSettingsInputDto::default(),
                }
            )),
            "update_search_settings"
        );
        assert_eq!(request_kind(&LauncherRequest::GetStatus), "get_status");
        assert_eq!(
            request_kind(&LauncherRequest::RebuildIndex),
            "rebuild_index"
        );
        assert_eq!(
            request_kind(&LauncherRequest::ResetSearchSettings),
            "reset_search_settings"
        );
    }

    #[test]
    fn request_command_name_maps_every_variant() {
        assert_eq!(
            request_command_name(&LauncherRequest::Search(LauncherSearchPayload {
                query: "a".to_string(),
                limit: None,
            })),
            "launcher_search"
        );
        assert_eq!(
            request_command_name(&LauncherRequest::Execute(LauncherExecutePayload {
                action: LauncherActionDto::OpenBuiltinRoute {
                    route: "/settings".to_string(),
                },
            })),
            "launcher_execute"
        );
        assert_eq!(
            request_command_name(&LauncherRequest::GetSearchSettings),
            "launcher_get_search_settings"
        );
        assert_eq!(
            request_command_name(&LauncherRequest::UpdateSearchSettings(
                LauncherUpdateSettingsPayload {
                    input: LauncherUpdateSearchSettingsInputDto::default(),
                }
            )),
            "launcher_update_search_settings"
        );
        assert_eq!(
            request_command_name(&LauncherRequest::GetStatus),
            "launcher_get_status"
        );
        assert_eq!(
            request_command_name(&LauncherRequest::RebuildIndex),
            "launcher_rebuild_index"
        );
        assert_eq!(
            request_command_name(&LauncherRequest::ResetSearchSettings),
            "launcher_reset_search_settings"
        );
    }
}

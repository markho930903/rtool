use super::run_command_sync;
use crate::features::command_payload::{
    CommandPayloadContext, CommandRequestDto,
};
use protocol::InvokeError;
use protocol::models::{ActionResultDto, ResourceHistoryDto, ResourceSnapshotDto};
use rtool_core::ResourceMonitorApplicationService;
use serde::Deserialize;
use serde_json::Value;

const RESOURCE_MONITOR_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "resource_monitor",
    "资源监控命令参数无效",
    "资源监控命令返回序列化失败",
    "未知资源监控命令",
);

#[tauri::command]
pub fn resource_monitor_snapshot(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ResourceSnapshotDto, InvokeError> {
    let service = ResourceMonitorApplicationService;
    run_command_sync(
        "resource_monitor_snapshot",
        request_id,
        window_label,
        move || Ok::<_, InvokeError>(service.snapshot()),
    )
}

#[tauri::command]
pub fn resource_monitor_history(
    limit: Option<u32>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ResourceHistoryDto, InvokeError> {
    let service = ResourceMonitorApplicationService;
    run_command_sync(
        "resource_monitor_history",
        request_id,
        window_label,
        move || Ok::<_, InvokeError>(service.history(limit)),
    )
}

#[tauri::command]
pub fn resource_monitor_reset_session(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ActionResultDto, InvokeError> {
    let service = ResourceMonitorApplicationService;
    run_command_sync(
        "resource_monitor_reset_session",
        request_id,
        window_label,
        move || Ok::<_, InvokeError>(service.reset_session()),
    )
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ResourceHistoryPayload {
    limit: Option<u32>,
}

#[tauri::command]
pub fn resource_monitor_handle(
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "snapshot" => RESOURCE_MONITOR_COMMAND_CONTEXT.serialize(
            "snapshot",
            resource_monitor_snapshot(request_id, window_label)?,
        ),
        "history" => {
            let payload: ResourceHistoryPayload =
                RESOURCE_MONITOR_COMMAND_CONTEXT.parse("history", request.payload)?;
            RESOURCE_MONITOR_COMMAND_CONTEXT.serialize(
                "history",
                resource_monitor_history(payload.limit, request_id, window_label)?,
            )
        }
        "reset_session" => RESOURCE_MONITOR_COMMAND_CONTEXT.serialize(
            "reset_session",
            resource_monitor_reset_session(request_id, window_label)?,
        ),
        _ => Err(RESOURCE_MONITOR_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

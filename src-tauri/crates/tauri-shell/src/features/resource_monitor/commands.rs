use super::run_command_sync;
use app_application::ResourceMonitorApplicationService;
use app_core::InvokeError;
use app_core::models::{ActionResultDto, ResourceHistoryDto, ResourceSnapshotDto};

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

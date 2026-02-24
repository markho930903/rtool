use super::run_command_sync;
use app_core::InvokeError;
use app_core::models::{ActionResultDto, ResourceHistoryDto, ResourceSnapshotDto};

#[tauri::command]
pub fn resource_monitor_snapshot(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ResourceSnapshotDto, InvokeError> {
    run_command_sync(
        "resource_monitor_snapshot",
        request_id,
        window_label,
        move || Ok::<_, InvokeError>(app_resource_monitor::snapshot()),
    )
}

#[tauri::command]
pub fn resource_monitor_history(
    limit: Option<u32>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ResourceHistoryDto, InvokeError> {
    run_command_sync(
        "resource_monitor_history",
        request_id,
        window_label,
        move || {
            let limit = limit
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value > 0);
            Ok::<_, InvokeError>(app_resource_monitor::history(limit))
        },
    )
}

#[tauri::command]
pub fn resource_monitor_reset_session(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ActionResultDto, InvokeError> {
    run_command_sync(
        "resource_monitor_reset_session",
        request_id,
        window_label,
        move || {
            app_resource_monitor::reset_session();
            Ok::<_, InvokeError>(ActionResultDto {
                ok: true,
                message: "resource monitor session reset".to_string(),
            })
        },
    )
}

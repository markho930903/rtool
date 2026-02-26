use tauri::{AppHandle, Emitter};

const RESOURCE_MONITOR_TICK_EVENT: &str = "rtool://resource-monitor/tick";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMonitorTickPayload {
    pub sampled_at: i64,
}

pub fn emit_tick(app_handle: &AppHandle, sampled_at: i64) {
    if let Err(error) = app_handle.emit(
        RESOURCE_MONITOR_TICK_EVENT,
        ResourceMonitorTickPayload { sampled_at },
    ) {
        tracing::warn!(
            event = "resource_monitor_tick_emit_failed",
            detail = error.to_string()
        );
    }
}

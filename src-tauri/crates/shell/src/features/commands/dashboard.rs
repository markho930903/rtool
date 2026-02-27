use super::run_blocking_command;
use crate::app::state::AppState;
use crate::features::command_payload::{CommandPayloadContext, CommandRequestDto};
use protocol::InvokeError;
use protocol::models::{AppHealthSnapshotDto, DashboardSnapshotDto};
use serde_json::Value;
use tauri::State;

const DASHBOARD_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "dashboard",
    "仪表盘命令参数无效",
    "仪表盘命令返回序列化失败",
    "未知仪表盘命令",
);

#[tauri::command]
pub async fn dashboard_snapshot(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<DashboardSnapshotDto, InvokeError> {
    let uptime_seconds = state.started_at().elapsed().as_secs();
    let db_path = state.db_path.clone();
    let service = state.app_services.dashboard;
    let app_name = env!("CARGO_PKG_NAME").to_string();
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let build_mode = if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "release".to_string()
    };
    run_blocking_command(
        "dashboard_snapshot",
        request_id,
        window_label,
        "dashboard_snapshot",
        move || service.snapshot(app_name, app_version, build_mode, uptime_seconds, db_path),
    )
    .await
}

#[tauri::command]
pub async fn app_get_health_snapshot(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<AppHealthSnapshotDto, InvokeError> {
    let transfer_service = state.app_services.transfer.clone();
    let dashboard_service = state.app_services.dashboard;
    run_blocking_command(
        "app_get_health_snapshot",
        request_id,
        window_label,
        "app_get_health_snapshot",
        move || dashboard_service.health_snapshot(transfer_service.runtime_status()),
    )
    .await
}

#[tauri::command]
pub async fn dashboard_handle(
    state: State<'_, AppState>,
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "snapshot" => DASHBOARD_COMMAND_CONTEXT.serialize(
            "snapshot",
            dashboard_snapshot(state, request_id, window_label).await?,
        ),
        "health_snapshot" => DASHBOARD_COMMAND_CONTEXT.serialize(
            "health_snapshot",
            app_get_health_snapshot(state, request_id, window_label).await?,
        ),
        _ => Err(DASHBOARD_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

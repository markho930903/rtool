use crate::command_runtime::{run_command_async, run_command_sync};
use protocol::InvokeError;
use protocol::models::{LogConfigDto, LogPageDto, LogQueryDto};
use rtool_core::LoggingApplicationService;
use serde_json::Value;

#[tauri::command]
pub async fn client_log(
    level: String,
    request_id: Option<String>,
    scope: String,
    message: String,
    metadata: Option<Value>,
) -> Result<(), InvokeError> {
    LoggingApplicationService
        .client_log(level, request_id, scope, message, metadata)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn logging_query(
    query: Option<LogQueryDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LogPageDto, InvokeError> {
    let normalized = query.unwrap_or_default();
    let service = LoggingApplicationService;
    run_command_async(
        "logging_query",
        request_id,
        window_label,
        move || async move { service.query(normalized).await },
    )
    .await
}

#[tauri::command]
pub async fn logging_get_config(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LogConfigDto, InvokeError> {
    let service = LoggingApplicationService;
    run_command_sync("logging_get_config", request_id, window_label, move || {
        service.get_config()
    })
}

#[tauri::command]
pub async fn logging_update_config(
    config: LogConfigDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LogConfigDto, InvokeError> {
    let service = LoggingApplicationService;
    run_command_async(
        "logging_update_config",
        request_id,
        window_label,
        move || async move { service.update_config(config).await },
    )
    .await
}

#[tauri::command]
pub async fn logging_export_jsonl(
    query: Option<LogQueryDto>,
    output_path: Option<String>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<String, InvokeError> {
    let normalized = query.unwrap_or_default();
    let service = LoggingApplicationService;
    run_command_async(
        "logging_export_jsonl",
        request_id,
        window_label,
        move || async move { service.export_jsonl(normalized, output_path).await },
    )
    .await
}

#[cfg(test)]
#[path = "../../../tests/logging_tests.inc"]
mod tests;

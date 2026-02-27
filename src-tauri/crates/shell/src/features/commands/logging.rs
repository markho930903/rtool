use crate::command_runtime::{run_command_async, run_command_sync};
use crate::features::command_payload::{
    CommandPayloadContext, CommandRequestDto,
};
use protocol::InvokeError;
use protocol::models::{LogConfigDto, LogPageDto, LogQueryDto};
use rtool_core::LoggingApplicationService;
use serde::Deserialize;
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


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientLogPayload {
    level: String,
    scope: String,
    message: String,
    metadata: Option<Value>,
    request_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct LoggingQueryPayload {
    query: Option<LogQueryDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoggingConfigPayload {
    config: LogConfigDto,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct LoggingExportPayload {
    query: Option<LogQueryDto>,
    output_path: Option<String>,
}

const LOGGING_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "logging",
    "日志命令参数无效",
    "日志命令返回序列化失败",
    "未知日志命令",
);

#[tauri::command]
pub async fn logging_handle(
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "client_log" => {
            let payload: ClientLogPayload =
                LOGGING_COMMAND_CONTEXT.parse("client_log", request.payload)?;
            client_log(
                payload.level,
                payload.request_id.or(request_id),
                payload.scope,
                payload.message,
                payload.metadata,
            )
            .await?;
            Ok(Value::Null)
        }
        "query" => {
            let payload: LoggingQueryPayload =
                LOGGING_COMMAND_CONTEXT.parse("query", request.payload)?;
            LOGGING_COMMAND_CONTEXT.serialize(
                "query",
                logging_query(payload.query, request_id, window_label).await?,
            )
        }
        "get_config" => LOGGING_COMMAND_CONTEXT.serialize(
            "get_config",
            logging_get_config(request_id, window_label).await?,
        ),
        "update_config" => {
            let payload: LoggingConfigPayload =
                LOGGING_COMMAND_CONTEXT.parse("update_config", request.payload)?;
            LOGGING_COMMAND_CONTEXT.serialize(
                "update_config",
                logging_update_config(payload.config, request_id, window_label).await?,
            )
        }
        "export_jsonl" => {
            let payload: LoggingExportPayload =
                LOGGING_COMMAND_CONTEXT.parse("export_jsonl", request.payload)?;
            LOGGING_COMMAND_CONTEXT.serialize(
                "export_jsonl",
                logging_export_jsonl(payload.query, payload.output_path, request_id, window_label)
                    .await?,
            )
        }
        _ => Err(LOGGING_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

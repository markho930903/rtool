use crate::shared::command_response::CommandPayloadContext;
use crate::shared::command_runtime::{run_command_async, run_command_sync};
use crate::shared::request_context::InvokeMeta;
use rtool_app::LoggingApplicationService;
use rtool_contracts::InvokeError;
use rtool_contracts::models::{LogConfigDto, LogPageDto, LogQueryDto};
use serde::Deserialize;
use serde_json::Value;

async fn client_log(
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

async fn logging_query(
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

async fn logging_get_config(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LogConfigDto, InvokeError> {
    let service = LoggingApplicationService;
    run_command_sync("logging_get_config", request_id, window_label, move || {
        service.get_config()
    })
}

async fn logging_update_config(
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

async fn logging_export_jsonl(
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
pub(crate) struct ClientLogPayload {
    level: String,
    scope: String,
    message: String,
    metadata: Option<Value>,
    request_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct LoggingQueryPayload {
    query: Option<LogQueryDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoggingConfigPayload {
    config: LogConfigDto,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct LoggingExportPayload {
    query: Option<LogQueryDto>,
    output_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub(crate) enum LoggingRequest {
    ClientLog(ClientLogPayload),
    Query(LoggingQueryPayload),
    GetConfig,
    UpdateConfig(LoggingConfigPayload),
    ExportJsonl(LoggingExportPayload),
}

const LOGGING_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "logging",
    "日志命令参数无效",
    "日志命令返回序列化失败",
    "未知日志命令",
);

pub(crate) async fn handle_logging(
    request: LoggingRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    let (request_id, window_label) = meta.unwrap_or_default().split();

    match request {
        LoggingRequest::ClientLog(payload) => {
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
        LoggingRequest::Query(payload) => LOGGING_COMMAND_CONTEXT.serialize(
            "query",
            logging_query(payload.query, request_id, window_label).await?,
        ),
        LoggingRequest::GetConfig => LOGGING_COMMAND_CONTEXT.serialize(
            "get_config",
            logging_get_config(request_id, window_label).await?,
        ),
        LoggingRequest::UpdateConfig(payload) => LOGGING_COMMAND_CONTEXT.serialize(
            "update_config",
            logging_update_config(payload.config, request_id, window_label).await?,
        ),
        LoggingRequest::ExportJsonl(payload) => LOGGING_COMMAND_CONTEXT.serialize(
            "export_jsonl",
            logging_export_jsonl(payload.query, payload.output_path, request_id, window_label)
                .await?,
        ),
    }
}

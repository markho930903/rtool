use super::run_blocking_command;
use app_core::models::{LogConfigDto, LogPageDto, LogQueryDto};
use app_core::{AppError, InvokeError};
use app_infra::logging::{
    RecordLogInput, export_log_entries, get_log_config, record_log_event, sanitize_for_log,
    sanitize_json_value, update_log_config,
};
use app_infra::runtime::blocking::run_blocking;
use serde_json::Value;

const MAX_MESSAGE_LEN: usize = 2048;

fn normalize_level(level: &str) -> Option<&'static str> {
    match level.trim().to_ascii_lowercase().as_str() {
        "trace" => Some("trace"),
        "debug" => Some("debug"),
        "info" => Some("info"),
        "warn" => Some("warn"),
        "error" => Some("error"),
        _ => None,
    }
}

fn normalize_message(message: &str) -> String {
    let truncated = if message.len() > MAX_MESSAGE_LEN {
        let mut compact = String::new();
        for ch in message.chars() {
            if compact.len() + ch.len_utf8() > MAX_MESSAGE_LEN {
                break;
            }
            compact.push(ch);
        }
        format!("{compact}...(truncated,len={})", message.len())
    } else {
        message.to_string()
    };

    sanitize_for_log(&truncated)
}

#[tauri::command]
pub async fn client_log(
    level: String,
    request_id: Option<String>,
    scope: String,
    message: String,
    metadata: Option<Value>,
) -> Result<(), InvokeError> {
    let level = normalize_level(&level).ok_or_else(|| {
        AppError::new("invalid_log_level", "日志级别非法")
            .with_context("level", sanitize_for_log(&level))
    })?;

    let scope = sanitize_for_log(&scope);
    let message = normalize_message(&message);
    let metadata = metadata.map(|value| sanitize_json_value(&value));
    let request_id = request_id.unwrap_or_else(|| "unknown".to_string());

    let record = RecordLogInput {
        level: level.to_string(),
        scope: scope.clone(),
        event: "client_log".to_string(),
        request_id: request_id.clone(),
        window_label: None,
        message: message.clone(),
        metadata: metadata.clone(),
        raw_ref: None,
    };
    let request_id_for_record = request_id.clone();
    run_blocking("client_log_record", move || {
        if let Err(error) = record_log_event(record) {
            tracing::warn!(
                event = "client_log_record_failed",
                request_id = %request_id_for_record,
                error_code = error.code,
                error_detail = error.causes.first().map(String::as_str).unwrap_or_default()
            );
        }
        Ok(())
    })
    .await?;

    match level {
        "trace" => tracing::trace!(
            event = "client_log",
            scope = %scope,
            request_id = %request_id,
            message = %message,
            metadata = ?metadata
        ),
        "debug" => tracing::debug!(
            event = "client_log",
            scope = %scope,
            request_id = %request_id,
            message = %message,
            metadata = ?metadata
        ),
        "info" => tracing::info!(
            event = "client_log",
            scope = %scope,
            request_id = %request_id,
            message = %message,
            metadata = ?metadata
        ),
        "warn" => tracing::warn!(
            event = "client_log",
            scope = %scope,
            request_id = %request_id,
            message = %message,
            metadata = ?metadata
        ),
        "error" => tracing::error!(
            event = "client_log",
            scope = %scope,
            request_id = %request_id,
            message = %message,
            metadata = ?metadata
        ),
        _ => unreachable!("normalize_level should reject unsupported levels"),
    }

    Ok(())
}

#[tauri::command]
pub async fn logging_query(
    query: Option<LogQueryDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LogPageDto, InvokeError> {
    let normalized = query.unwrap_or_default();
    run_blocking_command(
        "logging_query",
        request_id,
        window_label,
        "logging_query",
        move || app_infra::logging::query_log_entries(normalized),
    )
    .await
}

#[tauri::command]
pub async fn logging_get_config(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LogConfigDto, InvokeError> {
    run_blocking_command(
        "logging_get_config",
        request_id,
        window_label,
        "logging_get_config",
        get_log_config,
    )
    .await
}

#[tauri::command]
pub async fn logging_update_config(
    config: LogConfigDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LogConfigDto, InvokeError> {
    run_blocking_command(
        "logging_update_config",
        request_id,
        window_label,
        "logging_update_config",
        move || update_log_config(config),
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
    run_blocking_command(
        "logging_export_jsonl",
        request_id,
        window_label,
        "logging_export_jsonl",
        move || export_log_entries(normalized, output_path),
    )
    .await
}

#[cfg(test)]
#[path = "../../../../../tests/commands/logging_tests.rs"]
mod tests;

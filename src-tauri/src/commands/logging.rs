use super::{command_end_error, command_end_ok, command_start, normalize_request_id};
use crate::core::models::{LogConfigDto, LogPageDto, LogQueryDto};
use crate::core::{AppError, AppResult};
use crate::infrastructure::logging::{
    RecordLogInput, export_log_entries, get_log_config, record_log_event, sanitize_for_log,
    sanitize_json_value, update_log_config,
};
use crate::infrastructure::runtime::blocking::run_blocking;
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
) -> AppResult<()> {
    let level = normalize_level(&level).ok_or_else(|| {
        AppError::new("invalid_log_level", "日志级别非法")
            .with_detail(format!("unsupported level: {}", sanitize_for_log(&level)))
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
                error_detail = error.detail.unwrap_or_default()
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
) -> AppResult<LogPageDto> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("logging_query", &request_id, window_label.as_deref());

    let normalized = query.unwrap_or_default();
    let result = run_blocking("logging_query", move || {
        crate::infrastructure::logging::query_log_entries(normalized)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("logging_query", &request_id, started_at),
        Err(error) => command_end_error("logging_query", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn logging_get_config(
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<LogConfigDto> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("logging_get_config", &request_id, window_label.as_deref());

    let result = run_blocking("logging_get_config", get_log_config).await;
    match &result {
        Ok(_) => command_end_ok("logging_get_config", &request_id, started_at),
        Err(error) => command_end_error("logging_get_config", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn logging_update_config(
    config: LogConfigDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<LogConfigDto> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "logging_update_config",
        &request_id,
        window_label.as_deref(),
    );

    let result = run_blocking("logging_update_config", move || update_log_config(config)).await;
    match &result {
        Ok(_) => command_end_ok("logging_update_config", &request_id, started_at),
        Err(error) => command_end_error("logging_update_config", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub async fn logging_export_jsonl(
    query: Option<LogQueryDto>,
    output_path: Option<String>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<String> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("logging_export_jsonl", &request_id, window_label.as_deref());

    let normalized = query.unwrap_or_default();
    let result = run_blocking("logging_export_jsonl", move || {
        export_log_entries(normalized, output_path)
    })
    .await;
    match &result {
        Ok(_) => command_end_ok("logging_export_jsonl", &request_id, started_at),
        Err(error) => command_end_error("logging_export_jsonl", &request_id, started_at, error),
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_reject_invalid_level() {
        let result = client_log(
            "invalid".to_string(),
            Some("req".to_string()),
            "ui".to_string(),
            "message".to_string(),
            None,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.expect_err("expected error").code,
            "invalid_log_level"
        );
    }
}

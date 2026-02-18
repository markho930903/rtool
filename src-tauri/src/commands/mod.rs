pub mod app_manager;
pub mod clipboard;
pub mod dashboard;
pub mod i18n_import;
pub mod launcher;
pub mod locale;
pub mod logging;
pub mod palette;
pub mod transfer;

use std::time::Instant;

use crate::core::AppError;
use crate::infrastructure::logging::{
    RecordLogInput, record_log_event_best_effort, sanitize_for_log,
};

pub(crate) fn normalize_request_id(request_id: Option<String>) -> String {
    request_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) fn command_start(
    command: &str,
    request_id: &str,
    window_label: Option<&str>,
) -> Instant {
    tracing::info!(
        event = "command_start",
        command = command,
        request_id = request_id,
        window_label = window_label.unwrap_or("unknown")
    );

    record_log_event_best_effort(RecordLogInput {
        level: "info".to_string(),
        scope: "command".to_string(),
        event: "command_start".to_string(),
        request_id: request_id.to_string(),
        window_label: window_label.map(ToString::to_string),
        message: command.to_string(),
        metadata: Some(serde_json::json!({
            "command": command,
            "windowLabel": window_label.unwrap_or("unknown"),
        })),
        raw_ref: None,
    });

    Instant::now()
}

pub(crate) fn command_end_ok(command: &str, request_id: &str, started_at: Instant) {
    let duration_ms = started_at.elapsed().as_millis() as u64;
    tracing::info!(
        event = "command_end",
        command = command,
        request_id = request_id,
        ok = true,
        duration_ms = duration_ms
    );

    record_log_event_best_effort(RecordLogInput {
        level: "info".to_string(),
        scope: "command".to_string(),
        event: "command_end".to_string(),
        request_id: request_id.to_string(),
        window_label: None,
        message: command.to_string(),
        metadata: Some(serde_json::json!({
            "command": command,
            "ok": true,
            "durationMs": duration_ms,
        })),
        raw_ref: None,
    });
}

pub(crate) fn command_end_error(
    command: &str,
    request_id: &str,
    started_at: Instant,
    error: &AppError,
) {
    let duration_ms = started_at.elapsed().as_millis() as u64;
    tracing::error!(
        event = "command_end",
        command = command,
        request_id = request_id,
        ok = false,
        duration_ms = duration_ms,
        error_code = error.code.as_str(),
        error_message = sanitize_for_log(&error.message),
        error_detail = error
            .detail
            .as_deref()
            .map(sanitize_for_log)
            .unwrap_or_else(String::new)
    );

    record_log_event_best_effort(RecordLogInput {
        level: "error".to_string(),
        scope: "command".to_string(),
        event: "command_end".to_string(),
        request_id: request_id.to_string(),
        window_label: None,
        message: command.to_string(),
        metadata: Some(serde_json::json!({
            "command": command,
            "ok": false,
            "durationMs": duration_ms,
            "errorCode": error.code.as_str(),
            "errorMessage": sanitize_for_log(&error.message),
            "errorDetail": error
                .detail
                .as_deref()
                .map(sanitize_for_log)
                .unwrap_or_default(),
        })),
        raw_ref: None,
    });
}

pub(crate) fn command_end_status(
    command: &str,
    request_id: &str,
    started_at: Instant,
    ok: bool,
    error_code: Option<&str>,
    message: Option<&str>,
) {
    if ok {
        let duration_ms = started_at.elapsed().as_millis() as u64;
        tracing::info!(
            event = "command_end",
            command = command,
            request_id = request_id,
            ok = true,
            duration_ms = duration_ms
        );

        record_log_event_best_effort(RecordLogInput {
            level: "info".to_string(),
            scope: "command".to_string(),
            event: "command_end".to_string(),
            request_id: request_id.to_string(),
            window_label: None,
            message: command.to_string(),
            metadata: Some(serde_json::json!({
                "command": command,
                "ok": true,
                "durationMs": duration_ms,
            })),
            raw_ref: None,
        });
        return;
    }

    let duration_ms = started_at.elapsed().as_millis() as u64;
    tracing::error!(
        event = "command_end",
        command = command,
        request_id = request_id,
        ok = false,
        duration_ms = duration_ms,
        error_code = error_code.unwrap_or("command_failed"),
        error_message = message.map(sanitize_for_log).unwrap_or_else(String::new)
    );

    record_log_event_best_effort(RecordLogInput {
        level: "error".to_string(),
        scope: "command".to_string(),
        event: "command_end".to_string(),
        request_id: request_id.to_string(),
        window_label: None,
        message: command.to_string(),
        metadata: Some(serde_json::json!({
            "command": command,
            "ok": false,
            "durationMs": duration_ms,
            "errorCode": error_code.unwrap_or("command_failed"),
            "errorMessage": message.map(sanitize_for_log).unwrap_or_default(),
        })),
        raw_ref: None,
    });
}

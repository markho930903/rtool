pub mod app_manager;
pub mod clipboard;
pub mod dashboard;
pub mod i18n_import;
pub mod launcher;
pub mod locale;
pub mod logging;
pub mod palette;
pub mod transfer;

use std::future::Future;
use std::time::Instant;

use crate::core::{AppResult, InvokeError};
use crate::infrastructure::logging::{
    RecordLogInput, record_log_event_best_effort, sanitize_for_log,
};
use crate::infrastructure::runtime::blocking::run_blocking;

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

pub(crate) fn command_end_error<E>(command: &str, request_id: &str, started_at: Instant, error: &E)
where
    E: Clone + Into<InvokeError>,
{
    let error: InvokeError = error.clone().into().with_request_id(request_id.to_string());
    let duration_ms = started_at.elapsed().as_millis() as u64;
    let causes: Vec<String> = error
        .causes
        .iter()
        .map(|cause| sanitize_for_log(cause))
        .collect();
    let primary_cause = causes.first().cloned().unwrap_or_default();
    let error_detail = primary_cause.clone();

    tracing::error!(
        event = "command_end",
        command = command,
        request_id = request_id,
        ok = false,
        duration_ms = duration_ms,
        error_code = error.code.as_str(),
        error_message = sanitize_for_log(&error.message),
        error_detail = error_detail.as_str(),
        error_primary_cause = primary_cause.as_str()
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
            "errorDetail": error_detail,
            "errorCauses": causes,
            "errorCausesCount": error.causes.len(),
            "errorPrimaryCause": primary_cause,
            "errorContext": error.context.iter().map(|item| serde_json::json!({
                "key": item.key,
                "value": sanitize_for_log(&item.value),
            })).collect::<Vec<_>>(),
        })),
        raw_ref: None,
    });
}

pub(crate) fn run_command_sync<T, E, F>(
    command: &str,
    request_id: Option<String>,
    window_label: Option<String>,
    op: F,
) -> Result<T, InvokeError>
where
    E: Clone + Into<InvokeError>,
    F: FnOnce() -> Result<T, E>,
{
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(command, &request_id, window_label.as_deref());
    let result = op();
    match &result {
        Ok(_) => command_end_ok(command, &request_id, started_at),
        Err(error) => command_end_error(command, &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

pub(crate) async fn run_command_async<T, E, Fut, F>(
    command: &str,
    request_id: Option<String>,
    window_label: Option<String>,
    op: F,
) -> Result<T, InvokeError>
where
    E: Clone + Into<InvokeError>,
    Fut: Future<Output = Result<T, E>>,
    F: FnOnce() -> Fut,
{
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(command, &request_id, window_label.as_deref());
    let result = op().await;
    match &result {
        Ok(_) => command_end_ok(command, &request_id, started_at),
        Err(error) => command_end_error(command, &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

pub(crate) async fn run_blocking_command<T, F>(
    command: &str,
    request_id: Option<String>,
    window_label: Option<String>,
    blocking_label: &'static str,
    job: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    run_command_async(command, request_id, window_label, move || async move {
        run_blocking(blocking_label, job).await
    })
    .await
}

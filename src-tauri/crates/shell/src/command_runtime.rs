use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use usecase::RequestContext;
use foundation::{AppResult, InvokeError};
use foundation::logging::{RecordLogInput, record_log_event_best_effort, sanitize_for_log};
use foundation::runtime::blocking::run_blocking;

const COMMAND_DETAIL_SAMPLE_RATE: u64 = 16;
const COMMAND_START_SAMPLE_RATE: u64 = 32;
const COMMAND_SLOW_DETAIL_MS: u64 = 400;
const COMMAND_SLOW_TRACE_MS: u64 = 300;
const COMMAND_SUMMARY_WINDOW_MS: i64 = 10_000;
const COMMAND_SUMMARY_MAX_EVENTS: u32 = 200;
const COMMAND_SUMMARY_TOP_N: usize = 5;
const COMMAND_SUMMARY_MAX_KEYS: usize = 32;

#[derive(Debug, Clone, Default)]
struct CommandWindowEntry {
    count: u32,
    total_duration_ms: u64,
    max_duration_ms: u64,
    error_count: u32,
    slow_count: u32,
}

#[derive(Debug, Clone)]
struct CommandRuntimeWindow {
    started_at_ms: i64,
    total_count: u32,
    ok_count: u32,
    error_count: u32,
    total_duration_ms: u64,
    max_duration_ms: u64,
    slow_count: u32,
    by_command: HashMap<String, CommandWindowEntry>,
}

impl CommandRuntimeWindow {
    fn new(started_at_ms: i64) -> Self {
        Self {
            started_at_ms,
            total_count: 0,
            ok_count: 0,
            error_count: 0,
            total_duration_ms: 0,
            max_duration_ms: 0,
            slow_count: 0,
            by_command: HashMap::new(),
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .unwrap_or_default()
}

fn stable_sample(command: &str, request_id: &str, sample_rate: u64) -> bool {
    if sample_rate <= 1 {
        return true;
    }
    let mut hasher = DefaultHasher::new();
    command.hash(&mut hasher);
    request_id.hash(&mut hasher);
    hasher.finish() % sample_rate == 0
}

fn should_emit_start_detail(command: &str, request_id: &str) -> bool {
    stable_sample(command, request_id, COMMAND_START_SAMPLE_RATE)
}

fn should_emit_success_detail(command: &str, request_id: &str, duration_ms: u64) -> bool {
    duration_ms >= COMMAND_SLOW_DETAIL_MS
        || stable_sample(command, request_id, COMMAND_DETAIL_SAMPLE_RATE)
}

fn command_runtime_window_slot() -> &'static Mutex<CommandRuntimeWindow> {
    static WINDOW: OnceLock<Mutex<CommandRuntimeWindow>> = OnceLock::new();
    WINDOW.get_or_init(|| Mutex::new(CommandRuntimeWindow::new(now_ms())))
}

fn make_top_commands(entries: &HashMap<String, CommandWindowEntry>) -> Vec<serde_json::Value> {
    let mut ranked = entries.iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.count.cmp(&left.1.count));
    ranked
        .into_iter()
        .take(COMMAND_SUMMARY_TOP_N)
        .map(|(command, stats)| {
            let avg_duration_ms = if stats.count == 0 {
                0
            } else {
                stats.total_duration_ms / u64::from(stats.count)
            };
            serde_json::json!({
                "command": command,
                "count": stats.count,
                "avgDurationMs": avg_duration_ms,
                "maxDurationMs": stats.max_duration_ms,
                "errorCount": stats.error_count,
                "slowCount": stats.slow_count,
            })
        })
        .collect()
}

fn emit_runtime_summary(window: CommandRuntimeWindow, ended_at_ms: i64, trigger: &'static str) {
    if window.total_count == 0 {
        return;
    }
    let avg_duration_ms = if window.total_count == 0 {
        0
    } else {
        window.total_duration_ms / u64::from(window.total_count)
    };
    record_log_event_best_effort(RecordLogInput {
        level: "info".to_string(),
        scope: "command".to_string(),
        event: "command_summary".to_string(),
        request_id: format!("command-summary-{ended_at_ms}"),
        window_label: None,
        message: "command_runtime_window".to_string(),
        metadata: Some(serde_json::json!({
            "trigger": trigger,
            "windowStartAt": window.started_at_ms,
            "windowEndAt": ended_at_ms,
            "windowMs": ended_at_ms.saturating_sub(window.started_at_ms),
            "totalCount": window.total_count,
            "okCount": window.ok_count,
            "errorCount": window.error_count,
            "slowCount": window.slow_count,
            "avgDurationMs": avg_duration_ms,
            "maxDurationMs": window.max_duration_ms,
            "topCommands": make_top_commands(&window.by_command),
        })),
        raw_ref: None,
    });
}

fn observe_runtime_window(command: &str, duration_ms: u64, ok: bool) {
    let timestamp = now_ms();
    let mut snapshot = None;
    let mut trigger = "window";

    {
        let mut guard = match command_runtime_window_slot().lock() {
            Ok(value) => value,
            Err(poisoned) => poisoned.into_inner(),
        };

        if timestamp.saturating_sub(guard.started_at_ms) >= COMMAND_SUMMARY_WINDOW_MS
            && guard.total_count > 0
        {
            snapshot = Some(guard.clone());
            *guard = CommandRuntimeWindow::new(timestamp);
            trigger = "time";
        }

        guard.total_count = guard.total_count.saturating_add(1);
        guard.total_duration_ms = guard.total_duration_ms.saturating_add(duration_ms);
        guard.max_duration_ms = guard.max_duration_ms.max(duration_ms);
        if ok {
            guard.ok_count = guard.ok_count.saturating_add(1);
        } else {
            guard.error_count = guard.error_count.saturating_add(1);
        }
        if duration_ms >= COMMAND_SLOW_DETAIL_MS {
            guard.slow_count = guard.slow_count.saturating_add(1);
        }

        let key = if guard.by_command.contains_key(command)
            || guard.by_command.len() < COMMAND_SUMMARY_MAX_KEYS
        {
            command.to_string()
        } else {
            "__other__".to_string()
        };
        let entry = guard.by_command.entry(key).or_default();
        entry.count = entry.count.saturating_add(1);
        entry.total_duration_ms = entry.total_duration_ms.saturating_add(duration_ms);
        entry.max_duration_ms = entry.max_duration_ms.max(duration_ms);
        if !ok {
            entry.error_count = entry.error_count.saturating_add(1);
        }
        if duration_ms >= COMMAND_SLOW_DETAIL_MS {
            entry.slow_count = entry.slow_count.saturating_add(1);
        }

        if guard.total_count >= COMMAND_SUMMARY_MAX_EVENTS {
            if snapshot.is_none() {
                snapshot = Some(guard.clone());
            }
            *guard = CommandRuntimeWindow::new(timestamp);
            trigger = "count";
        }
    }

    if let Some(window) = snapshot {
        emit_runtime_summary(window, timestamp, trigger);
    }
}

pub(crate) fn command_start(
    command: &str,
    request_id: &str,
    window_label: Option<&str>,
) -> Instant {
    foundation::record_command_start(command, request_id);

    tracing::debug!(
        event = "command_start",
        command = command,
        request_id = request_id,
        window_label = window_label.unwrap_or("unknown")
    );

    if should_emit_start_detail(command, request_id) {
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
                "sampleRate": COMMAND_START_SAMPLE_RATE,
            })),
            raw_ref: None,
        });
    }

    Instant::now()
}

pub(crate) fn command_end_ok(command: &str, request_id: &str, started_at: Instant) {
    let duration_ms = started_at.elapsed().as_millis() as u64;
    foundation::record_command_end(command, request_id, true, duration_ms);
    observe_runtime_window(command, duration_ms, true);

    if duration_ms >= COMMAND_SLOW_TRACE_MS {
        tracing::info!(
            event = "command_end",
            command = command,
            request_id = request_id,
            ok = true,
            duration_ms = duration_ms
        );
    } else {
        tracing::debug!(
            event = "command_end",
            command = command,
            request_id = request_id,
            ok = true,
            duration_ms = duration_ms
        );
    }

    if should_emit_success_detail(command, request_id, duration_ms) {
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
                "sampleRate": COMMAND_DETAIL_SAMPLE_RATE,
                "isSlowPath": duration_ms >= COMMAND_SLOW_DETAIL_MS,
            })),
            raw_ref: None,
        });
    }
}

pub(crate) fn command_end_error<E>(command: &str, request_id: &str, started_at: Instant, error: &E)
where
    E: Clone + Into<InvokeError>,
{
    let error: InvokeError = error.clone().into().with_request_id(request_id.to_string());
    let duration_ms = started_at.elapsed().as_millis() as u64;
    foundation::record_command_end(command, request_id, false, duration_ms);
    observe_runtime_window(command, duration_ms, false);

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
    let context = RequestContext::new(request_id, window_label);
    let started_at = command_start(command, context.request_id(), context.window_label());
    let result = op();
    match &result {
        Ok(_) => command_end_ok(command, context.request_id(), started_at),
        Err(error) => command_end_error(command, context.request_id(), started_at, error),
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
    let context = RequestContext::new(request_id, window_label);
    let started_at = command_start(command, context.request_id(), context.window_label());
    let result = op().await;
    match &result {
        Ok(_) => command_end_ok(command, context.request_id(), started_at),
        Err(error) => command_end_error(command, context.request_id(), started_at, error),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_sample_should_be_deterministic() {
        let first = stable_sample("app_manager_list", "request-1", 16);
        let second = stable_sample("app_manager_list", "request-1", 16);
        assert_eq!(first, second);
    }

    #[test]
    fn should_emit_success_detail_should_always_keep_slow_path() {
        assert!(should_emit_success_detail(
            "app_manager_list",
            "request-1",
            COMMAND_SLOW_DETAIL_MS + 1
        ));
    }
}

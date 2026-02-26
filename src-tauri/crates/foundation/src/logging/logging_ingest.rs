use super::logging_store::{cleanup_expired_log_entries, save_log_entry, upsert_aggregated_log};
use super::{
    DEFAULT_ALLOW_RAW_VIEW, DEFAULT_HIGH_FREQ_MAX_PER_KEY, DEFAULT_HIGH_FREQ_WINDOW_MS,
    DEFAULT_KEEP_DAYS, DEFAULT_MIN_LEVEL, DEFAULT_REALTIME_ENABLED,
    LOG_RETENTION_CLEANUP_INTERVAL_MS, LogCenter, LogConfigDto, MAX_COLLECTION_ITEMS,
    MAX_NESTED_DEPTH, MAX_STRING_LEN, RecordLogInput, SENSITIVE_HOST_KEYS, SENSITIVE_PATH_KEYS,
    SENSITIVE_TEXT_KEYS,
};
use anyhow::Context;
use crate::{AppError, ResultExt};
use serde_json::{Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn short_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

fn truncate_text(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }

    let mut truncated = String::new();
    for ch in value.chars() {
        if truncated.len() + ch.len_utf8() > max_len {
            break;
        }
        truncated.push(ch);
    }

    format!("{truncated}...(truncated,len={})", value.len())
}

pub(super) fn normalize_level(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "trace" => Some("trace"),
        "debug" => Some("debug"),
        "info" => Some("info"),
        "warn" => Some("warn"),
        "error" => Some("error"),
        _ => None,
    }
}

pub(super) fn level_rank(value: &str) -> Option<u8> {
    match normalize_level(value)? {
        "trace" => Some(0),
        "debug" => Some(1),
        "info" => Some(2),
        "warn" => Some(3),
        "error" => Some(4),
        _ => None,
    }
}

fn looks_like_path(value: &str) -> bool {
    if value.starts_with("file://") {
        return true;
    }

    if value.starts_with("~/") || value.starts_with('/') {
        return true;
    }

    value.contains(":\\") || value.contains('\\') || value.matches('/').count() >= 2
}

fn redact_text_value(value: &str) -> String {
    format!(
        "[redacted-text len={} hash={}]",
        value.len(),
        short_hash(value)
    )
}

pub fn sanitize_path(value: &str) -> String {
    let normalized = value.trim().trim_matches('"').trim_matches('\'');
    let file_name = Path::new(normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("unknown");

    format!("[path:{} dir_hash={}]", file_name, short_hash(normalized))
}

pub fn sanitize_for_log(value: &str) -> String {
    let normalized = value.trim();
    if normalized.is_empty() {
        return String::new();
    }

    if normalized.starts_with("data:") {
        return format!(
            "[data-url redacted len={} hash={}]",
            normalized.len(),
            short_hash(normalized)
        );
    }

    if looks_like_path(normalized) {
        return sanitize_path(normalized);
    }

    truncate_text(normalized, MAX_STRING_LEN)
}

fn is_sensitive_key(parent_key: Option<&str>, candidates: &[&str]) -> bool {
    parent_key.is_some_and(|key| {
        let normalized = key.to_ascii_lowercase();
        candidates.iter().any(|item| normalized.contains(item))
    })
}

fn sanitize_json_value_inner(value: &Value, depth: usize, parent_key: Option<&str>) -> Value {
    if depth >= MAX_NESTED_DEPTH {
        return Value::String("[max-depth-reached]".to_string());
    }

    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
        Value::String(raw) => {
            if is_sensitive_key(parent_key, &SENSITIVE_TEXT_KEYS) {
                return Value::String(redact_text_value(raw));
            }

            if is_sensitive_key(parent_key, &SENSITIVE_PATH_KEYS) {
                return Value::String(sanitize_path(raw));
            }

            if is_sensitive_key(parent_key, &SENSITIVE_HOST_KEYS) {
                return Value::String(format!("[host hash={}]", short_hash(raw)));
            }

            Value::String(sanitize_for_log(raw))
        }
        Value::Array(items) => {
            let sanitized = items
                .iter()
                .take(MAX_COLLECTION_ITEMS)
                .map(|item| sanitize_json_value_inner(item, depth + 1, parent_key))
                .collect::<Vec<_>>();
            Value::Array(sanitized)
        }
        Value::Object(object) => {
            let mut next = Map::new();
            for (index, (key, item)) in object.iter().enumerate() {
                if index >= MAX_COLLECTION_ITEMS {
                    next.insert(
                        "_truncated".to_string(),
                        Value::String(format!("{} keys truncated", object.len() - index)),
                    );
                    break;
                }
                next.insert(
                    key.to_string(),
                    sanitize_json_value_inner(item, depth + 1, Some(key)),
                );
            }
            Value::Object(next)
        }
    }
}

pub fn sanitize_json_value(value: &Value) -> Value {
    sanitize_json_value_inner(value, 0, None)
}

pub(crate) fn cleanup_expired_logs_with_duration(
    log_dir: &Path,
    keep_duration: Duration,
    now: SystemTime,
) -> Result<usize, AppError> {
    if !log_dir.exists() {
        return Ok(0);
    }

    let entries = fs::read_dir(log_dir)
        .with_context(|| format!("读取日志目录失败: {}", log_dir.display()))
        .with_code("log_cleanup_read_dir_failed", "读取日志目录失败")
        .with_ctx("logDir", log_dir.display().to_string())?;

    let mut removed = 0usize;
    for entry in entries {
        let entry = entry
            .with_context(|| format!("读取日志条目失败: {}", log_dir.display()))
            .with_code("log_cleanup_read_entry_failed", "读取日志条目失败")
            .with_ctx("logDir", log_dir.display().to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let metadata = entry
            .metadata()
            .with_context(|| format!("读取日志元数据失败: {}", path.display()))
            .with_code("log_cleanup_metadata_failed", "读取日志元数据失败")
            .with_ctx("logPath", path.display().to_string())?;
        let modified_at = metadata
            .modified()
            .with_context(|| format!("读取日志修改时间失败: {}", path.display()))
            .with_code("log_cleanup_modified_time_failed", "读取日志修改时间失败")
            .with_ctx("logPath", path.display().to_string())?;

        let elapsed = now.duration_since(modified_at).unwrap_or_default();
        if elapsed <= keep_duration {
            continue;
        }

        fs::remove_file(&path)
            .with_context(|| format!("删除过期日志失败: {}", path.display()))
            .with_code("log_cleanup_remove_failed", "删除过期日志失败")
            .with_ctx("logPath", path.display().to_string())?;
        removed += 1;
    }

    Ok(removed)
}

pub fn cleanup_expired_logs(log_dir: &Path, keep_days: u64) -> Result<(), AppError> {
    let keep_duration = Duration::from_secs(keep_days.saturating_mul(24 * 60 * 60));
    let _ = cleanup_expired_logs_with_duration(log_dir, keep_duration, SystemTime::now())?;
    Ok(())
}

fn sanitize_record_input(input: RecordLogInput) -> RecordLogInput {
    RecordLogInput {
        level: normalize_level(&input.level).unwrap_or("info").to_string(),
        scope: sanitize_for_log(&input.scope),
        event: sanitize_for_log(&input.event),
        request_id: sanitize_for_log(&input.request_id),
        window_label: input.window_label.map(|value| sanitize_for_log(&value)),
        message: sanitize_for_log(&input.message),
        metadata: input.metadata.as_ref().map(sanitize_json_value),
        raw_ref: input.raw_ref.map(|value| sanitize_for_log(&value)),
    }
}

impl LogCenter {
    fn should_emit_level(&self, level: &str, config: &LogConfigDto) -> bool {
        let min_rank = level_rank(&config.min_level).unwrap_or(2);
        let current_rank = level_rank(level).unwrap_or(2);
        current_rank >= min_rank
    }

    async fn maybe_cleanup(&self, config: &LogConfigDto, timestamp: i64) {
        let should_cleanup = {
            let mut last_cleanup_guard = match self.last_cleanup_at.lock() {
                Ok(value) => value,
                Err(_) => return,
            };

            if timestamp - *last_cleanup_guard < LOG_RETENTION_CLEANUP_INTERVAL_MS {
                false
            } else {
                *last_cleanup_guard = timestamp;
                true
            }
        };

        if !should_cleanup {
            return;
        }

        let _ = cleanup_expired_logs(&self.log_dir, u64::from(config.keep_days));
        let _ = cleanup_expired_log_entries(&self.db_conn, config.keep_days).await;
    }

    fn emit_realtime(&self, config: &LogConfigDto, entry: &crate::models::LogEntryDto) {
        if !config.realtime_enabled {
            return;
        }

        if let Some(sink) = &self.event_sink {
            let _ = sink.emit_stream(entry);
        }
    }

    fn should_aggregate(
        &self,
        config: &LogConfigDto,
        key: &str,
        timestamp: i64,
    ) -> Option<super::HighFrequencyWindow> {
        let mut map = self.high_frequency.lock().ok()?;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| super::HighFrequencyWindow {
                started_at: timestamp,
                count: 0,
                aggregated_row_id: None,
                aggregated_count: 0,
            });

        if timestamp - entry.started_at >= i64::from(config.high_freq_window_ms) {
            *entry = super::HighFrequencyWindow {
                started_at: timestamp,
                count: 0,
                aggregated_row_id: None,
                aggregated_count: 0,
            };
        }

        entry.count = entry.count.saturating_add(1);
        if entry.count <= config.high_freq_max_per_key {
            return None;
        }

        Some(super::HighFrequencyWindow {
            started_at: entry.started_at,
            count: entry.count,
            aggregated_row_id: entry.aggregated_row_id,
            aggregated_count: entry.aggregated_count,
        })
    }

    fn update_aggregate_window(&self, key: &str, next: super::HighFrequencyWindow) {
        if let Ok(mut map) = self.high_frequency.lock() {
            map.insert(key.to_string(), next);
        }
    }

    pub(super) async fn ingest(&self, input: RecordLogInput) -> Result<(), AppError> {
        let sanitized = sanitize_record_input(input);
        let config = self
            .config
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| LogConfigDto {
                min_level: DEFAULT_MIN_LEVEL.to_string(),
                keep_days: DEFAULT_KEEP_DAYS,
                realtime_enabled: DEFAULT_REALTIME_ENABLED,
                high_freq_window_ms: DEFAULT_HIGH_FREQ_WINDOW_MS,
                high_freq_max_per_key: DEFAULT_HIGH_FREQ_MAX_PER_KEY,
                allow_raw_view: DEFAULT_ALLOW_RAW_VIEW,
            });

        if !self.should_emit_level(&sanitized.level, &config) {
            return Ok(());
        }

        let timestamp = now_millis();
        self.maybe_cleanup(&config, timestamp).await;

        let event_key = format!(
            "{}|{}|{}|{}",
            sanitized.level,
            sanitized.scope,
            sanitized.event,
            sanitized.window_label.clone().unwrap_or_default()
        );

        if let Some(mut window) = self.should_aggregate(&config, &event_key, timestamp) {
            let entry = upsert_aggregated_log(
                &self.db_conn,
                &event_key,
                &sanitized,
                timestamp,
                &mut window,
            )
            .await?;
            self.update_aggregate_window(&event_key, window);
            self.emit_realtime(&config, &entry);
            return Ok(());
        }

        let entry = save_log_entry(&self.db_conn, &sanitized, timestamp).await?;
        self.emit_realtime(&config, &entry);
        Ok(())
    }
}

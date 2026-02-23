use app_core::models::{LogConfigDto, LogEntryDto, LogPageDto, LogQueryDto};
use app_core::{AppError, ResultExt};
use crate::db::{self, DbPool};
use crate::runtime::blocking::run_blocking;
use anyhow::Context;
use rusqlite::{OptionalExtension, params};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder as RollingBuilder, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const DEFAULT_KEEP_DAYS: u32 = 7;
const DEFAULT_MIN_LEVEL: &str = "info";
const DEFAULT_REALTIME_ENABLED: bool = true;
const DEFAULT_HIGH_FREQ_WINDOW_MS: u32 = 1000;
const DEFAULT_HIGH_FREQ_MAX_PER_KEY: u32 = 20;
const DEFAULT_ALLOW_RAW_VIEW: bool = false;
const LOG_RETENTION_CLEANUP_INTERVAL_MS: i64 = 30 * 60 * 1000;

const SETTING_KEY_MIN_LEVEL: &str = "logging.minLevel";
const SETTING_KEY_KEEP_DAYS: &str = "logging.keepDays";
const SETTING_KEY_REALTIME_ENABLED: &str = "logging.realtimeEnabled";
const SETTING_KEY_HIGH_FREQ_WINDOW_MS: &str = "logging.highFreqWindowMs";
const SETTING_KEY_HIGH_FREQ_MAX_PER_KEY: &str = "logging.highFreqMaxPerKey";
const SETTING_KEY_ALLOW_RAW_VIEW: &str = "logging.allowRawView";

const MAX_STRING_LEN: usize = 256;
const MAX_COLLECTION_ITEMS: usize = 64;
const MAX_NESTED_DEPTH: usize = 6;
const QUERY_LIMIT_MAX: u32 = 500;
const QUERY_LIMIT_DEFAULT: u32 = 100;
const EXPORT_FLUSH_EVERY_PAGES: u32 = 4;
const EXPORT_THROTTLE_SLEEP_MS: u64 = 1;

const SENSITIVE_TEXT_KEYS: [&str; 5] = ["text", "content", "clipboard", "prompt", "input"];
const SENSITIVE_PATH_KEYS: [&str; 4] = ["path", "file", "filepath", "filename"];
const SENSITIVE_HOST_KEYS: [&str; 2] = ["host", "hostname"];

#[derive(Debug, Clone)]
pub struct LoggingGuard {
    log_dir: PathBuf,
    level: String,
}

impl LoggingGuard {
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    pub fn level(&self) -> &str {
        &self.level
    }
}

#[derive(Debug, Clone)]
pub struct RecordLogInput {
    pub level: String,
    pub scope: String,
    pub event: String,
    pub request_id: String,
    pub window_label: Option<String>,
    pub message: String,
    pub metadata: Option<Value>,
    pub raw_ref: Option<String>,
}

#[derive(Debug)]
struct HighFrequencyWindow {
    started_at: i64,
    count: u32,
    aggregated_row_id: Option<i64>,
    aggregated_count: u32,
}

pub trait LoggingEventSink: Send + Sync {
    fn emit_stream(&self, entry: &LogEntryDto) -> Result<(), AppError>;
}

struct LogCenter {
    event_sink: Option<Arc<dyn LoggingEventSink>>,
    db_pool: DbPool,
    log_dir: PathBuf,
    config: Mutex<LogConfigDto>,
    high_frequency: Mutex<HashMap<String, HighFrequencyWindow>>,
    last_cleanup_at: Mutex<i64>,
}

fn worker_guard_slot() -> &'static Mutex<Option<WorkerGuard>> {
    static SLOT: OnceLock<Mutex<Option<WorkerGuard>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

fn log_center_slot() -> &'static OnceLock<Arc<LogCenter>> {
    static SLOT: OnceLock<Arc<LogCenter>> = OnceLock::new();
    &SLOT
}

fn short_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn now_millis() -> i64 {
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

fn normalize_level(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "trace" => Some("trace"),
        "debug" => Some("debug"),
        "info" => Some("info"),
        "warn" => Some("warn"),
        "error" => Some("error"),
        _ => None,
    }
}

fn level_rank(value: &str) -> Option<u8> {
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

fn read_bool_setting(pool: &DbPool, key: &str, default: bool) -> bool {
    db::get_app_setting(pool, key)
        .ok()
        .flatten()
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(default)
}

fn read_u32_setting(pool: &DbPool, key: &str, default: u32, min: u32, max: u32) -> u32 {
    db::get_app_setting(pool, key)
        .ok()
        .flatten()
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn read_level_setting(pool: &DbPool) -> String {
    db::get_app_setting(pool, SETTING_KEY_MIN_LEVEL)
        .ok()
        .flatten()
        .and_then(|value| normalize_level(&value).map(ToString::to_string))
        .unwrap_or_else(|| DEFAULT_MIN_LEVEL.to_string())
}

fn load_log_config(pool: &DbPool) -> LogConfigDto {
    LogConfigDto {
        min_level: read_level_setting(pool),
        keep_days: read_u32_setting(pool, SETTING_KEY_KEEP_DAYS, DEFAULT_KEEP_DAYS, 1, 90),
        realtime_enabled: read_bool_setting(
            pool,
            SETTING_KEY_REALTIME_ENABLED,
            DEFAULT_REALTIME_ENABLED,
        ),
        high_freq_window_ms: read_u32_setting(
            pool,
            SETTING_KEY_HIGH_FREQ_WINDOW_MS,
            DEFAULT_HIGH_FREQ_WINDOW_MS,
            100,
            60_000,
        ),
        high_freq_max_per_key: read_u32_setting(
            pool,
            SETTING_KEY_HIGH_FREQ_MAX_PER_KEY,
            DEFAULT_HIGH_FREQ_MAX_PER_KEY,
            1,
            200,
        ),
        allow_raw_view: read_bool_setting(pool, SETTING_KEY_ALLOW_RAW_VIEW, DEFAULT_ALLOW_RAW_VIEW),
    }
}

fn persist_log_config(pool: &DbPool, config: &LogConfigDto) -> Result<(), AppError> {
    db::set_app_setting(pool, SETTING_KEY_MIN_LEVEL, &config.min_level)?;
    db::set_app_setting(pool, SETTING_KEY_KEEP_DAYS, &config.keep_days.to_string())?;
    db::set_app_setting(
        pool,
        SETTING_KEY_REALTIME_ENABLED,
        if config.realtime_enabled {
            "true"
        } else {
            "false"
        },
    )?;
    db::set_app_setting(
        pool,
        SETTING_KEY_HIGH_FREQ_WINDOW_MS,
        &config.high_freq_window_ms.to_string(),
    )?;
    db::set_app_setting(
        pool,
        SETTING_KEY_HIGH_FREQ_MAX_PER_KEY,
        &config.high_freq_max_per_key.to_string(),
    )?;
    db::set_app_setting(
        pool,
        SETTING_KEY_ALLOW_RAW_VIEW,
        if config.allow_raw_view {
            "true"
        } else {
            "false"
        },
    )?;

    Ok(())
}

fn clamp_and_normalize_config(mut config: LogConfigDto) -> Result<LogConfigDto, AppError> {
    let level = normalize_level(&config.min_level).ok_or_else(|| {
        AppError::new("invalid_log_level", "日志级别非法")
            .with_context("level", sanitize_for_log(&config.min_level))
    })?;

    config.min_level = level.to_string();
    config.keep_days = config.keep_days.clamp(1, 90);
    config.high_freq_window_ms = config.high_freq_window_ms.clamp(100, 60_000);
    config.high_freq_max_per_key = config.high_freq_max_per_key.clamp(1, 200);
    Ok(config)
}

fn cleanup_expired_logs_with_duration(
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

fn cleanup_expired_log_entries(pool: &DbPool, keep_days: u32) -> Result<(), AppError> {
    let keep_ms = i64::from(keep_days) * 24 * 60 * 60 * 1000;
    let cutoff = now_millis().saturating_sub(keep_ms);
    let conn = pool.get()?;
    conn.execute(
        "DELETE FROM log_entries WHERE timestamp < ?1",
        params![cutoff],
    )?;
    Ok(())
}

fn parse_metadata_value(metadata: Option<String>) -> Option<Value> {
    metadata.and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn serialize_metadata_value(metadata: &Option<Value>) -> Option<String> {
    metadata
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok())
}

fn row_to_log_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<LogEntryDto> {
    let aggregated_count: Option<i64> = row.get(10)?;
    Ok(LogEntryDto {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        level: row.get(2)?,
        scope: row.get(3)?,
        event: row.get(4)?,
        request_id: row.get(5)?,
        window_label: row.get(6)?,
        message: row.get(7)?,
        metadata: parse_metadata_value(row.get(8)?),
        raw_ref: row.get(9)?,
        aggregated_count: aggregated_count.and_then(|value| u32::try_from(value).ok()),
    })
}

fn save_log_entry(
    pool: &DbPool,
    input: &RecordLogInput,
    timestamp: i64,
) -> Result<LogEntryDto, AppError> {
    let metadata = serialize_metadata_value(&input.metadata);
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO log_entries (timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL)",
        params![
            timestamp,
            input.level,
            input.scope,
            input.event,
            input.request_id,
            input.window_label,
            input.message,
            metadata,
            input.raw_ref
        ],
    )?;

    let id = conn.last_insert_rowid();
    Ok(LogEntryDto {
        id,
        timestamp,
        level: input.level.clone(),
        scope: input.scope.clone(),
        event: input.event.clone(),
        request_id: input.request_id.clone(),
        window_label: input.window_label.clone(),
        message: input.message.clone(),
        metadata: input.metadata.clone(),
        raw_ref: input.raw_ref.clone(),
        aggregated_count: None,
    })
}

fn upsert_aggregated_log(
    pool: &DbPool,
    key: &str,
    input: &RecordLogInput,
    timestamp: i64,
    window: &mut HighFrequencyWindow,
) -> Result<LogEntryDto, AppError> {
    let conn = pool.get()?;
    if let Some(row_id) = window.aggregated_row_id {
        window.aggregated_count = window.aggregated_count.saturating_add(1);
        conn.execute(
            "UPDATE log_entries
             SET timestamp = ?1,
                 aggregated_count = ?2,
                 message = ?3
             WHERE id = ?4",
            params![
                timestamp,
                i64::from(window.aggregated_count),
                format!("high_frequency_aggregated key={}", sanitize_for_log(key)),
                row_id
            ],
        )?;

        let row = conn
            .query_row(
                "SELECT id, timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count
                 FROM log_entries
                 WHERE id = ?1
                 LIMIT 1",
                params![row_id],
                row_to_log_entry,
            )
            .optional()?;

        return row.ok_or_else(|| AppError::new("log_aggregate_missing", "聚合日志记录不存在"));
    }

    window.aggregated_count = 1;
    let aggregated_message = format!("high_frequency_aggregated key={}", sanitize_for_log(key));
    conn.execute(
        "INSERT INTO log_entries (timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, ?8)",
        params![
            timestamp,
            input.level,
            input.scope,
            "high_frequency_aggregated",
            input.request_id,
            input.window_label,
            aggregated_message,
            i64::from(window.aggregated_count)
        ],
    )?;

    let row_id = conn.last_insert_rowid();
    window.aggregated_row_id = Some(row_id);

    Ok(LogEntryDto {
        id: row_id,
        timestamp,
        level: input.level.clone(),
        scope: input.scope.clone(),
        event: "high_frequency_aggregated".to_string(),
        request_id: input.request_id.clone(),
        window_label: input.window_label.clone(),
        message: aggregated_message,
        metadata: None,
        raw_ref: None,
        aggregated_count: Some(window.aggregated_count),
    })
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

    fn maybe_cleanup(&self, config: &LogConfigDto, timestamp: i64) {
        let mut last_cleanup_guard = match self.last_cleanup_at.lock() {
            Ok(value) => value,
            Err(_) => return,
        };

        if timestamp - *last_cleanup_guard < LOG_RETENTION_CLEANUP_INTERVAL_MS {
            return;
        }

        *last_cleanup_guard = timestamp;

        let _ = cleanup_expired_logs(&self.log_dir, u64::from(config.keep_days));
        let _ = cleanup_expired_log_entries(&self.db_pool, config.keep_days);
    }

    fn emit_realtime(&self, config: &LogConfigDto, entry: &LogEntryDto) {
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
    ) -> Option<HighFrequencyWindow> {
        let mut map = self.high_frequency.lock().ok()?;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| HighFrequencyWindow {
                started_at: timestamp,
                count: 0,
                aggregated_row_id: None,
                aggregated_count: 0,
            });

        if timestamp - entry.started_at >= i64::from(config.high_freq_window_ms) {
            *entry = HighFrequencyWindow {
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

        Some(HighFrequencyWindow {
            started_at: entry.started_at,
            count: entry.count,
            aggregated_row_id: entry.aggregated_row_id,
            aggregated_count: entry.aggregated_count,
        })
    }

    fn update_aggregate_window(&self, key: &str, next: HighFrequencyWindow) {
        if let Ok(mut map) = self.high_frequency.lock() {
            map.insert(key.to_string(), next);
        }
    }

    fn ingest(&self, input: RecordLogInput) -> Result<(), AppError> {
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
        self.maybe_cleanup(&config, timestamp);

        let event_key = format!(
            "{}|{}|{}|{}",
            sanitized.level,
            sanitized.scope,
            sanitized.event,
            sanitized.window_label.clone().unwrap_or_default()
        );

        if let Some(mut window) = self.should_aggregate(&config, &event_key, timestamp) {
            let entry = upsert_aggregated_log(
                &self.db_pool,
                &event_key,
                &sanitized,
                timestamp,
                &mut window,
            )?;
            self.update_aggregate_window(&event_key, window);
            self.emit_realtime(&config, &entry);
            return Ok(());
        }

        let entry = save_log_entry(&self.db_pool, &sanitized, timestamp)?;
        self.emit_realtime(&config, &entry);
        Ok(())
    }
}

pub fn resolve_log_level() -> String {
    let env_level = std::env::var("RTOOL_LOG_LEVEL")
        .ok()
        .map(|value| value.to_ascii_lowercase());
    if let Some(level) = env_level
        && matches!(
            level.as_str(),
            "trace" | "debug" | "info" | "warn" | "error"
        )
    {
        return level;
    }

    if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "info".to_string()
    }
}

pub fn init_logging(app_data_dir: &Path) -> Result<LoggingGuard, AppError> {
    let log_dir = app_data_dir.join("logs");
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("创建日志目录失败: {}", log_dir.display()))
        .with_code("log_dir_create_failed", "创建日志目录失败")
        .with_ctx("logDir", log_dir.display().to_string())?;
    cleanup_expired_logs(&log_dir, u64::from(DEFAULT_KEEP_DAYS))?;

    let file_appender = RollingBuilder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("rtool")
        .filename_suffix("log")
        .build(&log_dir)
        .with_context(|| format!("创建日志写入器失败: {}", log_dir.display()))
        .with_code("log_appender_create_failed", "创建日志写入器失败")
        .with_ctx("logDir", log_dir.display().to_string())?;
    let (file_writer, worker_guard) = tracing_appender::non_blocking(file_appender);

    if let Ok(mut slot) = worker_guard_slot().lock() {
        *slot = Some(worker_guard);
    }

    let level = resolve_log_level();
    if !tracing::dispatcher::has_been_set() {
        let env_filter = EnvFilter::new(level.clone());
        let file_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_ansi(false)
            .with_writer(file_writer)
            .with_current_span(false)
            .with_span_list(false);

        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer);
        #[cfg(debug_assertions)]
        let subscriber = subscriber.with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_ansi(true)
                .with_target(true),
        );

        subscriber
            .try_init()
            .with_context(|| format!("初始化日志订阅器失败: level={level}"))
            .with_code("log_subscriber_init_failed", "初始化日志订阅器失败")
            .with_ctx("logLevel", level.clone())?;
    }

    Ok(LoggingGuard { log_dir, level })
}

pub fn init_log_center(
    db_pool: DbPool,
    log_dir: PathBuf,
    event_sink: Option<Arc<dyn LoggingEventSink>>,
) -> Result<LogConfigDto, AppError> {
    let config = clamp_and_normalize_config(load_log_config(&db_pool))?;
    persist_log_config(&db_pool, &config)?;

    let center = Arc::new(LogCenter {
        event_sink,
        db_pool,
        log_dir,
        config: Mutex::new(config.clone()),
        high_frequency: Mutex::new(HashMap::new()),
        last_cleanup_at: Mutex::new(0),
    });

    match log_center_slot().set(center) {
        Ok(_) => Ok(config),
        Err(_) => get_log_config(),
    }
}

fn with_log_center<T>(f: impl FnOnce(&LogCenter) -> Result<T, AppError>) -> Result<T, AppError> {
    let center = log_center_slot()
        .get()
        .ok_or_else(|| AppError::new("log_center_uninitialized", "日志中心未初始化"))?;
    f(center)
}

pub fn record_log_event(input: RecordLogInput) -> Result<(), AppError> {
    with_log_center(|center| center.ingest(input))
}

pub fn record_log_event_best_effort(input: RecordLogInput) {
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::spawn(async move {
            let _ = run_blocking("record_log_event_best_effort", move || {
                if let Err(error) = record_log_event(input) {
                    tracing::warn!(
                        event = "logging_ingest_failed",
                        error_code = error.code,
                        error_detail = error.causes.first().map(String::as_str).unwrap_or_default()
                    );
                }
                Ok(())
            })
            .await;
        });
    } else {
        std::thread::spawn(move || {
            if let Err(error) = record_log_event(input) {
                eprintln!("logging ingest failed: {}", error);
            }
        });
    }
}

pub fn get_log_config() -> Result<LogConfigDto, AppError> {
    with_log_center(|center| {
        center
            .config
            .lock()
            .map(|value| value.clone())
            .map_err(|_| AppError::new("log_config_read_failed", "读取日志配置失败"))
    })
}

pub fn update_log_config(input: LogConfigDto) -> Result<LogConfigDto, AppError> {
    let normalized = clamp_and_normalize_config(input)?;

    with_log_center(|center| {
        persist_log_config(&center.db_pool, &normalized)?;
        let mut guard = center
            .config
            .lock()
            .map_err(|_| AppError::new("log_config_update_failed", "更新日志配置失败"))?;
        *guard = normalized.clone();
        Ok(normalized)
    })
}

pub fn query_log_entries(query: LogQueryDto) -> Result<LogPageDto, AppError> {
    with_log_center(|center| {
        let limit = query.limit.clamp(1, QUERY_LIMIT_MAX);
        let mut sql = String::from(
            "SELECT id, timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count FROM log_entries WHERE 1=1",
        );
        let mut params = Vec::<rusqlite::types::Value>::new();

        if let Some(cursor) = query.cursor.as_deref() {
            let cursor_id = cursor.parse::<i64>().map_err(|_| {
                AppError::new("invalid_cursor", "日志分页游标非法")
                    .with_context("cursor", sanitize_for_log(cursor))
            })?;
            sql.push_str(" AND id < ?");
            params.push(rusqlite::types::Value::Integer(cursor_id));
        }

        if let Some(levels) = query.levels.as_ref().filter(|levels| !levels.is_empty()) {
            let mut normalized_levels = Vec::new();
            for level in levels {
                let normalized = normalize_level(level).ok_or_else(|| {
                    AppError::new("invalid_log_level", "日志级别非法")
                        .with_context("level", sanitize_for_log(level))
                })?;
                normalized_levels.push(normalized.to_string());
            }

            sql.push_str(" AND level IN (");
            for (index, value) in normalized_levels.iter().enumerate() {
                if index > 0 {
                    sql.push_str(", ");
                }
                sql.push('?');
                params.push(rusqlite::types::Value::Text(value.clone()));
            }
            sql.push(')');
        }

        if let Some(scope) = query
            .scope
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            sql.push_str(" AND scope = ?");
            params.push(rusqlite::types::Value::Text(sanitize_for_log(scope)));
        }

        if let Some(request_id) = query
            .request_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            sql.push_str(" AND request_id = ?");
            params.push(rusqlite::types::Value::Text(sanitize_for_log(request_id)));
        }

        if let Some(window_label) = query
            .window_label
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            sql.push_str(" AND window_label = ?");
            params.push(rusqlite::types::Value::Text(sanitize_for_log(window_label)));
        }

        if let Some(keyword) = query
            .keyword
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            sql.push_str(" AND (message LIKE ? OR metadata LIKE ? OR event LIKE ?)");
            let pattern = format!("%{}%", sanitize_for_log(keyword));
            params.push(rusqlite::types::Value::Text(pattern.clone()));
            params.push(rusqlite::types::Value::Text(pattern.clone()));
            params.push(rusqlite::types::Value::Text(pattern));
        }

        if let Some(start_at) = query.start_at {
            sql.push_str(" AND timestamp >= ?");
            params.push(rusqlite::types::Value::Integer(start_at));
        }

        if let Some(end_at) = query.end_at {
            sql.push_str(" AND timestamp <= ?");
            params.push(rusqlite::types::Value::Integer(end_at));
        }

        sql.push_str(" ORDER BY id DESC LIMIT ?");
        params.push(rusqlite::types::Value::Integer(i64::from(limit) + 1));

        let conn = center.db_pool.get()?;
        let mut statement = conn.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(params), row_to_log_entry)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }

        let next_cursor =
            if items.len() > usize::try_from(limit).unwrap_or(QUERY_LIMIT_DEFAULT as usize) {
                let marker = items
                    .get(usize::try_from(limit).unwrap_or(0))
                    .map(|value| value.id)
                    .unwrap_or_default();
                items.truncate(usize::try_from(limit).unwrap_or(QUERY_LIMIT_DEFAULT as usize));
                Some(marker.to_string())
            } else {
                None
            };

        Ok(LogPageDto { items, next_cursor })
    })
}

pub fn export_log_entries(
    query: LogQueryDto,
    output_path: Option<String>,
) -> Result<String, AppError> {
    let mut cursor = query.cursor.clone();
    let mut page_count = 0u32;

    let target_path = with_log_center(|center| {
        let resolved = output_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                center
                    .log_dir
                    .join(format!("rtool-log-export-{}.jsonl", now_millis()))
            });

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建日志导出目录失败: {}", parent.display()))
                .with_code("log_export_dir_create_failed", "创建日志导出目录失败")
                .with_ctx("outputDir", parent.display().to_string())?;
        }

        Ok(resolved)
    })?;

    let file = File::create(&target_path)
        .with_context(|| format!("创建日志导出文件失败: {}", target_path.display()))
        .with_code("log_export_file_create_failed", "创建日志导出文件失败")
        .with_ctx("targetPath", target_path.display().to_string())?;
    let mut writer = BufWriter::new(file);

    loop {
        let mut next_query = query.clone();
        next_query.cursor = cursor.clone();
        next_query.limit = QUERY_LIMIT_MAX;

        let page = query_log_entries(next_query)?;
        for item in &page.items {
            let line = serde_json::to_string(item)
                .with_context(|| format!("序列化日志导出内容失败: entryId={}", item.id))
                .with_code("log_export_serialize_failed", "序列化日志导出内容失败")
                .with_ctx("entryId", item.id.to_string())?;
            writeln!(writer, "{}", line)
                .with_context(|| format!("写入日志导出文件失败: {}", target_path.display()))
                .with_code("log_export_write_failed", "写入日志导出文件失败")
                .with_ctx("targetPath", target_path.display().to_string())?;
        }

        page_count = page_count.saturating_add(1);
        if page_count.is_multiple_of(EXPORT_FLUSH_EVERY_PAGES) {
            writer
                .flush()
                .with_context(|| format!("刷新日志导出文件失败: {}", target_path.display()))
                .with_code("log_export_flush_failed", "刷新日志导出文件失败")
                .with_ctx("targetPath", target_path.display().to_string())?;
            std::thread::sleep(Duration::from_millis(EXPORT_THROTTLE_SLEEP_MS));
        }

        if page.next_cursor.is_none() {
            break;
        }
        cursor = page.next_cursor;
    }

    writer
        .flush()
        .with_context(|| format!("刷新日志导出文件失败: {}", target_path.display()))
        .with_code("log_export_flush_failed", "刷新日志导出文件失败")
        .with_ctx("targetPath", target_path.display().to_string())?;

    Ok(target_path.to_string_lossy().to_string())
}

#[cfg(test)]
#[path = "../../../tests/infrastructure/logging_tests.rs"]
mod tests;

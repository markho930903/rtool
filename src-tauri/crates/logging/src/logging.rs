use crate::models::{LogConfigDto, LogEntryDto, LogPageDto, LogQueryDto};
use crate::{AppError, ResultExt};
use anyhow::Context;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder as RollingBuilder, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::db::DbConn;

#[path = "config.rs"]
mod config;
#[path = "export.rs"]
mod export;
#[path = "ingest.rs"]
mod ingest;
#[path = "query.rs"]
mod query;
#[path = "store.rs"]
mod store;

pub use ingest::{cleanup_expired_logs, sanitize_for_log, sanitize_json_value, sanitize_path};


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
const LOG_INGEST_QUEUE_CAPACITY: usize = 4096;

const SENSITIVE_TEXT_KEYS: [&str; 5] = ["text", "content", "clipboard", "prompt", "input"];
const SENSITIVE_PATH_KEYS: [&str; 4] = ["path", "file", "filepath", "filename"];
const SENSITIVE_HOST_KEYS: [&str; 2] = ["host", "hostname"];

pub(super) fn default_log_config() -> LogConfigDto {
    LogConfigDto {
        min_level: DEFAULT_MIN_LEVEL.to_string(),
        keep_days: DEFAULT_KEEP_DAYS,
        realtime_enabled: DEFAULT_REALTIME_ENABLED,
        high_freq_window_ms: DEFAULT_HIGH_FREQ_WINDOW_MS,
        high_freq_max_per_key: DEFAULT_HIGH_FREQ_MAX_PER_KEY,
        allow_raw_view: DEFAULT_ALLOW_RAW_VIEW,
    }
}

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
    pub metadata: Option<serde_json::Value>,
    pub raw_ref: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct HighFrequencyWindow {
    started_at: i64,
    count: u32,
    aggregated_row_id: Option<i64>,
    aggregated_count: u32,
}

impl HighFrequencyWindow {
    fn new(started_at: i64) -> Self {
        Self {
            started_at,
            count: 0,
            aggregated_row_id: None,
            aggregated_count: 0,
        }
    }
}

pub trait LoggingEventSink: Send + Sync {
    fn emit_stream(&self, entry: &LogEntryDto) -> Result<(), AppError>;
}

struct LogCenter {
    event_sink: Option<Arc<dyn LoggingEventSink>>,
    db_conn: DbConn,
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

fn log_ingest_sender_slot() -> &'static OnceLock<mpsc::SyncSender<RecordLogInput>> {
    static SLOT: OnceLock<mpsc::SyncSender<RecordLogInput>> = OnceLock::new();
    &SLOT
}

fn log_ingest_sender() -> &'static mpsc::SyncSender<RecordLogInput> {
    log_ingest_sender_slot().get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<RecordLogInput>(LOG_INGEST_QUEUE_CAPACITY);
        if let Err(error) = std::thread::Builder::new()
            .name("rtool-log-ingest".to_string())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        eprintln!("logging runtime init failed: {}", error);
                        return;
                    }
                };
                while let Ok(input) = receiver.recv() {
                    if let Err(error) = runtime.block_on(record_log_event(input)) {
                        eprintln!("logging ingest failed: {}", error);
                    }
                }
            })
        {
            eprintln!("logging ingest thread spawn failed: {}", error);
        }
        sender
    })
}

fn log_ingest_drop_counter() -> &'static AtomicU64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    &COUNTER
}

pub fn resolve_log_level() -> String {
    config::resolve_log_level()
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

pub async fn init_log_center(
    db_conn: DbConn,
    log_dir: PathBuf,
    event_sink: Option<Arc<dyn LoggingEventSink>>,
) -> Result<LogConfigDto, AppError> {
    let config = config::clamp_and_normalize_config(config::load_log_config(&db_conn).await)?;
    config::persist_log_config(&db_conn, &config).await?;

    let center = Arc::new(LogCenter {
        event_sink,
        db_conn,
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

fn get_log_center() -> Result<Arc<LogCenter>, AppError> {
    log_center_slot()
        .get()
        .cloned()
        .ok_or_else(|| AppError::new("log_center_uninitialized", "日志中心未初始化"))
}

pub async fn record_log_event(input: RecordLogInput) -> Result<(), AppError> {
    let center = get_log_center()?;
    center.ingest(input).await
}

pub fn record_log_event_best_effort(input: RecordLogInput) {
    match log_ingest_sender().try_send(input) {
        Ok(()) => {}
        Err(mpsc::TrySendError::Full(_)) => {
            let dropped = log_ingest_drop_counter()
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            if dropped == 1 || dropped % 1000 == 0 {
                eprintln!(
                    "logging enqueue dropped due to full queue: dropped={}, capacity={}",
                    dropped, LOG_INGEST_QUEUE_CAPACITY
                );
            }
        }
        Err(mpsc::TrySendError::Disconnected(_)) => {
            eprintln!("logging enqueue failed: ingest channel disconnected");
        }
    }
}

pub fn get_log_config() -> Result<LogConfigDto, AppError> {
    let center = get_log_center()?;
    center
        .config
        .lock()
        .map(|value| value.clone())
        .map_err(|_| AppError::new("log_config_read_failed", "读取日志配置失败"))
}

pub async fn update_log_config(input: LogConfigDto) -> Result<LogConfigDto, AppError> {
    let normalized = config::clamp_and_normalize_config(input)?;
    let center = get_log_center()?;
    config::persist_log_config(&center.db_conn, &normalized).await?;
    let mut guard = center
        .config
        .lock()
        .map_err(|_| AppError::new("log_config_update_failed", "更新日志配置失败"))?;
    *guard = normalized.clone();
    Ok(normalized)
}

pub async fn query_log_entries(query: LogQueryDto) -> Result<LogPageDto, AppError> {
    let center = get_log_center()?;
    query::query_log_entries(&center, query).await
}

pub async fn export_log_entries(
    query: LogQueryDto,
    output_path: Option<String>,
) -> Result<String, AppError> {
    let center = get_log_center()?;
    export::export_log_entries(&center, query, output_path).await
}

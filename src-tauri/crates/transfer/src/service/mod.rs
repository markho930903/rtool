use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rand::Rng;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep};

use anyhow::Context;
use app_core::models::{
    TransferClearHistoryInputDto, TransferDirection, TransferFileDto, TransferFileInputDto,
    TransferHistoryFilterDto, TransferHistoryPageDto, TransferPairingCodeDto, TransferPeerDto,
    TransferPeerTrustLevel, TransferProgressSnapshotDto, TransferSendFilesInputDto,
    TransferSessionDto, TransferSettingsDto, TransferStatus, TransferUpdateSettingsInputDto,
};
use app_core::{AppError, AppResult, ResultExt};
use app_infra::db::DbConn;
use app_infra::runtime::blocking::run_blocking;
use app_infra::transfer::TRANSFER_LISTEN_PORT;
use app_infra::transfer::archive::{cleanup_temp_paths, collect_sources};
use app_infra::transfer::discovery::{
    DiscoveryPacket, PeerMap, run_broadcast_loop, run_listen_loop,
};
use app_infra::transfer::preview::build_preview;
use app_infra::transfer::protocol::{
    AckFrameItem, CAPABILITY_ACK_BATCH, CAPABILITY_CODEC_BIN, CAPABILITY_PIPELINE, FrameCodec,
    ManifestFileFrame, MissingChunkFrame, PROTOCOL_VERSION, TransferFrame, derive_proof,
    derive_session_key, random_hex, read_frame, read_frame_from, write_frame, write_frame_to,
};
use app_infra::transfer::resume::{
    chunk_count, completed_bytes, empty_bitmap, mark_chunk_done, missing_chunks,
};
use app_infra::transfer::session::{
    ChunkReader, ChunkWriter, build_part_path, file_hash_hex, resolve_conflict_path,
    resolve_target_path,
};
use app_infra::transfer::store::{
    TransferFilePersistItem, cleanup_expired, ensure_session_exists, get_file_bitmap,
    get_peer_by_device_id, insert_or_update_file, insert_session, list_failed_sessions,
    list_history, list_stored_peers, load_settings, mark_peer_pair_failure, mark_peer_pair_success,
    merge_online_peers, save_settings, upsert_files_batch, upsert_peer, upsert_session_progress,
};

mod discovery;
mod event_sink;
mod events;
mod handshake;
mod incoming;
mod incoming_pipeline;
mod manifest_stage;
mod outgoing;
mod outgoing_pipeline;
mod persistence;
mod pipeline;
mod send_preparation;
mod session_control;
mod task_spawner;
const PAIR_CODE_EXPIRE_MS: i64 = 120_000;
const CHUNK_ACK_TIMEOUT_MS: u64 = 3_000;
const MAX_CHUNK_RETRY: u8 = 3;
const TRANSFER_SESSION_CANCELED_CODE: &str = "transfer_session_canceled";

pub use event_sink::{NoopTransferEventSink, TransferEventSink};
pub use task_spawner::{
    NoopTransferTaskSpawner, TokioTransferTaskSpawner, TransferTask, TransferTaskSpawner,
};

fn lock_mutex<'a, T>(lock: &'a Mutex<T>, name: &'static str) -> std::sync::MutexGuard<'a, T> {
    match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                event = "transfer_lock_poisoned",
                lock = name,
                access = "mutex"
            );
            poisoned.into_inner()
        }
    }
}

fn read_lock<'a, T>(lock: &'a RwLock<T>, name: &'static str) -> std::sync::RwLockReadGuard<'a, T> {
    match lock.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                event = "transfer_lock_poisoned",
                lock = name,
                access = "read"
            );
            poisoned.into_inner()
        }
    }
}

fn write_lock<'a, T>(
    lock: &'a RwLock<T>,
    name: &'static str,
) -> std::sync::RwLockWriteGuard<'a, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                event = "transfer_lock_poisoned",
                lock = name,
                access = "write"
            );
            poisoned.into_inner()
        }
    }
}

#[derive(Debug, Clone)]
struct PairCodeEntry {
    code: String,
    expires_at: i64,
}

#[derive(Debug)]
struct RuntimeSessionControl {
    paused_tx: watch::Sender<bool>,
    canceled: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct TransferService {
    event_sink: Arc<dyn TransferEventSink>,
    task_spawner: Arc<dyn TransferTaskSpawner>,
    db_conn: DbConn,
    device_id: String,
    device_name: String,
    settings: Arc<RwLock<TransferSettingsDto>>,
    peers: PeerMap,
    discovery_stop: Arc<AtomicBool>,
    discovery_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    listener_started: Arc<AtomicBool>,
    pair_code: Arc<RwLock<Option<PairCodeEntry>>>,
    session_controls: Arc<RwLock<HashMap<String, RuntimeSessionControl>>>,
    session_pair_codes: Arc<RwLock<HashMap<String, String>>>,
    session_last_emit_ms: Arc<RwLock<HashMap<String, i64>>>,
}

impl TransferService {
    pub async fn new(
        event_sink: Arc<dyn TransferEventSink>,
        task_spawner: Arc<dyn TransferTaskSpawner>,
        db_conn: DbConn,
        app_data_dir: &Path,
    ) -> AppResult<Self> {
        let device_id = resolve_or_create_device_id(&db_conn).await?;
        let device_name = resolve_device_name();
        let default_download_dir = resolve_default_download_dir(app_data_dir);
        let settings = load_settings(&db_conn, default_download_dir).await?;

        let service = Self {
            event_sink,
            task_spawner,
            db_conn,
            device_id,
            device_name,
            settings: Arc::new(RwLock::new(settings)),
            peers: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            discovery_stop: Arc::new(AtomicBool::new(false)),
            discovery_tasks: Arc::new(Mutex::new(Vec::new())),
            listener_started: Arc::new(AtomicBool::new(false)),
            pair_code: Arc::new(RwLock::new(None)),
            session_controls: Arc::new(RwLock::new(HashMap::new())),
            session_pair_codes: Arc::new(RwLock::new(HashMap::new())),
            session_last_emit_ms: Arc::new(RwLock::new(HashMap::new())),
        };

        let _ = cleanup_expired(&service.db_conn, now_millis()).await;
        Ok(service)
    }

    pub fn bootstrap_background_tasks(&self) -> AppResult<()> {
        self.ensure_listener_started()
    }

    pub fn ensure_bootstrapped(&self) -> AppResult<()> {
        self.ensure_listener_started()
    }

    fn spawn_task<F>(&self, task_name: &'static str, fut: F) -> AppResult<JoinHandle<()>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task: TransferTask = Box::pin(fut);
        self.task_spawner.spawn(task_name, task)
    }

    pub fn get_settings(&self) -> TransferSettingsDto {
        read_lock(self.settings.as_ref(), "settings").clone()
    }

    pub async fn update_settings(
        &self,
        input: TransferUpdateSettingsInputDto,
    ) -> AppResult<TransferSettingsDto> {
        let mut next = self.get_settings();

        if let Some(value) = input.default_download_dir {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(AppError::new(
                    "transfer_setting_download_dir_invalid",
                    "默认下载目录不能为空",
                ));
            }
            next.default_download_dir = trimmed.to_string();
        }
        if let Some(value) = input.max_parallel_files {
            next.max_parallel_files = value.clamp(1, 8);
        }
        if let Some(value) = input.max_inflight_chunks {
            next.max_inflight_chunks = value.clamp(1, 64);
        }
        if let Some(value) = input.chunk_size_kb {
            next.chunk_size_kb = value.clamp(64, 4096);
        }
        if let Some(value) = input.auto_cleanup_days {
            next.auto_cleanup_days = value.clamp(1, 365);
        }
        if let Some(value) = input.resume_enabled {
            next.resume_enabled = value;
        }
        if let Some(value) = input.discovery_enabled {
            next.discovery_enabled = value;
        }
        if let Some(value) = input.pairing_required {
            next.pairing_required = value;
        }

        save_settings(&self.db_conn, &next).await?;
        *write_lock(self.settings.as_ref(), "settings") = next.clone();
        Ok(next)
    }

    pub fn generate_pairing_code(&self) -> TransferPairingCodeDto {
        let code = generate_pair_code();
        let expires_at = now_millis() + PAIR_CODE_EXPIRE_MS;

        *write_lock(self.pair_code.as_ref(), "pair_code") = Some(PairCodeEntry {
            code: code.clone(),
            expires_at,
        });

        TransferPairingCodeDto { code, expires_at }
    }

    pub async fn send_files(
        &self,
        input: TransferSendFilesInputDto,
    ) -> AppResult<TransferSessionDto> {
        let prepared = self.prepare_outgoing_send(input).await?;
        self.persist_new_session_with_files(&prepared.session)
            .await?;

        self.attach_outgoing_session_runtime(
            prepared.session.id.as_str(),
            prepared.pair_code.as_str(),
        );
        self.spawn_outgoing_worker(
            prepared.session.id.clone(),
            prepared.peer_address,
            prepared.pair_code,
            prepared.temp_paths,
        )?;

        let created_session_id = prepared.session.id.clone();
        ensure_session_exists(&self.db_conn, created_session_id.as_str()).await
    }

    pub async fn retry_session(&self, session_id: &str) -> AppResult<TransferSessionDto> {
        let session = list_failed_sessions(&self.db_conn, session_id)
            .await?
            .ok_or_else(|| {
                AppError::new("transfer_session_not_retryable", "该会话当前状态不支持重试")
            })?;

        if session.direction != TransferDirection::Send {
            return Err(AppError::new(
                "transfer_session_retry_direction_invalid",
                "当前仅支持重试发送会话",
            ));
        }

        let pair_code = read_lock(self.session_pair_codes.as_ref(), "session_pair_codes")
            .get(session_id)
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "transfer_retry_pair_code_missing",
                    "缺少配对码，请重新发起传输",
                )
            })?;

        let mut inputs = Vec::new();
        for file in session.files {
            if let Some(source_path) = file.source_path {
                inputs.push(TransferFileInputDto {
                    path: source_path,
                    relative_path: Some(file.relative_path),
                    compress_folder: Some(false),
                });
            }
        }

        self.send_files(TransferSendFilesInputDto {
            peer_device_id: session.peer_device_id,
            pair_code,
            files: inputs,
            direction: Some(TransferDirection::Send),
            session_id: Some(session.id),
        })
        .await
    }

    pub async fn list_history(
        &self,
        filter: TransferHistoryFilterDto,
    ) -> AppResult<TransferHistoryPageDto> {
        list_history(&self.db_conn, &filter).await
    }

    pub async fn clear_history(&self, input: TransferClearHistoryInputDto) -> AppResult<()> {
        let all = input.all.unwrap_or(false);
        let older_than_days = input.older_than_days.unwrap_or(30).clamp(1, 365);
        app_infra::transfer::store::clear_history(&self.db_conn, all, older_than_days).await?;
        self.emit_history_sync(if all { "clear_all" } else { "clear_expired" });
        Ok(())
    }
}

async fn resolve_or_create_device_id(conn: &DbConn) -> AppResult<String> {
    let key = "transfer.device_id";
    if let Some(value) = app_infra::db::get_app_setting(conn, key).await? {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let value = uuid::Uuid::new_v4().to_string();
    app_infra::db::set_app_setting(conn, key, value.as_str()).await?;
    Ok(value)
}

fn resolve_device_name() -> String {
    if let Ok(value) = std::env::var("HOSTNAME") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Ok(value) = std::env::var("COMPUTERNAME") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    "rtool-device".to_string()
}

fn resolve_default_download_dir(app_data_dir: &Path) -> String {
    if let Some(home) = dirs_home() {
        let downloads = home.join("Downloads");
        return downloads.to_string_lossy().to_string();
    }
    app_data_dir.join("downloads").to_string_lossy().to_string()
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn generate_pair_code() -> String {
    let mut value = [0u8; 4];
    rand::rng().fill_bytes(&mut value);
    let number = u32::from_be_bytes(value) % 100_000_000;
    format!("{number:08}")
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default()
}

fn calculate_speed(transferred_bytes: u64, started_at: i64) -> u64 {
    let elapsed_ms = (now_millis() - started_at).max(1) as u64;
    transferred_bytes.saturating_mul(1000) / elapsed_ms
}

fn estimate_eta(total_bytes: u64, transferred_bytes: u64, speed_bps: u64) -> Option<u64> {
    if speed_bps == 0 || transferred_bytes >= total_bytes {
        return None;
    }

    Some((total_bytes - transferred_bytes).div_ceil(speed_bps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct RejectSpawner;

    impl TransferTaskSpawner for RejectSpawner {
        fn spawn(&self, task_name: &'static str, _task: TransferTask) -> AppResult<JoinHandle<()>> {
            Err(
                AppError::new("transfer_runtime_unavailable", "传输后台任务运行时不可用")
                    .with_context("task", task_name),
            )
        }
    }

    struct CountingSpawner {
        spawned: AtomicUsize,
        runtime: tokio::runtime::Runtime,
    }

    impl CountingSpawner {
        fn new() -> Self {
            Self {
                spawned: AtomicUsize::new(0),
                runtime: tokio::runtime::Runtime::new().expect("create tokio runtime"),
            }
        }

        fn spawn_count(&self) -> usize {
            self.spawned.load(Ordering::SeqCst)
        }
    }

    impl TransferTaskSpawner for CountingSpawner {
        fn spawn(
            &self,
            _task_name: &'static str,
            _task: TransferTask,
        ) -> AppResult<JoinHandle<()>> {
            self.spawned.fetch_add(1, Ordering::SeqCst);
            Ok(self.runtime.spawn(async {}))
        }
    }

    async fn create_service(
        task_spawner: Arc<dyn TransferTaskSpawner>,
    ) -> AppResult<TransferService> {
        let app_data_dir =
            std::env::temp_dir().join(format!("rtool-transfer-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&app_data_dir).expect("create app data dir");
        let db_path = app_data_dir
            .join("rtool-turso.db")
            .to_string_lossy()
            .to_string();
        let db_conn = app_infra::db::open_db(Path::new(db_path.as_str())).await?;
        app_infra::db::init_db(&db_conn).await?;
        TransferService::new(
            Arc::new(NoopTransferEventSink),
            task_spawner,
            db_conn,
            app_data_dir.as_path(),
        )
        .await
    }

    #[test]
    fn new_should_not_require_tokio_runtime_context() {
        let runtime = tokio::runtime::Runtime::new().expect("create tokio runtime");
        let service = runtime.block_on(create_service(Arc::new(RejectSpawner)));
        assert!(service.is_ok());
    }

    #[test]
    fn bootstrap_background_tasks_should_be_idempotent() {
        let spawner = Arc::new(CountingSpawner::new());
        let runtime = tokio::runtime::Runtime::new().expect("create tokio runtime");
        let service = runtime
            .block_on(create_service(spawner.clone()))
            .expect("create service");

        service
            .bootstrap_background_tasks()
            .expect("bootstrap background tasks first call");
        service
            .bootstrap_background_tasks()
            .expect("bootstrap background tasks second call");

        assert_eq!(spawner.spawn_count(), 1);
    }
}

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use rand::Rng;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep};

use crate::core::models::{
    TransferClearHistoryInputDto, TransferDirection, TransferFileDto, TransferFileInputDto,
    TransferHistoryFilterDto, TransferHistoryPageDto, TransferPairingCodeDto, TransferPeerDto,
    TransferPeerTrustLevel, TransferProgressSnapshotDto, TransferSendFilesInputDto,
    TransferSessionDto, TransferSettingsDto, TransferStatus, TransferUpdateSettingsInputDto,
};
use crate::core::{AppError, AppResult, ResultExt};
use crate::infrastructure::db::DbPool;
use crate::infrastructure::runtime::blocking::run_blocking;
use crate::infrastructure::transfer::TRANSFER_LISTEN_PORT;
use crate::infrastructure::transfer::archive::{cleanup_temp_paths, collect_sources};
use crate::infrastructure::transfer::discovery::{
    DiscoveryPacket, PeerMap, run_broadcast_loop, run_listen_loop,
};
use crate::infrastructure::transfer::preview::build_preview;
use crate::infrastructure::transfer::protocol::{
    AckFrameItem, CAPABILITY_ACK_BATCH_V2, CAPABILITY_CODEC_BIN_V2, CAPABILITY_PIPELINE_V2,
    FrameCodec, ManifestFileFrame, MissingChunkFrame, PROTOCOL_VERSION_V2, TransferFrame,
    derive_proof, derive_session_key, random_hex, read_frame, read_frame_from, write_frame,
    write_frame_to,
};
use crate::infrastructure::transfer::resume::{
    chunk_count, completed_bytes, empty_bitmap, mark_chunk_done, missing_chunks,
};
use crate::infrastructure::transfer::session::{
    ChunkReader, ChunkWriter, build_part_path, file_hash_hex, resolve_conflict_path,
    resolve_target_path,
};
use crate::infrastructure::transfer::store::{
    TransferFilePersistItem, cleanup_expired, ensure_session_exists, get_file_bitmap,
    get_peer_by_device_id, insert_or_update_file, insert_session, list_failed_sessions,
    list_history, list_stored_peers, load_settings, mark_peer_pair_failure, mark_peer_pair_success,
    merge_online_peers, save_settings, upsert_files_batch, upsert_peer, upsert_session_progress,
};
use anyhow::Context;

mod discovery;
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

const TRANSFER_PEER_SYNC_EVENT: &str = "rtool://transfer/peer_sync";
const TRANSFER_SESSION_SYNC_EVENT: &str = "rtool://transfer/session_sync";
const TRANSFER_HISTORY_SYNC_EVENT: &str = "rtool://transfer/history_sync";
const PAIR_CODE_EXPIRE_MS: i64 = 120_000;
const CHUNK_ACK_TIMEOUT_MS: u64 = 3_000;
const MAX_CHUNK_RETRY: u8 = 3;
const TRANSFER_SESSION_CANCELED_CODE: &str = "transfer_session_canceled";

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
    app_handle: AppHandle,
    db_pool: DbPool,
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
    pub fn new(app_handle: AppHandle, db_pool: DbPool, app_data_dir: &Path) -> AppResult<Self> {
        let device_id = resolve_or_create_device_id(&db_pool)?;
        let device_name = resolve_device_name();
        let default_download_dir = resolve_default_download_dir(app_data_dir);
        let settings = load_settings(&db_pool, default_download_dir)?;

        let service = Self {
            app_handle,
            db_pool,
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

        service.ensure_listener_started();
        let _ = cleanup_expired(&service.db_pool, now_millis());
        Ok(service)
    }

    pub fn get_settings(&self) -> TransferSettingsDto {
        read_lock(self.settings.as_ref(), "settings").clone()
    }

    pub fn update_settings(
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

        save_settings(&self.db_pool, &next)?;
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
        );

        let pool = self.db_pool.clone();
        let created_session_id = prepared.session.id.clone();
        run_blocking("transfer_ensure_session", move || {
            ensure_session_exists(&pool, created_session_id.as_str())
        })
        .await
    }

    pub async fn retry_session(&self, session_id: &str) -> AppResult<TransferSessionDto> {
        let session = list_failed_sessions(&self.db_pool, session_id)?.ok_or_else(|| {
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

    pub fn list_history(
        &self,
        filter: TransferHistoryFilterDto,
    ) -> AppResult<TransferHistoryPageDto> {
        list_history(&self.db_pool, &filter)
    }

    pub fn clear_history(&self, input: TransferClearHistoryInputDto) -> AppResult<()> {
        let all = input.all.unwrap_or(false);
        let older_than_days = input.older_than_days.unwrap_or(30).clamp(1, 365);
        crate::infrastructure::transfer::store::clear_history(&self.db_pool, all, older_than_days)?;
        self.emit_history_sync(if all { "clear_all" } else { "clear_expired" });
        Ok(())
    }
}

fn resolve_or_create_device_id(pool: &DbPool) -> AppResult<String> {
    let key = "transfer.device_id";
    if let Some(value) = crate::infrastructure::db::get_app_setting(pool, key)? {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let value = uuid::Uuid::new_v4().to_string();
    crate::infrastructure::db::set_app_setting(pool, key, value.as_str())?;
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

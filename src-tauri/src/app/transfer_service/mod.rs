use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use rand::Rng;
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep};

use crate::core::models::{
    TransferClearHistoryInputDto, TransferFileDto, TransferFileInputDto, TransferHistoryFilterDto,
    TransferHistoryPageDto, TransferPairingCodeDto, TransferPeerDto, TransferProgressSnapshotDto,
    TransferSendFilesInputDto, TransferSessionDto, TransferSettingsDto,
    TransferUpdateSettingsInputDto,
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
    insert_or_update_file, insert_session, list_failed_sessions, list_history, list_stored_peers,
    load_settings, mark_peer_pair_failure, mark_peer_pair_success, merge_online_peers,
    save_settings, upsert_files_batch, upsert_peer, upsert_session_progress,
};
use anyhow::Context;

mod incoming;
mod outgoing;

const TRANSFER_PEER_SYNC_EVENT: &str = "rtool://transfer/peer_sync";
const TRANSFER_SESSION_SYNC_EVENT: &str = "rtool://transfer/session_sync";
const TRANSFER_HISTORY_SYNC_EVENT: &str = "rtool://transfer/history_sync";
const PAIR_CODE_EXPIRE_MS: i64 = 120_000;
const CHUNK_ACK_TIMEOUT_MS: u64 = 3_000;
const MAX_CHUNK_RETRY: u8 = 3;

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistorySyncPayload {
    reason: String,
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
        self.settings.read().expect("settings read").clone()
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
        *self.settings.write().expect("settings write") = next.clone();
        Ok(next)
    }

    pub fn generate_pairing_code(&self) -> TransferPairingCodeDto {
        let code = generate_pair_code();
        let expires_at = now_millis() + PAIR_CODE_EXPIRE_MS;

        *self.pair_code.write().expect("pair code write") = Some(PairCodeEntry {
            code: code.clone(),
            expires_at,
        });

        TransferPairingCodeDto { code, expires_at }
    }

    pub fn start_discovery(&self) {
        self.discovery_stop.store(false, Ordering::Relaxed);

        let settings = self.get_settings();
        if !settings.discovery_enabled {
            return;
        }

        let mut capabilities = vec![
            "chunk".to_string(),
            "resume".to_string(),
            "history".to_string(),
        ];
        if settings.codec_v2_enabled {
            capabilities.push(CAPABILITY_CODEC_BIN_V2.to_string());
        }
        if settings.pipeline_v2_enabled {
            capabilities.push(CAPABILITY_ACK_BATCH_V2.to_string());
            capabilities.push(CAPABILITY_PIPELINE_V2.to_string());
        }

        let packet = DiscoveryPacket {
            device_id: self.device_id.clone(),
            display_name: self.device_name.clone(),
            listen_port: TRANSFER_LISTEN_PORT,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            pairing_required: settings.pairing_required,
            capabilities,
            ts: now_millis(),
        };

        let stop_a = self.discovery_stop.clone();
        let task_broadcast = tauri::async_runtime::spawn(async move {
            run_broadcast_loop(stop_a, packet).await;
        });

        let stop_b = self.discovery_stop.clone();
        let peers = self.peers.clone();
        let local_device_id = self.device_id.clone();
        let task_listen = tauri::async_runtime::spawn(async move {
            run_listen_loop(stop_b, peers, local_device_id).await;
        });

        let mut tasks = self.discovery_tasks.lock().expect("tasks lock");
        tasks.push(task_broadcast);
        tasks.push(task_listen);

        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut ticker = interval(Duration::from_secs(2));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            ticker.tick().await;
            loop {
                if service.discovery_stop.load(Ordering::Relaxed) {
                    break;
                }
                let _ = service.emit_peer_sync().await;
                ticker.tick().await;
            }
        });
    }

    pub fn stop_discovery(&self) {
        self.discovery_stop.store(true, Ordering::Relaxed);

        let mut tasks = self.discovery_tasks.lock().expect("tasks lock");
        for task in tasks.drain(..) {
            task.abort();
        }
    }

    pub async fn list_peers(&self) -> AppResult<Vec<TransferPeerDto>> {
        let online = self.collect_online_peers().await;
        let pool = self.db_pool.clone();
        let online_for_upsert = online.clone();
        let _ = run_blocking("transfer_upsert_peers", move || {
            for peer in &online_for_upsert {
                let _ = upsert_peer(&pool, peer);
            }
            Ok(())
        })
        .await;

        let pool = self.db_pool.clone();
        let stored = run_blocking("transfer_list_stored_peers", move || {
            list_stored_peers(&pool)
        })
        .await?;
        Ok(merge_online_peers(stored, online.as_slice()))
    }

    pub async fn send_files(
        &self,
        input: TransferSendFilesInputDto,
    ) -> AppResult<TransferSessionDto> {
        let peers = self.list_peers().await?;
        let peer = peers
            .into_iter()
            .find(|value| value.device_id == input.peer_device_id)
            .ok_or_else(|| {
                AppError::new("transfer_peer_not_found", "未找到目标设备")
                    .with_context("peerDeviceId", input.peer_device_id.clone())
            })?;

        let session_id = input
            .session_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let settings = self.get_settings();
        let input_files = input.files.clone();
        let bundle = run_blocking("transfer_collect_sources", move || {
            collect_sources(input_files.as_slice())
        })
        .await?;
        let cleanup_after_at = now_millis() + i64::from(settings.auto_cleanup_days) * 86_400_000;

        let mut files = Vec::with_capacity(bundle.files.len());
        let mut total_bytes = 0u64;
        for source in &bundle.files {
            let source = source.clone();
            let session_for_manifest = session_id.clone();
            let settings_for_manifest = settings.clone();
            let file_dto = run_blocking("transfer_prepare_manifest_file", move || {
                let source_path = PathBuf::from(source.source_path.as_str());
                let hash = file_hash_hex(source_path.as_path())?;
                let chunk_size = settings_for_manifest.chunk_size_kb.saturating_mul(1024);
                let chunk_count = chunk_count(source.size_bytes, chunk_size);
                let (mime_type, preview_kind, preview_data) = build_preview(source_path.as_path());
                Ok(TransferFileDto {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: session_for_manifest,
                    relative_path: source.relative_path,
                    source_path: Some(source.source_path),
                    target_path: None,
                    size_bytes: source.size_bytes,
                    transferred_bytes: 0,
                    chunk_size,
                    chunk_count,
                    status: "queued".to_string(),
                    blake3: Some(hash),
                    mime_type,
                    preview_kind,
                    preview_data,
                    is_folder_archive: source.is_folder_archive,
                })
            })
            .await?;
            total_bytes = total_bytes.saturating_add(file_dto.size_bytes);
            files.push(file_dto);
        }

        let session = TransferSessionDto {
            id: session_id.clone(),
            direction: input.direction.unwrap_or_else(|| "send".to_string()),
            peer_device_id: peer.device_id.clone(),
            peer_name: peer.display_name.clone(),
            status: "queued".to_string(),
            total_bytes,
            transferred_bytes: 0,
            avg_speed_bps: 0,
            save_dir: settings.default_download_dir.clone(),
            created_at: now_millis(),
            started_at: None,
            finished_at: None,
            error_code: None,
            error_message: None,
            cleanup_after_at: Some(cleanup_after_at),
            files,
        };

        let pool = self.db_pool.clone();
        let session_for_store = session.clone();
        run_blocking("transfer_insert_session", move || {
            insert_session(&pool, &session_for_store)?;
            for file in &session_for_store.files {
                let bitmap = empty_bitmap(file.chunk_count);
                insert_or_update_file(&pool, file, bitmap.as_slice())?;
            }
            Ok(())
        })
        .await?;

        self.register_session_control(session.id.as_str());
        self.session_pair_codes
            .write()
            .expect("pair code map")
            .insert(session.id.clone(), input.pair_code.clone());

        let service = self.clone();
        let peer_address = format!("{}:{}", peer.address, peer.listen_port);
        let pair_code = input.pair_code;
        let temp_paths = bundle.temp_paths;
        let spawned_session_id = session.id.clone();
        tauri::async_runtime::spawn(async move {
            let session_id = spawned_session_id;
            if let Err(error) = service
                .run_outgoing_session(
                    session_id.as_str(),
                    peer_address.as_str(),
                    pair_code.as_str(),
                )
                .await
            {
                tracing::error!(
                    event = "transfer_send_failed",
                    session_id = session_id,
                    error_code = error.code,
                    error_detail = error.causes.first().map(String::as_str).unwrap_or_default()
                );
                let _ = service
                    .update_session_failure(session_id.as_str(), &error)
                    .await;
            }
            let _ = run_blocking("transfer_cleanup_temp_paths", move || {
                cleanup_temp_paths(temp_paths.as_slice());
                Ok(())
            })
            .await;
            service.unregister_session_control(session_id.as_str());
        });

        let pool = self.db_pool.clone();
        let created_session_id = session.id.clone();
        run_blocking("transfer_ensure_session", move || {
            ensure_session_exists(&pool, created_session_id.as_str())
        })
        .await
    }

    pub fn pause_session(&self, session_id: &str) -> AppResult<()> {
        let controls = self.session_controls.read().expect("session controls read");
        let control = controls
            .get(session_id)
            .ok_or_else(|| AppError::new("transfer_session_not_running", "会话未运行，无法暂停"))?;
        let _ = control.paused_tx.send(true);
        let mut session = ensure_session_exists(&self.db_pool, session_id)?;
        session.status = "paused".to_string();
        insert_session(&self.db_pool, &session)?;
        self.maybe_emit_session_snapshot(&session, None, 0, None, true, None, None, None, None);
        Ok(())
    }

    pub fn resume_session(&self, session_id: &str) -> AppResult<()> {
        let controls = self.session_controls.read().expect("session controls read");
        let control = controls
            .get(session_id)
            .ok_or_else(|| AppError::new("transfer_session_not_running", "会话未运行，无法继续"))?;
        let _ = control.paused_tx.send(false);
        let mut session = ensure_session_exists(&self.db_pool, session_id)?;
        session.status = "running".to_string();
        insert_session(&self.db_pool, &session)?;
        self.maybe_emit_session_snapshot(&session, None, 0, None, true, None, None, None, None);
        Ok(())
    }

    pub fn cancel_session(&self, session_id: &str) -> AppResult<()> {
        let controls = self.session_controls.read().expect("session controls read");
        let control = controls
            .get(session_id)
            .ok_or_else(|| AppError::new("transfer_session_not_running", "会话未运行，无法取消"))?;
        control.canceled.store(true, Ordering::Relaxed);

        let mut session = ensure_session_exists(&self.db_pool, session_id)?;
        session.status = "canceled".to_string();
        session.finished_at = Some(now_millis());
        insert_session(&self.db_pool, &session)?;
        self.maybe_emit_session_snapshot(&session, None, 0, None, true, None, None, None, None);
        Ok(())
    }

    pub async fn retry_session(&self, session_id: &str) -> AppResult<TransferSessionDto> {
        let session = list_failed_sessions(&self.db_pool, session_id)?.ok_or_else(|| {
            AppError::new("transfer_session_not_retryable", "该会话当前状态不支持重试")
        })?;

        if session.direction != "send" {
            return Err(AppError::new(
                "transfer_session_retry_direction_invalid",
                "当前仅支持重试发送会话",
            ));
        }

        let pair_code = self
            .session_pair_codes
            .read()
            .expect("pair code map read")
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
            direction: Some("send".to_string()),
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

    fn ensure_listener_started(&self) {
        if self.listener_started.swap(true, Ordering::Relaxed) {
            return;
        }

        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            let listener = match TcpListener::bind(("0.0.0.0", TRANSFER_LISTEN_PORT)).await {
                Ok(value) => value,
                Err(error) => {
                    tracing::error!(
                        event = "transfer_listener_bind_failed",
                        error = error.to_string()
                    );
                    return;
                }
            };

            loop {
                match listener.accept().await {
                    Ok((stream, address)) => {
                        let service_inner = service.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = service_inner.handle_incoming(stream).await {
                                tracing::warn!(
                                    event = "transfer_incoming_session_failed",
                                    address = address.to_string(),
                                    error_code = error.code,
                                    error_detail = error
                                        .causes
                                        .first()
                                        .map(String::as_str)
                                        .unwrap_or_default()
                                );
                            }
                        });
                    }
                    Err(error) => {
                        tracing::warn!(
                            event = "transfer_listener_accept_failed",
                            error = error.to_string()
                        );
                        sleep(Duration::from_millis(250)).await;
                    }
                }
            }
        });
    }

    async fn emit_peer_sync(&self) -> AppResult<()> {
        let peers = self.list_peers().await?;
        self.app_handle
            .emit(TRANSFER_PEER_SYNC_EVENT, peers)
            .with_context(|| format!("推送设备列表失败: event={TRANSFER_PEER_SYNC_EVENT}"))
            .with_code("transfer_event_emit_failed", "推送设备列表失败")
            .with_ctx("event", TRANSFER_PEER_SYNC_EVENT)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_session_snapshot(
        &self,
        session: &TransferSessionDto,
        active_file_id: Option<String>,
        speed_bps: u64,
        eta_seconds: Option<u64>,
        protocol_version: Option<u16>,
        codec: Option<FrameCodec>,
        inflight_chunks: Option<u32>,
        retransmit_chunks: Option<u32>,
    ) {
        let payload = TransferProgressSnapshotDto {
            session: session.clone(),
            active_file_id,
            speed_bps,
            eta_seconds,
            protocol_version,
            codec: codec.map(|value| value.as_str().to_string()),
            inflight_chunks,
            retransmit_chunks,
        };
        if let Err(error) = self.app_handle.emit(TRANSFER_SESSION_SYNC_EVENT, payload) {
            tracing::warn!(
                event = "transfer_event_emit_failed",
                event_name = TRANSFER_SESSION_SYNC_EVENT,
                error = error.to_string()
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn maybe_emit_session_snapshot(
        &self,
        session: &TransferSessionDto,
        active_file_id: Option<String>,
        speed_bps: u64,
        eta_seconds: Option<u64>,
        force: bool,
        protocol_version: Option<u16>,
        codec: Option<FrameCodec>,
        inflight_chunks: Option<u32>,
        retransmit_chunks: Option<u32>,
    ) {
        let settings = self.get_settings();
        let now = now_millis();
        let should_emit = if force {
            true
        } else {
            let mut guard = self
                .session_last_emit_ms
                .write()
                .expect("session emit map write");
            let interval = i64::from(settings.event_emit_interval_ms.max(50));
            let last = guard.get(session.id.as_str()).copied().unwrap_or_default();
            if now - last >= interval {
                guard.insert(session.id.clone(), now);
                true
            } else {
                false
            }
        };

        if should_emit {
            self.emit_session_snapshot(
                session,
                active_file_id,
                speed_bps,
                eta_seconds,
                protocol_version,
                codec,
                inflight_chunks,
                retransmit_chunks,
            );
        }
    }

    fn emit_history_sync(&self, reason: &str) {
        if let Err(error) = self.app_handle.emit(
            TRANSFER_HISTORY_SYNC_EVENT,
            HistorySyncPayload {
                reason: reason.to_string(),
            },
        ) {
            tracing::warn!(
                event = "transfer_event_emit_failed",
                event_name = TRANSFER_HISTORY_SYNC_EVENT,
                error = error.to_string()
            );
        }
    }

    async fn collect_online_peers(&self) -> Vec<TransferPeerDto> {
        let peers = self.peers.read().await;
        peers
            .values()
            .map(|peer| TransferPeerDto {
                device_id: peer.device_id.clone(),
                display_name: peer.display_name.clone(),
                address: peer.address.clone(),
                listen_port: peer.listen_port,
                last_seen_at: peer.last_seen_at,
                paired_at: None,
                trust_level: "online".to_string(),
                failed_attempts: 0,
                blocked_until: None,
                pairing_required: peer.pairing_required,
                online: true,
            })
            .collect()
    }

    fn register_session_control(&self, session_id: &str) {
        let (paused_tx, _) = watch::channel(false);
        self.session_controls
            .write()
            .expect("session controls write")
            .insert(
                session_id.to_string(),
                RuntimeSessionControl {
                    paused_tx,
                    canceled: Arc::new(AtomicBool::new(false)),
                },
            );
    }

    fn unregister_session_control(&self, session_id: &str) {
        self.session_controls
            .write()
            .expect("session controls write")
            .remove(session_id);
        self.session_last_emit_ms
            .write()
            .expect("session emit map write")
            .remove(session_id);
    }

    fn read_session_control(&self, session_id: &str) -> Option<RuntimeSessionControl> {
        self.session_controls
            .read()
            .expect("session controls read")
            .get(session_id)
            .map(|value| RuntimeSessionControl {
                paused_tx: value.paused_tx.clone(),
                canceled: value.canceled.clone(),
            })
    }

    async fn blocking_ensure_session_exists(
        &self,
        session_id: String,
    ) -> AppResult<TransferSessionDto> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_ensure_session_exists", move || {
            ensure_session_exists(&pool, session_id.as_str())
        })
        .await
    }

    async fn blocking_get_file_bitmap(
        &self,
        session_id: String,
        file_id: String,
    ) -> AppResult<Option<Vec<u8>>> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_get_file_bitmap", move || {
            get_file_bitmap(&pool, session_id.as_str(), file_id.as_str())
        })
        .await
    }

    async fn blocking_upsert_files_batch(
        &self,
        items: Vec<TransferFilePersistItem>,
    ) -> AppResult<()> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_upsert_files_batch", move || {
            upsert_files_batch(&pool, items.as_slice())
        })
        .await
    }

    async fn blocking_upsert_session_progress(&self, session: TransferSessionDto) -> AppResult<()> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_upsert_session_progress", move || {
            upsert_session_progress(&pool, &session)
        })
        .await
    }

    async fn blocking_insert_or_update_file(
        &self,
        file: TransferFileDto,
        completed_bitmap: Vec<u8>,
    ) -> AppResult<()> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_insert_or_update_file", move || {
            insert_or_update_file(&pool, &file, completed_bitmap.as_slice())
        })
        .await
    }

    async fn validate_pair_code(&self, peer_device_id: &str, pair_code: &str) -> AppResult<()> {
        let settings = self.get_settings();
        if !settings.pairing_required {
            return Ok(());
        }

        let current = self.pair_code.read().expect("pair code read").clone();
        let Some(entry) = current else {
            return Err(AppError::new(
                "transfer_pair_code_missing",
                "接收端尚未生成配对码",
            ));
        };

        if now_millis() > entry.expires_at {
            return Err(AppError::new("transfer_pair_code_expired", "配对码已过期"));
        }

        if entry.code != pair_code {
            let pool = self.db_pool.clone();
            let device_id = peer_device_id.to_string();
            run_blocking("transfer_mark_pair_failure", move || {
                mark_peer_pair_failure(&pool, device_id.as_str(), Some(now_millis() + 60_000))
            })
            .await?;
            return Err(AppError::new("transfer_pair_code_invalid", "配对码错误"));
        }

        Ok(())
    }

    fn is_session_canceled(&self, session_id: &str) -> bool {
        self.read_session_control(session_id)
            .map(|value| value.canceled.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    async fn wait_if_paused(&self, session_id: &str) {
        loop {
            let Some(control) = self.read_session_control(session_id) else {
                return;
            };
            if !*control.paused_tx.borrow() {
                return;
            }

            let mut paused_rx = control.paused_tx.subscribe();
            if !*paused_rx.borrow() {
                return;
            }

            if paused_rx.changed().await.is_err() {
                return;
            }
        }
    }

    async fn update_session_failure(&self, session_id: &str, error: &AppError) -> AppResult<()> {
        let mut session = self
            .blocking_ensure_session_exists(session_id.to_string())
            .await?;
        session.status = "failed".to_string();
        session.error_code = Some(error.code.clone());
        session.error_message = Some(error.message.clone());
        session.finished_at = Some(now_millis());
        let pool = self.db_pool.clone();
        let session_for_store = session.clone();
        run_blocking("transfer_insert_failed_session", move || {
            insert_session(&pool, &session_for_store)
        })
        .await?;
        self.maybe_emit_session_snapshot(&session, None, 0, None, true, None, None, None, None);
        self.emit_history_sync("session_failed");
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

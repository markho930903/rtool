use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use rand::RngCore;
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Instant, sleep};

use crate::core::models::{
    TransferClearHistoryInputDto, TransferFileDto, TransferFileInputDto, TransferHistoryFilterDto,
    TransferHistoryPageDto, TransferPairingCodeDto, TransferPeerDto, TransferProgressSnapshotDto,
    TransferSendFilesInputDto, TransferSessionDto, TransferSettingsDto,
    TransferUpdateSettingsInputDto,
};
use crate::core::{AppError, AppResult};
use crate::infrastructure::db::DbPool;
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
    paused: Arc<AtomicBool>,
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
            loop {
                if service.discovery_stop.load(Ordering::Relaxed) {
                    break;
                }
                let _ = service.emit_peer_sync().await;
                sleep(Duration::from_secs(2)).await;
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
        for peer in &online {
            let _ = upsert_peer(&self.db_pool, peer);
        }
        let stored = list_stored_peers(&self.db_pool)?;
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
                    .with_detail(format!("peer_device_id={}", input.peer_device_id))
            })?;

        let session_id = input
            .session_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let bundle = collect_sources(input.files.as_slice())?;
        let settings = self.get_settings();
        let cleanup_after_at = now_millis() + i64::from(settings.auto_cleanup_days) * 86_400_000;

        let mut files = Vec::new();
        let mut total_bytes = 0u64;
        for source in &bundle.files {
            let hash = file_hash_hex(PathBuf::from(source.source_path.as_str()).as_path())?;
            let chunk_size = settings.chunk_size_kb.saturating_mul(1024);
            let chunk_count = chunk_count(source.size_bytes, chunk_size);
            let path = PathBuf::from(source.source_path.as_str());
            let (mime_type, preview_kind, preview_data) = build_preview(path.as_path());
            let file_dto = TransferFileDto {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: session_id.clone(),
                relative_path: source.relative_path.clone(),
                source_path: Some(source.source_path.clone()),
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
            };
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

        insert_session(&self.db_pool, &session)?;
        for file in &session.files {
            let bitmap = empty_bitmap(file.chunk_count);
            insert_or_update_file(&self.db_pool, file, bitmap.as_slice())?;
        }

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
                    error_detail = error.detail.as_deref().unwrap_or_default()
                );
                let _ = service.update_session_failure(session_id.as_str(), &error);
            }
            cleanup_temp_paths(temp_paths.as_slice());
            service.unregister_session_control(session_id.as_str());
        });

        Ok(ensure_session_exists(&self.db_pool, session.id.as_str())?)
    }

    pub fn pause_session(&self, session_id: &str) -> AppResult<()> {
        let controls = self.session_controls.read().expect("session controls read");
        let control = controls
            .get(session_id)
            .ok_or_else(|| AppError::new("transfer_session_not_running", "会话未运行，无法暂停"))?;
        control.paused.store(true, Ordering::Relaxed);
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
        control.paused.store(false, Ordering::Relaxed);
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
                                    error_detail = error.detail.unwrap_or_default()
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
            .map_err(|error| {
                AppError::new("transfer_event_emit_failed", "推送设备列表失败")
                    .with_detail(error.to_string())
            })
    }

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
        self.session_controls
            .write()
            .expect("session controls write")
            .insert(
                session_id.to_string(),
                RuntimeSessionControl {
                    paused: Arc::new(AtomicBool::new(false)),
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
                paused: value.paused.clone(),
                canceled: value.canceled.clone(),
            })
    }

    async fn run_outgoing_session(
        &self,
        session_id: &str,
        peer_address: &str,
        pair_code: &str,
    ) -> AppResult<()> {
        #[derive(Debug)]
        struct OutgoingFileRuntime {
            file: TransferFileDto,
            bitmap: Vec<u8>,
            reader: ChunkReader,
            remaining_chunks: u32,
            file_done_sent: bool,
        }

        #[derive(Debug)]
        struct InflightChunk {
            file_idx: usize,
            chunk_index: u32,
            sent_at: Instant,
            retries: u8,
        }

        let settings = self.get_settings();
        let mut stream = TcpStream::connect(peer_address).await.map_err(|error| {
            AppError::new("transfer_peer_connect_failed", "连接目标设备失败")
                .with_detail(format!("{peer_address}: {error}"))
        })?;

        let local_capabilities = vec![
            CAPABILITY_CODEC_BIN_V2.to_string(),
            CAPABILITY_ACK_BATCH_V2.to_string(),
            CAPABILITY_PIPELINE_V2.to_string(),
        ];
        let client_nonce = random_hex(16);
        write_frame(
            &mut stream,
            &TransferFrame::Hello {
                device_id: self.device_id.clone(),
                device_name: self.device_name.clone(),
                nonce: client_nonce.clone(),
                protocol_version: Some(PROTOCOL_VERSION_V2),
                capabilities: Some(local_capabilities.clone()),
            },
            None,
        )
        .await?;

        let challenge = read_frame(&mut stream, None).await?;
        let server_nonce = match challenge {
            TransferFrame::AuthChallenge { nonce, .. } => nonce,
            TransferFrame::Error { code, message } => {
                return Err(AppError::new(code, "目标设备拒绝连接").with_detail(message));
            }
            other => {
                return Err(AppError::new(
                    "transfer_protocol_challenge_invalid",
                    "握手挑战帧不合法",
                )
                .with_detail(format!("unexpected frame: {other:?}")));
            }
        };

        let proof = derive_proof(pair_code, client_nonce.as_str(), server_nonce.as_str());
        write_frame(
            &mut stream,
            &TransferFrame::AuthResponse {
                pair_code: pair_code.to_string(),
                proof,
            },
            None,
        )
        .await?;

        let (peer_protocol_version, peer_capabilities) = match read_frame(&mut stream, None).await?
        {
            TransferFrame::AuthOk {
                protocol_version,
                capabilities,
                ..
            } => (
                protocol_version.unwrap_or(1),
                capabilities.unwrap_or_default(),
            ),
            TransferFrame::Error { code, message } => {
                return Err(AppError::new(code, "认证失败").with_detail(message));
            }
            other => {
                return Err(
                    AppError::new("transfer_protocol_auth_invalid", "认证响应帧不合法")
                        .with_detail(format!("unexpected frame: {other:?}")),
                );
            }
        };

        let codec = if settings.codec_v2_enabled
            && peer_protocol_version >= PROTOCOL_VERSION_V2
            && peer_capabilities
                .iter()
                .any(|value| value == CAPABILITY_CODEC_BIN_V2)
        {
            FrameCodec::BinV2
        } else {
            FrameCodec::JsonV1
        };
        tracing::info!(
            event = "transfer_protocol_negotiated",
            session_id = session_id,
            peer_address,
            codec = codec.as_str(),
            peer_protocol_version
        );

        let session_key =
            derive_session_key(pair_code, client_nonce.as_str(), server_nonce.as_str());

        let mut session = ensure_session_exists(&self.db_pool, session_id)?;
        session.status = "running".to_string();
        session.started_at = Some(now_millis());
        upsert_session_progress(&self.db_pool, &session)?;

        let manifest_files = session
            .files
            .iter()
            .map(|file| ManifestFileFrame {
                file_id: file.id.clone(),
                relative_path: file.relative_path.clone(),
                size_bytes: file.size_bytes,
                chunk_size: file.chunk_size,
                chunk_count: file.chunk_count,
                blake3: file.blake3.clone().unwrap_or_default(),
                mime_type: file.mime_type.clone(),
                is_folder_archive: file.is_folder_archive,
            })
            .collect::<Vec<_>>();

        let (mut reader, mut writer) = stream.into_split();
        write_frame_to(
            &mut writer,
            &TransferFrame::Manifest {
                session_id: session.id.clone(),
                direction: session.direction.clone(),
                save_dir: session.save_dir.clone(),
                files: manifest_files,
            },
            Some(&session_key),
            codec,
        )
        .await?;

        let mut missing_by_file = HashMap::<String, Vec<u32>>::new();
        let manifest_ack = read_frame_from(&mut reader, Some(&session_key), Some(codec)).await?;
        match manifest_ack {
            TransferFrame::ManifestAck {
                session_id: ack_session_id,
                missing_chunks,
            } if ack_session_id == session.id => {
                for item in missing_chunks {
                    missing_by_file.insert(item.file_id, item.missing_chunk_indexes);
                }
            }
            TransferFrame::Error { code, message } => {
                return Err(AppError::new(code, "接收端拒绝文件清单").with_detail(message));
            }
            other => {
                return Err(AppError::new(
                    "transfer_protocol_manifest_ack_invalid",
                    "MANIFEST_ACK 帧不合法",
                )
                .with_detail(format!("unexpected frame: {other:?}")));
            }
        }

        let mut runtimes = Vec::<OutgoingFileRuntime>::new();
        let mut file_id_to_idx = HashMap::<String, usize>::new();
        let mut fair_queue = VecDeque::<(usize, u32)>::new();
        let mut per_file_missing = Vec::<VecDeque<u32>>::new();

        for (index, file) in session.files.iter_mut().enumerate() {
            let bitmap = get_file_bitmap(&self.db_pool, session.id.as_str(), file.id.as_str())
                .unwrap_or_default()
                .unwrap_or_else(|| empty_bitmap(file.chunk_count));
            let source_path = PathBuf::from(file.source_path.clone().unwrap_or_default());
            let mut missing = missing_by_file
                .get(file.id.as_str())
                .cloned()
                .unwrap_or_else(|| missing_chunks(bitmap.as_slice(), file.chunk_count));
            missing.sort_unstable();
            file.status = "running".to_string();
            file.transferred_bytes = completed_bytes(
                bitmap.as_slice(),
                file.chunk_count,
                file.chunk_size,
                file.size_bytes,
            );
            file_id_to_idx.insert(file.id.clone(), index);
            per_file_missing.push(VecDeque::from(missing.clone()));
            runtimes.push(OutgoingFileRuntime {
                file: file.clone(),
                bitmap,
                reader: ChunkReader::open(source_path.as_path()).await?,
                remaining_chunks: missing.len() as u32,
                file_done_sent: false,
            });
        }

        loop {
            let mut progressed = false;
            for (idx, queue) in per_file_missing.iter_mut().enumerate() {
                if let Some(chunk_index) = queue.pop_front() {
                    fair_queue.push_back((idx, chunk_index));
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }

        session.transferred_bytes = session
            .files
            .iter()
            .map(|item| item.transferred_bytes)
            .sum();
        let start_at = session.started_at.unwrap_or_else(now_millis);

        let mut inflight = HashMap::<(usize, u32), InflightChunk>::new();
        let mut retry_counts = HashMap::<(usize, u32), u8>::new();
        let mut retransmit_chunks = 0u32;
        let mut dirty_files = HashMap::<String, TransferFilePersistItem>::new();
        let mut last_db_flush = Instant::now();
        let db_flush_interval =
            Duration::from_millis(settings.db_flush_interval_ms.max(100) as u64);
        let max_inflight_chunks = if settings.pipeline_v2_enabled {
            settings.max_inflight_chunks.max(1) as usize
        } else {
            1
        };

        while !fair_queue.is_empty() || !inflight.is_empty() {
            self.wait_if_paused(session.id.as_str()).await;
            if self.is_session_canceled(session.id.as_str()) {
                return Err(AppError::new("transfer_session_canceled", "传输已取消"));
            }

            while inflight.len() < max_inflight_chunks {
                let Some((file_idx, chunk_index)) = fair_queue.pop_front() else {
                    break;
                };
                if inflight.contains_key(&(file_idx, chunk_index)) {
                    continue;
                }
                let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                    AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                })?;
                if runtime.file_done_sent || runtime.remaining_chunks == 0 {
                    continue;
                }
                if crate::infrastructure::transfer::resume::is_chunk_done(
                    runtime.bitmap.as_slice(),
                    chunk_index,
                ) {
                    continue;
                }

                let bytes = runtime
                    .reader
                    .read_chunk(chunk_index, runtime.file.chunk_size)
                    .await?;
                let hash = blake3::hash(bytes.as_slice()).to_hex().to_string();
                let frame = match codec {
                    FrameCodec::JsonV1 => TransferFrame::Chunk {
                        session_id: session.id.clone(),
                        file_id: runtime.file.id.clone(),
                        chunk_index,
                        total_chunks: runtime.file.chunk_count,
                        hash,
                        data: base64::engine::general_purpose::STANDARD.encode(bytes.as_slice()),
                    },
                    FrameCodec::BinV2 => TransferFrame::ChunkBinary {
                        session_id: session.id.clone(),
                        file_id: runtime.file.id.clone(),
                        chunk_index,
                        total_chunks: runtime.file.chunk_count,
                        hash,
                        data: bytes,
                    },
                };
                write_frame_to(&mut writer, &frame, Some(&session_key), codec).await?;
                inflight.insert(
                    (file_idx, chunk_index),
                    InflightChunk {
                        file_idx,
                        chunk_index,
                        sent_at: Instant::now(),
                        retries: retry_counts
                            .get(&(file_idx, chunk_index))
                            .copied()
                            .unwrap_or_default(),
                    },
                );
            }

            match tokio::time::timeout(
                Duration::from_millis(40),
                read_frame_from(&mut reader, Some(&session_key), Some(codec)),
            )
            .await
            {
                Ok(Ok(frame)) => {
                    let mut ack_items = Vec::<AckFrameItem>::new();
                    match frame {
                        TransferFrame::Ack {
                            session_id: ack_session_id,
                            file_id,
                            chunk_index,
                            ok,
                            error,
                        } if ack_session_id == session.id => {
                            ack_items.push(AckFrameItem {
                                file_id,
                                chunk_index,
                                ok,
                                error,
                            });
                        }
                        TransferFrame::AckBatch {
                            session_id: ack_session_id,
                            items,
                        } if ack_session_id == session.id => {
                            ack_items.extend(items);
                        }
                        TransferFrame::Error { code, message } => {
                            return Err(
                                AppError::new(code, "目标设备返回错误").with_detail(message)
                            );
                        }
                        TransferFrame::Ping { .. } => {}
                        _ => {}
                    }

                    for ack in ack_items {
                        let Some(file_idx) = file_id_to_idx.get(ack.file_id.as_str()).copied()
                        else {
                            continue;
                        };
                        let key = (file_idx, ack.chunk_index);
                        let Some(inflight_chunk) = inflight.remove(&key) else {
                            continue;
                        };
                        if ack.ok {
                            retry_counts.remove(&key);
                            let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                                AppError::new(
                                    "transfer_runtime_file_missing",
                                    "传输文件运行时状态不存在",
                                )
                            })?;
                            if !crate::infrastructure::transfer::resume::is_chunk_done(
                                runtime.bitmap.as_slice(),
                                ack.chunk_index,
                            ) {
                                mark_chunk_done(runtime.bitmap.as_mut_slice(), ack.chunk_index)?;
                                let previous = runtime.file.transferred_bytes;
                                runtime.file.transferred_bytes = completed_bytes(
                                    runtime.bitmap.as_slice(),
                                    runtime.file.chunk_count,
                                    runtime.file.chunk_size,
                                    runtime.file.size_bytes,
                                );
                                runtime.file.status = "running".to_string();
                                if runtime.file.transferred_bytes > previous {
                                    session.transferred_bytes = session
                                        .transferred_bytes
                                        .saturating_add(runtime.file.transferred_bytes - previous);
                                }
                                if runtime.remaining_chunks > 0 {
                                    runtime.remaining_chunks -= 1;
                                }

                                session.avg_speed_bps =
                                    calculate_speed(session.transferred_bytes, start_at);
                                session.files[file_idx] = runtime.file.clone();
                                dirty_files.insert(
                                    runtime.file.id.clone(),
                                    TransferFilePersistItem {
                                        file: runtime.file.clone(),
                                        completed_bitmap: runtime.bitmap.clone(),
                                    },
                                );
                            }

                            if runtime.remaining_chunks == 0 && !runtime.file_done_sent {
                                runtime.file.status = "success".to_string();
                                runtime.file.transferred_bytes = runtime.file.size_bytes;
                                session.files[file_idx] = runtime.file.clone();
                                dirty_files.insert(
                                    runtime.file.id.clone(),
                                    TransferFilePersistItem {
                                        file: runtime.file.clone(),
                                        completed_bitmap: runtime.bitmap.clone(),
                                    },
                                );
                                write_frame_to(
                                    &mut writer,
                                    &TransferFrame::FileDone {
                                        session_id: session.id.clone(),
                                        file_id: runtime.file.id.clone(),
                                        blake3: runtime.file.blake3.clone().unwrap_or_default(),
                                    },
                                    Some(&session_key),
                                    codec,
                                )
                                .await?;
                                runtime.file_done_sent = true;
                            }
                        } else {
                            let retry = inflight_chunk.retries.saturating_add(1);
                            if retry > MAX_CHUNK_RETRY {
                                return Err(AppError::new(
                                    "transfer_chunk_retry_exhausted",
                                    "分块重试次数已耗尽",
                                )
                                .with_detail(format!(
                                    "file_idx={}, chunk_index={}",
                                    inflight_chunk.file_idx, inflight_chunk.chunk_index
                                )));
                            }
                            retransmit_chunks = retransmit_chunks.saturating_add(1);
                            tracing::warn!(
                                event = "transfer_chunk_requeue_failed_ack",
                                session_id = session.id,
                                file_id = ack.file_id,
                                chunk_index = ack.chunk_index,
                                retry
                            );
                            retry_counts.insert(key, retry);
                            fair_queue.push_front(key);
                        }
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => {}
            }

            let mut timeout_chunks = Vec::new();
            for (key, value) in &inflight {
                if value.sent_at.elapsed() >= Duration::from_millis(CHUNK_ACK_TIMEOUT_MS) {
                    timeout_chunks.push(*key);
                }
            }
            for key in timeout_chunks {
                if let Some(old) = inflight.remove(&key) {
                    let retry = old.retries.saturating_add(1);
                    if retry > MAX_CHUNK_RETRY {
                        return Err(AppError::new(
                            "transfer_chunk_ack_timeout",
                            "分块确认超时且超过重试上限",
                        )
                        .with_detail(format!("file_idx={}, chunk_index={}", key.0, key.1)));
                    }
                    retransmit_chunks = retransmit_chunks.saturating_add(1);
                    tracing::warn!(
                        event = "transfer_chunk_requeue_timeout",
                        session_id = session.id,
                        file_idx = key.0,
                        chunk_index = key.1,
                        retry
                    );
                    retry_counts.insert(key, retry);
                    fair_queue.push_front(key);
                }
            }

            if last_db_flush.elapsed() >= db_flush_interval {
                if !dirty_files.is_empty() {
                    let items = dirty_files.values().cloned().collect::<Vec<_>>();
                    upsert_files_batch(&self.db_pool, items.as_slice())?;
                    dirty_files.clear();
                }
                upsert_session_progress(&self.db_pool, &session)?;
                last_db_flush = Instant::now();
            }

            let eta = estimate_eta(
                session.total_bytes,
                session.transferred_bytes,
                session.avg_speed_bps,
            );
            self.maybe_emit_session_snapshot(
                &session,
                None,
                session.avg_speed_bps,
                eta,
                false,
                Some(if codec == FrameCodec::BinV2 {
                    PROTOCOL_VERSION_V2
                } else {
                    1
                }),
                Some(codec),
                Some(inflight.len() as u32),
                Some(retransmit_chunks),
            );
        }

        if !dirty_files.is_empty() {
            let items = dirty_files.values().cloned().collect::<Vec<_>>();
            upsert_files_batch(&self.db_pool, items.as_slice())?;
        }

        write_frame_to(
            &mut writer,
            &TransferFrame::SessionDone {
                session_id: session.id.clone(),
                ok: true,
                error: None,
            },
            Some(&session_key),
            codec,
        )
        .await?;

        session.status = "success".to_string();
        session.transferred_bytes = session.total_bytes;
        session.avg_speed_bps = calculate_speed(session.transferred_bytes, start_at);
        session.finished_at = Some(now_millis());
        session.error_code = None;
        session.error_message = None;
        upsert_session_progress(&self.db_pool, &session)?;
        self.maybe_emit_session_snapshot(
            &session,
            None,
            session.avg_speed_bps,
            Some(0),
            true,
            Some(if codec == FrameCodec::BinV2 {
                PROTOCOL_VERSION_V2
            } else {
                1
            }),
            Some(codec),
            Some(0),
            Some(retransmit_chunks),
        );
        self.emit_history_sync("session_done");
        Ok(())
    }

    async fn handle_incoming(&self, mut stream: TcpStream) -> AppResult<()> {
        #[derive(Debug)]
        struct IncomingFileRuntime {
            file: TransferFileDto,
            bitmap: Vec<u8>,
            writer: ChunkWriter,
        }

        let settings = self.get_settings();
        let hello = read_frame(&mut stream, None).await?;
        let (peer_device_id, peer_name, client_nonce, peer_protocol_version, peer_capabilities) =
            match hello {
                TransferFrame::Hello {
                    device_id,
                    device_name,
                    nonce,
                    protocol_version,
                    capabilities,
                } => (
                    device_id,
                    device_name,
                    nonce,
                    protocol_version.unwrap_or(1),
                    capabilities.unwrap_or_default(),
                ),
                _ => {
                    return Err(AppError::new(
                        "transfer_protocol_hello_invalid",
                        "无效的 HELLO 帧",
                    ));
                }
            };

        let local_capabilities = vec![
            CAPABILITY_CODEC_BIN_V2.to_string(),
            CAPABILITY_ACK_BATCH_V2.to_string(),
            CAPABILITY_PIPELINE_V2.to_string(),
        ];
        let codec = if settings.codec_v2_enabled
            && peer_protocol_version >= PROTOCOL_VERSION_V2
            && peer_capabilities
                .iter()
                .any(|value| value == CAPABILITY_CODEC_BIN_V2)
        {
            FrameCodec::BinV2
        } else {
            FrameCodec::JsonV1
        };
        let ack_batch_enabled = settings.pipeline_v2_enabled
            && codec == FrameCodec::BinV2
            && peer_capabilities
                .iter()
                .any(|value| value == CAPABILITY_ACK_BATCH_V2);
        tracing::info!(
            event = "transfer_protocol_negotiated_incoming",
            peer_device_id,
            codec = codec.as_str(),
            peer_protocol_version,
            ack_batch_enabled
        );

        let server_nonce = random_hex(16);
        let expires_at = now_millis() + PAIR_CODE_EXPIRE_MS;
        write_frame(
            &mut stream,
            &TransferFrame::AuthChallenge {
                nonce: server_nonce.clone(),
                expires_at,
            },
            None,
        )
        .await?;

        let auth = read_frame(&mut stream, None).await?;
        let (pair_code, proof) = match auth {
            TransferFrame::AuthResponse { pair_code, proof } => (pair_code, proof),
            _ => {
                return Err(AppError::new(
                    "transfer_protocol_auth_response_invalid",
                    "无效的 AUTH_RESPONSE 帧",
                ));
            }
        };

        self.validate_pair_code(peer_device_id.as_str(), pair_code.as_str())?;
        let expected = derive_proof(
            pair_code.as_str(),
            client_nonce.as_str(),
            server_nonce.as_str(),
        );
        if proof != expected {
            mark_peer_pair_failure(
                &self.db_pool,
                peer_device_id.as_str(),
                Some(now_millis() + 60_000),
            )?;
            write_frame(
                &mut stream,
                &TransferFrame::Error {
                    code: "transfer_auth_failed".to_string(),
                    message: "配对码校验失败".to_string(),
                },
                None,
            )
            .await?;
            return Err(AppError::new("transfer_auth_failed", "配对码校验失败"));
        }

        mark_peer_pair_success(&self.db_pool, peer_device_id.as_str(), now_millis())?;
        write_frame(
            &mut stream,
            &TransferFrame::AuthOk {
                peer_device_id: self.device_id.clone(),
                peer_name: self.device_name.clone(),
                protocol_version: Some(PROTOCOL_VERSION_V2),
                capabilities: Some(local_capabilities),
            },
            None,
        )
        .await?;

        let session_key = derive_session_key(
            pair_code.as_str(),
            client_nonce.as_str(),
            server_nonce.as_str(),
        );
        let (mut reader, mut writer) = stream.into_split();

        let manifest = read_frame_from(&mut reader, Some(&session_key), Some(codec)).await?;
        let (session_id, direction, save_dir, files) = match manifest {
            TransferFrame::Manifest {
                session_id,
                direction,
                save_dir,
                files,
            } => (session_id, direction, save_dir, files),
            _ => {
                return Err(AppError::new(
                    "transfer_protocol_manifest_invalid",
                    "无效的 MANIFEST 帧",
                ));
            }
        };

        let cleanup_after_at = now_millis() + i64::from(settings.auto_cleanup_days) * 86_400_000;
        let total_bytes = files.iter().map(|value| value.size_bytes).sum::<u64>();

        let mut session = TransferSessionDto {
            id: session_id.clone(),
            direction: if direction == "receive" {
                "send".to_string()
            } else {
                "receive".to_string()
            },
            peer_device_id: peer_device_id.clone(),
            peer_name: peer_name.clone(),
            status: "running".to_string(),
            total_bytes,
            transferred_bytes: 0,
            avg_speed_bps: 0,
            save_dir: save_dir.clone(),
            created_at: now_millis(),
            started_at: Some(now_millis()),
            finished_at: None,
            error_code: None,
            error_message: None,
            cleanup_after_at: Some(cleanup_after_at),
            files: Vec::new(),
        };
        upsert_session_progress(&self.db_pool, &session)?;

        let save_dir_path = PathBuf::from(settings.default_download_dir);
        let mut missing_chunks_payload = Vec::new();
        let mut runtimes = Vec::<IncomingFileRuntime>::new();
        let mut file_id_to_idx = HashMap::<String, usize>::new();

        for manifest_file in files {
            let mut bitmap = get_file_bitmap(
                &self.db_pool,
                session.id.as_str(),
                manifest_file.file_id.as_str(),
            )
            .unwrap_or_default()
            .unwrap_or_else(|| empty_bitmap(manifest_file.chunk_count));
            if bitmap.is_empty() {
                bitmap = empty_bitmap(manifest_file.chunk_count);
            }

            let target_path = resolve_target_path(
                save_dir_path.as_path(),
                manifest_file.relative_path.as_str(),
            );
            let part_path = build_part_path(
                save_dir_path.as_path(),
                session.id.as_str(),
                manifest_file.relative_path.as_str(),
            );
            let missing = missing_chunks(bitmap.as_slice(), manifest_file.chunk_count);
            missing_chunks_payload.push(MissingChunkFrame {
                file_id: manifest_file.file_id.clone(),
                missing_chunk_indexes: missing,
            });

            let file = TransferFileDto {
                id: manifest_file.file_id,
                session_id: session.id.clone(),
                relative_path: manifest_file.relative_path,
                source_path: None,
                target_path: Some(target_path.to_string_lossy().to_string()),
                size_bytes: manifest_file.size_bytes,
                transferred_bytes: completed_bytes(
                    bitmap.as_slice(),
                    manifest_file.chunk_count,
                    manifest_file.chunk_size,
                    manifest_file.size_bytes,
                ),
                chunk_size: manifest_file.chunk_size,
                chunk_count: manifest_file.chunk_count,
                status: "running".to_string(),
                blake3: Some(manifest_file.blake3),
                mime_type: manifest_file.mime_type,
                preview_kind: None,
                preview_data: Some(part_path.to_string_lossy().to_string()),
                is_folder_archive: manifest_file.is_folder_archive,
            };
            insert_or_update_file(&self.db_pool, &file, bitmap.as_slice())?;
            file_id_to_idx.insert(file.id.clone(), runtimes.len());
            let writer = ChunkWriter::open(part_path.as_path(), Some(file.size_bytes)).await?;
            runtimes.push(IncomingFileRuntime {
                file: file.clone(),
                bitmap,
                writer,
            });
            session.files.push(file);
        }

        write_frame_to(
            &mut writer,
            &TransferFrame::ManifestAck {
                session_id: session.id.clone(),
                missing_chunks: missing_chunks_payload,
            },
            Some(&session_key),
            codec,
        )
        .await?;

        let started_at = now_millis();
        let mut ack_buffer = Vec::<AckFrameItem>::new();
        let mut last_ack_flush = Instant::now();
        let ack_flush_interval =
            Duration::from_millis(settings.ack_flush_interval_ms.max(5) as u64);
        let db_flush_interval =
            Duration::from_millis(settings.db_flush_interval_ms.max(100) as u64);
        let mut last_db_flush = Instant::now();
        let mut dirty_files = HashMap::<String, TransferFilePersistItem>::new();

        loop {
            let frame = read_frame_from(&mut reader, Some(&session_key), Some(codec)).await?;
            match frame {
                TransferFrame::Chunk {
                    session_id: incoming_session_id,
                    file_id,
                    chunk_index,
                    hash,
                    data,
                    ..
                } => {
                    if incoming_session_id != session.id {
                        continue;
                    }
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(data.as_bytes())
                        .map_err(|error| {
                            AppError::new("transfer_chunk_decode_failed", "分块解码失败")
                                .with_detail(error.to_string())
                        })?;
                    let Some(file_idx) = file_id_to_idx.get(file_id.as_str()).copied() else {
                        continue;
                    };
                    let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                        AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                    })?;
                    let calculated_hash = blake3::hash(decoded.as_slice()).to_hex().to_string();
                    if calculated_hash != hash {
                        ack_buffer.push(AckFrameItem {
                            file_id,
                            chunk_index,
                            ok: false,
                            error: Some("chunk_hash_mismatch".to_string()),
                        });
                        continue;
                    }
                    runtime
                        .writer
                        .write_chunk(chunk_index, runtime.file.chunk_size, decoded.as_slice())
                        .await?;
                    if !crate::infrastructure::transfer::resume::is_chunk_done(
                        runtime.bitmap.as_slice(),
                        chunk_index,
                    ) {
                        mark_chunk_done(runtime.bitmap.as_mut_slice(), chunk_index)?;
                        let previous = runtime.file.transferred_bytes;
                        runtime.file.transferred_bytes = completed_bytes(
                            runtime.bitmap.as_slice(),
                            runtime.file.chunk_count,
                            runtime.file.chunk_size,
                            runtime.file.size_bytes,
                        );
                        runtime.file.status = "running".to_string();
                        if runtime.file.transferred_bytes > previous {
                            session.transferred_bytes = session
                                .transferred_bytes
                                .saturating_add(runtime.file.transferred_bytes - previous);
                        }
                        session.files[file_idx] = runtime.file.clone();
                        dirty_files.insert(
                            runtime.file.id.clone(),
                            TransferFilePersistItem {
                                file: runtime.file.clone(),
                                completed_bitmap: runtime.bitmap.clone(),
                            },
                        );
                    }
                    ack_buffer.push(AckFrameItem {
                        file_id: runtime.file.id.clone(),
                        chunk_index,
                        ok: true,
                        error: None,
                    });
                    session.avg_speed_bps = calculate_speed(session.transferred_bytes, started_at);
                    let eta = estimate_eta(
                        session.total_bytes,
                        session.transferred_bytes,
                        session.avg_speed_bps,
                    );
                    self.maybe_emit_session_snapshot(
                        &session,
                        Some(runtime.file.id.clone()),
                        session.avg_speed_bps,
                        eta,
                        false,
                        Some(if codec == FrameCodec::BinV2 {
                            PROTOCOL_VERSION_V2
                        } else {
                            1
                        }),
                        Some(codec),
                        None,
                        None,
                    );
                }
                TransferFrame::ChunkBinary {
                    session_id: incoming_session_id,
                    file_id,
                    chunk_index,
                    hash,
                    data,
                    ..
                } => {
                    if incoming_session_id != session.id {
                        continue;
                    }
                    let Some(file_idx) = file_id_to_idx.get(file_id.as_str()).copied() else {
                        continue;
                    };
                    let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                        AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                    })?;
                    let calculated_hash = blake3::hash(data.as_slice()).to_hex().to_string();
                    if calculated_hash != hash {
                        ack_buffer.push(AckFrameItem {
                            file_id,
                            chunk_index,
                            ok: false,
                            error: Some("chunk_hash_mismatch".to_string()),
                        });
                        continue;
                    }
                    runtime
                        .writer
                        .write_chunk(chunk_index, runtime.file.chunk_size, data.as_slice())
                        .await?;
                    if !crate::infrastructure::transfer::resume::is_chunk_done(
                        runtime.bitmap.as_slice(),
                        chunk_index,
                    ) {
                        mark_chunk_done(runtime.bitmap.as_mut_slice(), chunk_index)?;
                        let previous = runtime.file.transferred_bytes;
                        runtime.file.transferred_bytes = completed_bytes(
                            runtime.bitmap.as_slice(),
                            runtime.file.chunk_count,
                            runtime.file.chunk_size,
                            runtime.file.size_bytes,
                        );
                        runtime.file.status = "running".to_string();
                        if runtime.file.transferred_bytes > previous {
                            session.transferred_bytes = session
                                .transferred_bytes
                                .saturating_add(runtime.file.transferred_bytes - previous);
                        }
                        session.files[file_idx] = runtime.file.clone();
                        dirty_files.insert(
                            runtime.file.id.clone(),
                            TransferFilePersistItem {
                                file: runtime.file.clone(),
                                completed_bitmap: runtime.bitmap.clone(),
                            },
                        );
                    }
                    ack_buffer.push(AckFrameItem {
                        file_id: runtime.file.id.clone(),
                        chunk_index,
                        ok: true,
                        error: None,
                    });
                    session.avg_speed_bps = calculate_speed(session.transferred_bytes, started_at);
                    let eta = estimate_eta(
                        session.total_bytes,
                        session.transferred_bytes,
                        session.avg_speed_bps,
                    );
                    self.maybe_emit_session_snapshot(
                        &session,
                        Some(runtime.file.id.clone()),
                        session.avg_speed_bps,
                        eta,
                        false,
                        Some(if codec == FrameCodec::BinV2 {
                            PROTOCOL_VERSION_V2
                        } else {
                            1
                        }),
                        Some(codec),
                        None,
                        None,
                    );
                }
                TransferFrame::FileDone {
                    session_id: incoming_session_id,
                    file_id,
                    blake3,
                } => {
                    if incoming_session_id != session.id {
                        continue;
                    }
                    let Some(file_idx) = file_id_to_idx.get(file_id.as_str()).copied() else {
                        continue;
                    };
                    let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                        AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                    })?;
                    runtime.writer.flush().await?;
                    let part_path =
                        PathBuf::from(runtime.file.preview_data.clone().unwrap_or_default());
                    let source_hash = file_hash_hex(part_path.as_path())?;
                    if source_hash != blake3 {
                        runtime.file.status = "failed".to_string();
                        insert_or_update_file(
                            &self.db_pool,
                            &runtime.file,
                            empty_bitmap(runtime.file.chunk_count).as_slice(),
                        )?;
                        return Err(AppError::new("transfer_file_hash_mismatch", "文件校验失败")
                            .with_detail(format!("file_id={}", runtime.file.id)));
                    }

                    let target =
                        PathBuf::from(runtime.file.target_path.clone().unwrap_or_default());
                    let final_path = resolve_conflict_path(target.as_path());
                    if let Some(parent) = final_path.parent() {
                        tokio::fs::create_dir_all(parent).await.map_err(|error| {
                            AppError::new("transfer_target_dir_create_failed", "创建目标目录失败")
                                .with_detail(error.to_string())
                        })?;
                    }
                    tokio::fs::rename(part_path.as_path(), final_path.as_path())
                        .await
                        .map_err(|error| {
                            AppError::new("transfer_target_rename_failed", "落盘文件失败")
                                .with_detail(error.to_string())
                        })?;

                    runtime.file.target_path = Some(final_path.to_string_lossy().to_string());
                    runtime.file.preview_data = runtime.file.target_path.clone();
                    runtime.file.transferred_bytes = runtime.file.size_bytes;
                    runtime.file.status = "success".to_string();
                    session.files[file_idx] = runtime.file.clone();
                    dirty_files.insert(
                        runtime.file.id.clone(),
                        TransferFilePersistItem {
                            file: runtime.file.clone(),
                            completed_bitmap: runtime.bitmap.clone(),
                        },
                    );
                }
                TransferFrame::SessionDone {
                    session_id: incoming_session_id,
                    ok,
                    error,
                } => {
                    if incoming_session_id != session.id {
                        continue;
                    }
                    if !ack_buffer.is_empty() {
                        if ack_batch_enabled {
                            write_frame_to(
                                &mut writer,
                                &TransferFrame::AckBatch {
                                    session_id: session.id.clone(),
                                    items: std::mem::take(&mut ack_buffer),
                                },
                                Some(&session_key),
                                codec,
                            )
                            .await?;
                        } else {
                            for item in std::mem::take(&mut ack_buffer) {
                                write_frame_to(
                                    &mut writer,
                                    &TransferFrame::Ack {
                                        session_id: session.id.clone(),
                                        file_id: item.file_id,
                                        chunk_index: item.chunk_index,
                                        ok: item.ok,
                                        error: item.error,
                                    },
                                    Some(&session_key),
                                    codec,
                                )
                                .await?;
                            }
                        }
                    }

                    if !dirty_files.is_empty() {
                        let items = dirty_files.values().cloned().collect::<Vec<_>>();
                        upsert_files_batch(&self.db_pool, items.as_slice())?;
                    }

                    session.finished_at = Some(now_millis());
                    session.transferred_bytes = session
                        .files
                        .iter()
                        .map(|value| value.transferred_bytes)
                        .sum();
                    if ok {
                        session.status = "success".to_string();
                        session.error_code = None;
                        session.error_message = None;
                    } else {
                        session.status = "failed".to_string();
                        session.error_code = Some("remote_failed".to_string());
                        session.error_message = error;
                    }
                    upsert_session_progress(&self.db_pool, &session)?;
                    self.maybe_emit_session_snapshot(
                        &session,
                        None,
                        session.avg_speed_bps,
                        Some(0),
                        true,
                        Some(if codec == FrameCodec::BinV2 {
                            PROTOCOL_VERSION_V2
                        } else {
                            1
                        }),
                        Some(codec),
                        None,
                        None,
                    );
                    self.emit_history_sync("incoming_done");
                    break;
                }
                TransferFrame::Ping { .. } => {}
                TransferFrame::Error { code, message } => {
                    session.status = "failed".to_string();
                    session.error_code = Some(code);
                    session.error_message = Some(message);
                    session.finished_at = Some(now_millis());
                    upsert_session_progress(&self.db_pool, &session)?;
                    self.maybe_emit_session_snapshot(
                        &session,
                        None,
                        0,
                        None,
                        true,
                        Some(if codec == FrameCodec::BinV2 {
                            PROTOCOL_VERSION_V2
                        } else {
                            1
                        }),
                        Some(codec),
                        None,
                        None,
                    );
                    break;
                }
                _ => {}
            }

            if !ack_buffer.is_empty()
                && (ack_buffer.len() >= settings.ack_batch_size as usize
                    || last_ack_flush.elapsed() >= ack_flush_interval)
            {
                if ack_batch_enabled {
                    write_frame_to(
                        &mut writer,
                        &TransferFrame::AckBatch {
                            session_id: session.id.clone(),
                            items: std::mem::take(&mut ack_buffer),
                        },
                        Some(&session_key),
                        codec,
                    )
                    .await?;
                } else {
                    for item in std::mem::take(&mut ack_buffer) {
                        write_frame_to(
                            &mut writer,
                            &TransferFrame::Ack {
                                session_id: session.id.clone(),
                                file_id: item.file_id,
                                chunk_index: item.chunk_index,
                                ok: item.ok,
                                error: item.error,
                            },
                            Some(&session_key),
                            codec,
                        )
                        .await?;
                    }
                }
                last_ack_flush = Instant::now();
            }

            if last_db_flush.elapsed() >= db_flush_interval {
                if !dirty_files.is_empty() {
                    let items = dirty_files.values().cloned().collect::<Vec<_>>();
                    upsert_files_batch(&self.db_pool, items.as_slice())?;
                    dirty_files.clear();
                }
                upsert_session_progress(&self.db_pool, &session)?;
                last_db_flush = Instant::now();
            }
        }

        Ok(())
    }

    fn validate_pair_code(&self, peer_device_id: &str, pair_code: &str) -> AppResult<()> {
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
            mark_peer_pair_failure(&self.db_pool, peer_device_id, Some(now_millis() + 60_000))?;
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
            let paused = self
                .read_session_control(session_id)
                .map(|value| value.paused.load(Ordering::Relaxed))
                .unwrap_or(false);
            if !paused {
                return;
            }
            sleep(Duration::from_millis(200)).await;
        }
    }

    fn update_session_failure(&self, session_id: &str, error: &AppError) -> AppResult<()> {
        let mut session = ensure_session_exists(&self.db_pool, session_id)?;
        session.status = "failed".to_string();
        session.error_code = Some(error.code.clone());
        session.error_message = Some(error.message.clone());
        session.finished_at = Some(now_millis());
        insert_session(&self.db_pool, &session)?;
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

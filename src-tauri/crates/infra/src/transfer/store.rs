use crate::db::{DbConn, get_app_settings_batch, set_app_settings_batch};
use app_core::models::{
    TransferDirection, TransferFileDto, TransferHistoryFilterDto, TransferHistoryPageDto,
    TransferPeerDto, TransferPeerTrustLevel, TransferSessionDto, TransferSettingsDto,
    TransferStatus,
};
use app_core::{AppError, AppResult};
use libsql::{params, params_from_iter};

pub const TRANSFER_DEFAULT_DOWNLOAD_DIR_KEY: &str = "transfer.default_download_dir";
pub const TRANSFER_MAX_PARALLEL_FILES_KEY: &str = "transfer.max_parallel_files";
pub const TRANSFER_MAX_INFLIGHT_CHUNKS_KEY: &str = "transfer.max_inflight_chunks";
pub const TRANSFER_CHUNK_SIZE_KB_KEY: &str = "transfer.chunk_size_kb";
pub const TRANSFER_AUTO_CLEANUP_DAYS_KEY: &str = "transfer.auto_cleanup_days";
pub const TRANSFER_RESUME_ENABLED_KEY: &str = "transfer.resume_enabled";
pub const TRANSFER_DISCOVERY_ENABLED_KEY: &str = "transfer.discovery_enabled";
pub const TRANSFER_PAIRING_REQUIRED_KEY: &str = "transfer.pairing_required";
pub const TRANSFER_DB_FLUSH_INTERVAL_MS_KEY: &str = "transfer.db_flush_interval_ms";
pub const TRANSFER_EVENT_EMIT_INTERVAL_MS_KEY: &str = "transfer.event_emit_interval_ms";
pub const TRANSFER_ACK_BATCH_SIZE_KEY: &str = "transfer.ack_batch_size";
pub const TRANSFER_ACK_FLUSH_INTERVAL_MS_KEY: &str = "transfer.ack_flush_interval_ms";

const HISTORY_LIMIT_MAX: u32 = 200;

fn to_db_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn from_db_i64(value: i64) -> u64 {
    value.max(0) as u64
}

fn to_bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn parse_bool(value: Option<String>, default: bool) -> bool {
    value
        .and_then(|raw| raw.parse::<bool>().ok())
        .unwrap_or(default)
}

fn parse_u32(value: Option<String>, default: u32) -> u32 {
    value
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(default)
}

fn parse_transfer_status(raw: String, source_field: &'static str) -> AppResult<TransferStatus> {
    TransferStatus::from_db(raw.as_str()).map_err(|error| error.with_context("sourceField", source_field))
}

fn parse_transfer_direction(
    raw: String,
    source_field: &'static str,
) -> AppResult<TransferDirection> {
    TransferDirection::from_db(raw.as_str())
        .map_err(|error| error.with_context("sourceField", source_field))
}

fn parse_transfer_trust_level(
    raw: String,
    source_field: &'static str,
) -> AppResult<TransferPeerTrustLevel> {
    TransferPeerTrustLevel::from_db(raw.as_str())
        .map_err(|error| error.with_context("sourceField", source_field))
}

pub async fn load_settings(conn: &DbConn, default_download_dir: String) -> AppResult<TransferSettingsDto> {
    let mut settings = TransferSettingsDto {
        default_download_dir,
        max_parallel_files: 2,
        max_inflight_chunks: 16,
        chunk_size_kb: 1024,
        auto_cleanup_days: 30,
        resume_enabled: true,
        discovery_enabled: true,
        pairing_required: true,
        db_flush_interval_ms: 400,
        event_emit_interval_ms: 250,
        ack_batch_size: 64,
        ack_flush_interval_ms: 20,
    };

    let keys = [
        TRANSFER_DEFAULT_DOWNLOAD_DIR_KEY,
        TRANSFER_MAX_PARALLEL_FILES_KEY,
        TRANSFER_MAX_INFLIGHT_CHUNKS_KEY,
        TRANSFER_CHUNK_SIZE_KB_KEY,
        TRANSFER_AUTO_CLEANUP_DAYS_KEY,
        TRANSFER_RESUME_ENABLED_KEY,
        TRANSFER_DISCOVERY_ENABLED_KEY,
        TRANSFER_PAIRING_REQUIRED_KEY,
        TRANSFER_DB_FLUSH_INTERVAL_MS_KEY,
        TRANSFER_EVENT_EMIT_INTERVAL_MS_KEY,
        TRANSFER_ACK_BATCH_SIZE_KEY,
        TRANSFER_ACK_FLUSH_INTERVAL_MS_KEY,
    ];
    let values = get_app_settings_batch(conn, &keys).await?;

    if let Some(value) = values.get(TRANSFER_DEFAULT_DOWNLOAD_DIR_KEY) {
        settings.default_download_dir = value.to_string();
    }
    settings.max_parallel_files = parse_u32(
        values.get(TRANSFER_MAX_PARALLEL_FILES_KEY).cloned(),
        2,
    )
    .clamp(1, 8);
    settings.max_inflight_chunks = parse_u32(
        values.get(TRANSFER_MAX_INFLIGHT_CHUNKS_KEY).cloned(),
        16,
    )
    .clamp(1, 64);
    settings.chunk_size_kb = parse_u32(values.get(TRANSFER_CHUNK_SIZE_KB_KEY).cloned(), 1024)
        .clamp(64, 4096);
    settings.auto_cleanup_days = parse_u32(
        values.get(TRANSFER_AUTO_CLEANUP_DAYS_KEY).cloned(),
        30,
    )
    .clamp(1, 365);
    settings.resume_enabled = parse_bool(values.get(TRANSFER_RESUME_ENABLED_KEY).cloned(), true);
    settings.discovery_enabled = parse_bool(
        values.get(TRANSFER_DISCOVERY_ENABLED_KEY).cloned(),
        true,
    );
    settings.pairing_required =
        parse_bool(values.get(TRANSFER_PAIRING_REQUIRED_KEY).cloned(), true);
    settings.db_flush_interval_ms = parse_u32(
        values.get(TRANSFER_DB_FLUSH_INTERVAL_MS_KEY).cloned(),
        400,
    )
    .clamp(100, 5000);
    settings.event_emit_interval_ms = parse_u32(
        values.get(TRANSFER_EVENT_EMIT_INTERVAL_MS_KEY).cloned(),
        250,
    )
    .clamp(100, 2000);
    settings.ack_batch_size = parse_u32(values.get(TRANSFER_ACK_BATCH_SIZE_KEY).cloned(), 64)
        .clamp(1, 512);
    settings.ack_flush_interval_ms = parse_u32(
        values.get(TRANSFER_ACK_FLUSH_INTERVAL_MS_KEY).cloned(),
        20,
    )
    .clamp(5, 2000);

    save_settings(conn, &settings).await?;
    Ok(settings)
}

pub async fn save_settings(conn: &DbConn, settings: &TransferSettingsDto) -> AppResult<()> {
    let max_parallel_files = settings.max_parallel_files.to_string();
    let max_inflight_chunks = settings.max_inflight_chunks.to_string();
    let chunk_size_kb = settings.chunk_size_kb.to_string();
    let auto_cleanup_days = settings.auto_cleanup_days.to_string();
    let db_flush_interval_ms = settings.db_flush_interval_ms.to_string();
    let event_emit_interval_ms = settings.event_emit_interval_ms.to_string();
    let ack_batch_size = settings.ack_batch_size.to_string();
    let ack_flush_interval_ms = settings.ack_flush_interval_ms.to_string();

    let entries = [
        (
            TRANSFER_DEFAULT_DOWNLOAD_DIR_KEY,
            settings.default_download_dir.as_str(),
        ),
        (TRANSFER_MAX_PARALLEL_FILES_KEY, max_parallel_files.as_str()),
        (
            TRANSFER_MAX_INFLIGHT_CHUNKS_KEY,
            max_inflight_chunks.as_str(),
        ),
        (TRANSFER_CHUNK_SIZE_KB_KEY, chunk_size_kb.as_str()),
        (TRANSFER_AUTO_CLEANUP_DAYS_KEY, auto_cleanup_days.as_str()),
        (
            TRANSFER_RESUME_ENABLED_KEY,
            to_bool_string(settings.resume_enabled),
        ),
        (
            TRANSFER_DISCOVERY_ENABLED_KEY,
            to_bool_string(settings.discovery_enabled),
        ),
        (
            TRANSFER_PAIRING_REQUIRED_KEY,
            to_bool_string(settings.pairing_required),
        ),
        (
            TRANSFER_DB_FLUSH_INTERVAL_MS_KEY,
            db_flush_interval_ms.as_str(),
        ),
        (
            TRANSFER_EVENT_EMIT_INTERVAL_MS_KEY,
            event_emit_interval_ms.as_str(),
        ),
        (TRANSFER_ACK_BATCH_SIZE_KEY, ack_batch_size.as_str()),
        (
            TRANSFER_ACK_FLUSH_INTERVAL_MS_KEY,
            ack_flush_interval_ms.as_str(),
        ),
    ];
    set_app_settings_batch(conn, entries.as_slice()).await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TransferFilePersistItem {
    pub file: TransferFileDto,
    pub completed_bitmap: Vec<u8>,
}

pub async fn upsert_peer(conn: &DbConn, peer: &TransferPeerDto) -> AppResult<()> {
    conn.execute(
        "INSERT INTO transfer_peers (device_id, display_name, last_seen_at, paired_at, trust_level, failed_attempts, blocked_until)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(device_id) DO UPDATE SET
           display_name = excluded.display_name,
           last_seen_at = excluded.last_seen_at",
        params![
            peer.device_id.as_str(),
            peer.display_name.as_str(),
            peer.last_seen_at,
            peer.paired_at,
            peer.trust_level.as_str(),
            peer.failed_attempts,
            peer.blocked_until,
        ],
    )
    .await?;
    Ok(())
}

pub async fn get_peer_by_device_id(conn: &DbConn, device_id: &str) -> AppResult<Option<TransferPeerDto>> {
    let mut rows = conn
        .query(
            "SELECT device_id, display_name, last_seen_at, paired_at, trust_level, failed_attempts, blocked_until
             FROM transfer_peers
             WHERE device_id = ?1
             LIMIT 1",
            params![device_id],
        )
        .await?;

    let Some(row) = rows.next().await? else {
        return Ok(None);
    };

    Ok(Some(TransferPeerDto {
        device_id: row.get::<String>(0)?,
        display_name: row.get::<String>(1)?,
        address: String::new(),
        listen_port: 0,
        last_seen_at: row.get::<i64>(2)?,
        paired_at: row.get::<Option<i64>>(3)?,
        trust_level: parse_transfer_trust_level(
            row.get::<String>(4)?,
            "transfer_peers.trust_level",
        )?,
        failed_attempts: row.get::<u32>(5)?,
        blocked_until: row.get::<Option<i64>>(6)?,
        pairing_required: true,
        online: false,
    }))
}

pub async fn mark_peer_pair_success(conn: &DbConn, device_id: &str, paired_at: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE transfer_peers
         SET paired_at = ?2,
             failed_attempts = 0,
             blocked_until = NULL,
             trust_level = ?3
         WHERE device_id = ?1",
        params![
            device_id,
            paired_at,
            TransferPeerTrustLevel::Trusted.as_str()
        ],
    )
    .await?;
    Ok(())
}

pub async fn mark_peer_pair_failure(
    conn: &DbConn,
    device_id: &str,
    blocked_until: Option<i64>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO transfer_peers (device_id, display_name, last_seen_at, paired_at, trust_level, failed_attempts, blocked_until)
         VALUES (?1, ?1, 0, NULL, ?3, 1, ?2)
         ON CONFLICT(device_id) DO UPDATE SET
           failed_attempts = transfer_peers.failed_attempts + 1,
           blocked_until = ?2",
        params![device_id, blocked_until, TransferPeerTrustLevel::Other.as_str()],
    )
    .await?;
    Ok(())
}

pub async fn list_stored_peers(conn: &DbConn) -> AppResult<Vec<TransferPeerDto>> {
    let mut rows = conn
        .query(
            "SELECT device_id, display_name, last_seen_at, paired_at, trust_level, failed_attempts, blocked_until
             FROM transfer_peers
             ORDER BY last_seen_at DESC",
            (),
        )
        .await?;

    let mut peers = Vec::new();
    while let Some(row) = rows.next().await? {
        peers.push(TransferPeerDto {
            device_id: row.get::<String>(0)?,
            display_name: row.get::<String>(1)?,
            address: String::new(),
            listen_port: 0,
            last_seen_at: row.get::<i64>(2)?,
            paired_at: row.get::<Option<i64>>(3)?,
            trust_level: parse_transfer_trust_level(
                row.get::<String>(4)?,
                "transfer_peers.trust_level",
            )?,
            failed_attempts: row.get::<u32>(5)?,
            blocked_until: row.get::<Option<i64>>(6)?,
            pairing_required: true,
            online: false,
        });
    }
    Ok(peers)
}

pub async fn insert_session(conn: &DbConn, session: &TransferSessionDto) -> AppResult<()> {
    conn.execute(
        "INSERT INTO transfer_sessions
         (id, direction, peer_device_id, peer_name, status, total_bytes, transferred_bytes, avg_speed_bps, save_dir,
          created_at, started_at, finished_at, error_code, error_message, cleanup_after_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT(id) DO UPDATE SET
          direction = excluded.direction,
          peer_device_id = excluded.peer_device_id,
          peer_name = excluded.peer_name,
          status = excluded.status,
          total_bytes = excluded.total_bytes,
          transferred_bytes = excluded.transferred_bytes,
          avg_speed_bps = excluded.avg_speed_bps,
          save_dir = excluded.save_dir,
          started_at = COALESCE(excluded.started_at, transfer_sessions.started_at),
          finished_at = excluded.finished_at,
          error_code = excluded.error_code,
          error_message = excluded.error_message,
          cleanup_after_at = excluded.cleanup_after_at",
        params![
            session.id.as_str(),
            session.direction.as_str(),
            session.peer_device_id.as_str(),
            session.peer_name.as_str(),
            session.status.as_str(),
            to_db_i64(session.total_bytes),
            to_db_i64(session.transferred_bytes),
            to_db_i64(session.avg_speed_bps),
            session.save_dir.as_str(),
            session.created_at,
            session.started_at,
            session.finished_at,
            session.error_code.as_deref(),
            session.error_message.as_deref(),
            session.cleanup_after_at,
        ],
    )
    .await?;
    Ok(())
}

pub async fn insert_or_update_file(
    conn: &DbConn,
    file: &TransferFileDto,
    completed_bitmap: &[u8],
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO transfer_files
         (id, session_id, relative_path, source_path, target_path, size_bytes, transferred_bytes, chunk_size, chunk_count,
          completed_bitmap, blake3, mime_type, preview_kind, preview_data, status, is_folder_archive, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, strftime('%s','now') * 1000)
         ON CONFLICT(id) DO UPDATE SET
          transferred_bytes = excluded.transferred_bytes,
          target_path = COALESCE(excluded.target_path, transfer_files.target_path),
          completed_bitmap = COALESCE(excluded.completed_bitmap, transfer_files.completed_bitmap),
          status = excluded.status,
          blake3 = COALESCE(excluded.blake3, transfer_files.blake3),
          mime_type = COALESCE(excluded.mime_type, transfer_files.mime_type),
          preview_kind = COALESCE(excluded.preview_kind, transfer_files.preview_kind),
          preview_data = COALESCE(excluded.preview_data, transfer_files.preview_data),
          updated_at = strftime('%s','now') * 1000",
        params![
            file.id.as_str(),
            file.session_id.as_str(),
            file.relative_path.as_str(),
            file.source_path.as_deref(),
            file.target_path.as_deref(),
            to_db_i64(file.size_bytes),
            to_db_i64(file.transferred_bytes),
            file.chunk_size,
            file.chunk_count,
            completed_bitmap,
            file.blake3.as_deref(),
            file.mime_type.as_deref(),
            file.preview_kind.as_deref(),
            file.preview_data.as_deref(),
            file.status.as_str(),
            if file.is_folder_archive { 1 } else { 0 },
        ],
    )
    .await?;
    Ok(())
}

pub async fn upsert_files_batch(conn: &DbConn, items: &[TransferFilePersistItem]) -> AppResult<()> {
    if items.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction().await?;
    for item in items {
        let file = &item.file;
        tx.execute(
            "INSERT INTO transfer_files
             (id, session_id, relative_path, source_path, target_path, size_bytes, transferred_bytes, chunk_size, chunk_count,
              completed_bitmap, blake3, mime_type, preview_kind, preview_data, status, is_folder_archive, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, strftime('%s','now') * 1000)
             ON CONFLICT(id) DO UPDATE SET
              transferred_bytes = excluded.transferred_bytes,
              target_path = COALESCE(excluded.target_path, transfer_files.target_path),
              completed_bitmap = COALESCE(excluded.completed_bitmap, transfer_files.completed_bitmap),
              status = excluded.status,
              blake3 = COALESCE(excluded.blake3, transfer_files.blake3),
              mime_type = COALESCE(excluded.mime_type, transfer_files.mime_type),
              preview_kind = COALESCE(excluded.preview_kind, transfer_files.preview_kind),
              preview_data = COALESCE(excluded.preview_data, transfer_files.preview_data),
              updated_at = strftime('%s','now') * 1000",
            params![
                file.id.as_str(),
                file.session_id.as_str(),
                file.relative_path.as_str(),
                file.source_path.as_deref(),
                file.target_path.as_deref(),
                to_db_i64(file.size_bytes),
                to_db_i64(file.transferred_bytes),
                file.chunk_size,
                file.chunk_count,
                item.completed_bitmap.as_slice(),
                file.blake3.as_deref(),
                file.mime_type.as_deref(),
                file.preview_kind.as_deref(),
                file.preview_data.as_deref(),
                file.status.as_str(),
                if file.is_folder_archive { 1 } else { 0 },
            ],
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn upsert_session_progress(conn: &DbConn, session: &TransferSessionDto) -> AppResult<()> {
    let tx = conn.transaction().await?;
    tx.execute(
        "INSERT INTO transfer_sessions
         (id, direction, peer_device_id, peer_name, status, total_bytes, transferred_bytes, avg_speed_bps, save_dir,
          created_at, started_at, finished_at, error_code, error_message, cleanup_after_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT(id) DO UPDATE SET
          direction = excluded.direction,
          peer_device_id = excluded.peer_device_id,
          peer_name = excluded.peer_name,
          status = excluded.status,
          total_bytes = excluded.total_bytes,
          transferred_bytes = excluded.transferred_bytes,
          avg_speed_bps = excluded.avg_speed_bps,
          save_dir = excluded.save_dir,
          started_at = COALESCE(excluded.started_at, transfer_sessions.started_at),
          finished_at = excluded.finished_at,
          error_code = excluded.error_code,
          error_message = excluded.error_message,
          cleanup_after_at = excluded.cleanup_after_at",
        params![
            session.id.as_str(),
            session.direction.as_str(),
            session.peer_device_id.as_str(),
            session.peer_name.as_str(),
            session.status.as_str(),
            to_db_i64(session.total_bytes),
            to_db_i64(session.transferred_bytes),
            to_db_i64(session.avg_speed_bps),
            session.save_dir.as_str(),
            session.created_at,
            session.started_at,
            session.finished_at,
            session.error_code.as_deref(),
            session.error_message.as_deref(),
            session.cleanup_after_at,
        ],
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get_file_bitmap(conn: &DbConn, session_id: &str, file_id: &str) -> AppResult<Option<Vec<u8>>> {
    let mut rows = conn
        .query(
            "SELECT completed_bitmap FROM transfer_files WHERE session_id = ?1 AND id = ?2 LIMIT 1",
            params![session_id, file_id],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        return Ok(Some(row.get::<Vec<u8>>(0)?));
    }
    Ok(None)
}

pub async fn get_session(conn: &DbConn, session_id: &str) -> AppResult<Option<TransferSessionDto>> {
    let mut rows = conn
        .query(
            "SELECT id, direction, peer_device_id, peer_name, status, total_bytes, transferred_bytes, avg_speed_bps, save_dir,
                    created_at, started_at, finished_at, error_code, error_message, cleanup_after_at
             FROM transfer_sessions
             WHERE id = ?1
             LIMIT 1",
            params![session_id],
        )
        .await?;

    let Some(row) = rows.next().await? else {
        return Ok(None);
    };

    let mut session = TransferSessionDto {
        id: row.get::<String>(0)?,
        direction: parse_transfer_direction(row.get::<String>(1)?, "transfer_sessions.direction")?,
        peer_device_id: row.get::<String>(2)?,
        peer_name: row.get::<String>(3)?,
        status: parse_transfer_status(row.get::<String>(4)?, "transfer_sessions.status")?,
        total_bytes: from_db_i64(row.get::<i64>(5)?),
        transferred_bytes: from_db_i64(row.get::<i64>(6)?),
        avg_speed_bps: from_db_i64(row.get::<i64>(7)?),
        save_dir: row.get::<String>(8)?,
        created_at: row.get::<i64>(9)?,
        started_at: row.get::<Option<i64>>(10)?,
        finished_at: row.get::<Option<i64>>(11)?,
        error_code: row.get::<Option<String>>(12)?,
        error_message: row.get::<Option<String>>(13)?,
        cleanup_after_at: row.get::<Option<i64>>(14)?,
        files: Vec::new(),
    };

    session.files = list_session_files(conn, session.id.as_str()).await?;
    Ok(Some(session))
}

pub async fn list_session_files(conn: &DbConn, session_id: &str) -> AppResult<Vec<TransferFileDto>> {
    let mut rows = conn
        .query(
            "SELECT id, session_id, relative_path, source_path, target_path, size_bytes, transferred_bytes, chunk_size, chunk_count,
                    status, blake3, mime_type, preview_kind, preview_data, is_folder_archive
             FROM transfer_files
             WHERE session_id = ?1
             ORDER BY relative_path ASC",
            params![session_id],
        )
        .await?;

    let mut files = Vec::new();
    while let Some(row) = rows.next().await? {
        files.push(TransferFileDto {
            id: row.get::<String>(0)?,
            session_id: row.get::<String>(1)?,
            relative_path: row.get::<String>(2)?,
            source_path: row.get::<Option<String>>(3)?,
            target_path: row.get::<Option<String>>(4)?,
            size_bytes: from_db_i64(row.get::<i64>(5)?),
            transferred_bytes: from_db_i64(row.get::<i64>(6)?),
            chunk_size: row.get::<u32>(7)?,
            chunk_count: row.get::<u32>(8)?,
            status: parse_transfer_status(row.get::<String>(9)?, "transfer_files.status")?,
            blake3: row.get::<Option<String>>(10)?,
            mime_type: row.get::<Option<String>>(11)?,
            preview_kind: row.get::<Option<String>>(12)?,
            preview_data: row.get::<Option<String>>(13)?,
            is_folder_archive: row.get::<i64>(14)? == 1,
        });
    }
    Ok(files)
}

async fn list_files_for_sessions(
    conn: &DbConn,
    session_ids: &[String],
) -> AppResult<std::collections::HashMap<String, Vec<TransferFileDto>>> {
    if session_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let placeholders = (1..=session_ids.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT id, session_id, relative_path, source_path, target_path, size_bytes, transferred_bytes, chunk_size, chunk_count,
                status, blake3, mime_type, preview_kind, preview_data, is_folder_archive
         FROM transfer_files
         WHERE session_id IN ({placeholders})
         ORDER BY session_id ASC, relative_path ASC"
    );
    let session_ids_owned = session_ids.to_vec();

    let mut rows = conn
        .query(sql.as_str(), params_from_iter(session_ids_owned))
        .await?;

    let mut grouped = std::collections::HashMap::<String, Vec<TransferFileDto>>::new();
    while let Some(row) = rows.next().await? {
        let session_id = row.get::<String>(1)?;
        let file = TransferFileDto {
            id: row.get::<String>(0)?,
            session_id: session_id.clone(),
            relative_path: row.get::<String>(2)?,
            source_path: row.get::<Option<String>>(3)?,
            target_path: row.get::<Option<String>>(4)?,
            size_bytes: from_db_i64(row.get::<i64>(5)?),
            transferred_bytes: from_db_i64(row.get::<i64>(6)?),
            chunk_size: row.get::<u32>(7)?,
            chunk_count: row.get::<u32>(8)?,
            status: parse_transfer_status(row.get::<String>(9)?, "transfer_files.status")?,
            blake3: row.get::<Option<String>>(10)?,
            mime_type: row.get::<Option<String>>(11)?,
            preview_kind: row.get::<Option<String>>(12)?,
            preview_data: row.get::<Option<String>>(13)?,
            is_folder_archive: row.get::<i64>(14)? == 1,
        };
        grouped.entry(session_id).or_default().push(file);
    }

    Ok(grouped)
}

pub async fn list_history(
    conn: &DbConn,
    filter: &TransferHistoryFilterDto,
) -> AppResult<TransferHistoryPageDto> {
    let limit = filter.limit.unwrap_or(30).clamp(1, HISTORY_LIMIT_MAX) as i64;
    let cursor = filter.cursor.clone().unwrap_or_default();
    let (cursor_created_at, cursor_id) = if cursor.trim().is_empty() {
        (None, None)
    } else if let Some((created_at, id)) = cursor.split_once(':') {
        (created_at.parse::<i64>().ok(), Some(id.to_string()))
    } else {
        (cursor.parse::<i64>().ok(), None)
    };
    let status = filter
        .status
        .map(|value| value.as_str().to_string())
        .unwrap_or_default();
    let peer = filter.peer_device_id.clone().unwrap_or_default();
    let cursor_enabled = if cursor_created_at.is_some() { 1 } else { 0 };
    let cursor_created_at = cursor_created_at.unwrap_or_default();
    let cursor_id = cursor_id.unwrap_or_default();
    let mut rows = conn
        .query(
            "SELECT id, direction, peer_device_id, peer_name, status, total_bytes, transferred_bytes, avg_speed_bps, save_dir,
                    created_at, started_at, finished_at, error_code, error_message, cleanup_after_at
             FROM transfer_sessions
             WHERE (?1 = 0 OR created_at < ?2 OR (created_at = ?2 AND id < ?3))
               AND (?4 = '' OR status = ?4)
               AND (?5 = '' OR peer_device_id = ?5)
             ORDER BY created_at DESC, id DESC
             LIMIT ?6",
            params![
                cursor_enabled,
                cursor_created_at,
                cursor_id,
                status,
                peer,
                limit
            ],
        )
        .await?;

    let mut items = Vec::new();
    while let Some(row) = rows.next().await? {
        items.push(TransferSessionDto {
            id: row.get::<String>(0)?,
            direction: parse_transfer_direction(row.get::<String>(1)?, "transfer_sessions.direction")?,
            peer_device_id: row.get::<String>(2)?,
            peer_name: row.get::<String>(3)?,
            status: parse_transfer_status(row.get::<String>(4)?, "transfer_sessions.status")?,
            total_bytes: from_db_i64(row.get::<i64>(5)?),
            transferred_bytes: from_db_i64(row.get::<i64>(6)?),
            avg_speed_bps: from_db_i64(row.get::<i64>(7)?),
            save_dir: row.get::<String>(8)?,
            created_at: row.get::<i64>(9)?,
            started_at: row.get::<Option<i64>>(10)?,
            finished_at: row.get::<Option<i64>>(11)?,
            error_code: row.get::<Option<String>>(12)?,
            error_message: row.get::<Option<String>>(13)?,
            cleanup_after_at: row.get::<Option<i64>>(14)?,
            files: Vec::new(),
        });
    }

    let session_ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    let mut files_by_session = list_files_for_sessions(conn, session_ids.as_slice()).await?;
    for item in &mut items {
        item.files = files_by_session.remove(item.id.as_str()).unwrap_or_default();
    }

    let next_cursor = items
        .last()
        .map(|value| format!("{}:{}", value.created_at, value.id));
    Ok(TransferHistoryPageDto { items, next_cursor })
}

pub async fn clear_history(conn: &DbConn, all: bool, older_than_days: u32) -> AppResult<()> {
    if all {
        conn.execute("DELETE FROM transfer_sessions", ()).await?;
        return Ok(());
    }

    let threshold = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default())
        - i64::from(older_than_days.clamp(1, 365)) * 86_400_000;

    conn.execute(
        "DELETE FROM transfer_sessions WHERE created_at < ?1",
        params![threshold],
    )
    .await?;
    Ok(())
}

pub async fn cleanup_expired(conn: &DbConn, now_millis: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM transfer_sessions WHERE cleanup_after_at IS NOT NULL AND cleanup_after_at <= ?1",
        params![now_millis],
    )
    .await?;
    Ok(())
}

pub async fn list_failed_sessions(
    conn: &DbConn,
    session_id: &str,
) -> AppResult<Option<TransferSessionDto>> {
    let session = get_session(conn, session_id).await?;
    let Some(value) = session else {
        return Ok(None);
    };
    if !value.status.is_retryable() {
        return Ok(None);
    }
    Ok(Some(value))
}

pub fn merge_online_peers(
    stored: Vec<TransferPeerDto>,
    online: &[TransferPeerDto],
) -> Vec<TransferPeerDto> {
    let mut output = std::collections::HashMap::new();

    for item in stored {
        output.insert(item.device_id.clone(), item);
    }

    for peer in online {
        let mut next = output
            .remove(peer.device_id.as_str())
            .unwrap_or_else(|| TransferPeerDto {
                device_id: peer.device_id.clone(),
                display_name: peer.display_name.clone(),
                address: peer.address.clone(),
                listen_port: peer.listen_port,
                last_seen_at: peer.last_seen_at,
                paired_at: None,
                trust_level: TransferPeerTrustLevel::Other,
                failed_attempts: 0,
                blocked_until: None,
                pairing_required: peer.pairing_required,
                online: true,
            });

        next.display_name = peer.display_name.clone();
        next.address = peer.address.clone();
        next.listen_port = peer.listen_port;
        next.last_seen_at = peer.last_seen_at;
        next.pairing_required = peer.pairing_required;
        next.online = true;
        output.insert(next.device_id.clone(), next);
    }

    let mut peers: Vec<TransferPeerDto> = output.into_values().collect();
    peers.sort_by(|left, right| {
        right
            .online
            .cmp(&left.online)
            .then(right.last_seen_at.cmp(&left.last_seen_at))
    });
    peers
}

pub async fn ensure_session_exists(conn: &DbConn, session_id: &str) -> AppResult<TransferSessionDto> {
    get_session(conn, session_id).await?.ok_or_else(|| {
        AppError::new("transfer_session_not_found", "未找到对应传输会话")
            .with_context("sessionId", session_id.to_string())
    })
}

#[cfg(test)]
#[path = "../../tests/transfer/store_tests.rs"]
mod tests;

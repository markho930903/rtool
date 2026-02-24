use rusqlite::{OptionalExtension, params, params_from_iter};

use app_core::models::{
    TransferDirection, TransferFileDto, TransferHistoryFilterDto, TransferHistoryPageDto,
    TransferPeerDto, TransferPeerTrustLevel, TransferSessionDto, TransferSettingsDto,
    TransferStatus,
};
use app_core::{AppError, AppResult};
use crate::db::{DbPool, get_app_setting, set_app_setting};

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

pub fn load_settings(
    pool: &DbPool,
    default_download_dir: String,
) -> AppResult<TransferSettingsDto> {
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

    if let Some(value) = get_app_setting(pool, TRANSFER_DEFAULT_DOWNLOAD_DIR_KEY)? {
        settings.default_download_dir = value;
    }
    settings.max_parallel_files =
        parse_u32(get_app_setting(pool, TRANSFER_MAX_PARALLEL_FILES_KEY)?, 2).clamp(1, 8);
    settings.max_inflight_chunks =
        parse_u32(get_app_setting(pool, TRANSFER_MAX_INFLIGHT_CHUNKS_KEY)?, 16).clamp(1, 64);
    settings.chunk_size_kb =
        parse_u32(get_app_setting(pool, TRANSFER_CHUNK_SIZE_KB_KEY)?, 1024).clamp(64, 4096);
    settings.auto_cleanup_days =
        parse_u32(get_app_setting(pool, TRANSFER_AUTO_CLEANUP_DAYS_KEY)?, 30).clamp(1, 365);
    settings.resume_enabled = parse_bool(get_app_setting(pool, TRANSFER_RESUME_ENABLED_KEY)?, true);
    settings.discovery_enabled =
        parse_bool(get_app_setting(pool, TRANSFER_DISCOVERY_ENABLED_KEY)?, true);
    settings.pairing_required =
        parse_bool(get_app_setting(pool, TRANSFER_PAIRING_REQUIRED_KEY)?, true);
    settings.db_flush_interval_ms = parse_u32(
        get_app_setting(pool, TRANSFER_DB_FLUSH_INTERVAL_MS_KEY)?,
        400,
    )
    .clamp(100, 5000);
    settings.event_emit_interval_ms = parse_u32(
        get_app_setting(pool, TRANSFER_EVENT_EMIT_INTERVAL_MS_KEY)?,
        250,
    )
    .clamp(100, 2000);
    settings.ack_batch_size =
        parse_u32(get_app_setting(pool, TRANSFER_ACK_BATCH_SIZE_KEY)?, 64).clamp(1, 512);
    settings.ack_flush_interval_ms = parse_u32(
        get_app_setting(pool, TRANSFER_ACK_FLUSH_INTERVAL_MS_KEY)?,
        20,
    )
    .clamp(5, 2000);

    save_settings(pool, &settings)?;
    Ok(settings)
}

pub fn save_settings(pool: &DbPool, settings: &TransferSettingsDto) -> AppResult<()> {
    set_app_setting(
        pool,
        TRANSFER_DEFAULT_DOWNLOAD_DIR_KEY,
        settings.default_download_dir.as_str(),
    )?;
    set_app_setting(
        pool,
        TRANSFER_MAX_PARALLEL_FILES_KEY,
        settings.max_parallel_files.to_string().as_str(),
    )?;
    set_app_setting(
        pool,
        TRANSFER_MAX_INFLIGHT_CHUNKS_KEY,
        settings.max_inflight_chunks.to_string().as_str(),
    )?;
    set_app_setting(
        pool,
        TRANSFER_CHUNK_SIZE_KB_KEY,
        settings.chunk_size_kb.to_string().as_str(),
    )?;
    set_app_setting(
        pool,
        TRANSFER_AUTO_CLEANUP_DAYS_KEY,
        settings.auto_cleanup_days.to_string().as_str(),
    )?;
    set_app_setting(
        pool,
        TRANSFER_RESUME_ENABLED_KEY,
        to_bool_string(settings.resume_enabled),
    )?;
    set_app_setting(
        pool,
        TRANSFER_DISCOVERY_ENABLED_KEY,
        to_bool_string(settings.discovery_enabled),
    )?;
    set_app_setting(
        pool,
        TRANSFER_PAIRING_REQUIRED_KEY,
        to_bool_string(settings.pairing_required),
    )?;
    set_app_setting(
        pool,
        TRANSFER_DB_FLUSH_INTERVAL_MS_KEY,
        settings.db_flush_interval_ms.to_string().as_str(),
    )?;
    set_app_setting(
        pool,
        TRANSFER_EVENT_EMIT_INTERVAL_MS_KEY,
        settings.event_emit_interval_ms.to_string().as_str(),
    )?;
    set_app_setting(
        pool,
        TRANSFER_ACK_BATCH_SIZE_KEY,
        settings.ack_batch_size.to_string().as_str(),
    )?;
    set_app_setting(
        pool,
        TRANSFER_ACK_FLUSH_INTERVAL_MS_KEY,
        settings.ack_flush_interval_ms.to_string().as_str(),
    )?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TransferFilePersistItem {
    pub file: TransferFileDto,
    pub completed_bitmap: Vec<u8>,
}

pub fn upsert_peer(pool: &DbPool, peer: &TransferPeerDto) -> AppResult<()> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO transfer_peers (device_id, display_name, last_seen_at, paired_at, trust_level, failed_attempts, blocked_until)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(device_id) DO UPDATE SET
           display_name = excluded.display_name,
           last_seen_at = excluded.last_seen_at",
        params![
            peer.device_id,
            peer.display_name,
            peer.last_seen_at,
            peer.paired_at,
            peer.trust_level.as_str(),
            peer.failed_attempts,
            peer.blocked_until,
        ],
    )?;
    Ok(())
}

pub fn get_peer_by_device_id(pool: &DbPool, device_id: &str) -> AppResult<Option<TransferPeerDto>> {
    let conn = pool.get()?;
    let peer_row = conn
        .query_row(
            "SELECT device_id, display_name, last_seen_at, paired_at, trust_level, failed_attempts, blocked_until
             FROM transfer_peers
             WHERE device_id = ?1
             LIMIT 1",
            params![device_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, u32>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                ))
            },
        )
        .optional()?;
    let Some((device_id, display_name, last_seen_at, paired_at, trust_level_raw, failed_attempts, blocked_until)) =
        peer_row
    else {
        return Ok(None);
    };
    Ok(Some(TransferPeerDto {
        device_id,
        display_name,
        address: String::new(),
        listen_port: 0,
        last_seen_at,
        paired_at,
        trust_level: parse_transfer_trust_level(trust_level_raw, "transfer_peers.trust_level")?,
        failed_attempts,
        blocked_until,
        pairing_required: true,
        online: false,
    }))
}

pub fn mark_peer_pair_success(pool: &DbPool, device_id: &str, paired_at: i64) -> AppResult<()> {
    let conn = pool.get()?;
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
    )?;
    Ok(())
}

pub fn mark_peer_pair_failure(
    pool: &DbPool,
    device_id: &str,
    blocked_until: Option<i64>,
) -> AppResult<()> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO transfer_peers (device_id, display_name, last_seen_at, paired_at, trust_level, failed_attempts, blocked_until)
         VALUES (?1, ?1, 0, NULL, ?3, 1, ?2)
         ON CONFLICT(device_id) DO UPDATE SET
           failed_attempts = transfer_peers.failed_attempts + 1,
           blocked_until = ?2",
        params![device_id, blocked_until, TransferPeerTrustLevel::Other.as_str()],
    )?;
    Ok(())
}

pub fn list_stored_peers(pool: &DbPool) -> AppResult<Vec<TransferPeerDto>> {
    let conn = pool.get()?;
    let mut statement = conn.prepare(
        "SELECT device_id, display_name, last_seen_at, paired_at, trust_level, failed_attempts, blocked_until
         FROM transfer_peers
         ORDER BY last_seen_at DESC",
    )?;

    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<i64>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, u32>(5)?,
            row.get::<_, Option<i64>>(6)?,
        ))
    })?;

    let mut peers = Vec::new();
    for row in rows {
        let (
            device_id,
            display_name,
            last_seen_at,
            paired_at,
            trust_level_raw,
            failed_attempts,
            blocked_until,
        ) = row?;
        peers.push(TransferPeerDto {
            device_id,
            display_name,
            address: String::new(),
            listen_port: 0,
            last_seen_at,
            paired_at,
            trust_level: parse_transfer_trust_level(
                trust_level_raw,
                "transfer_peers.trust_level",
            )?,
            failed_attempts,
            blocked_until,
            pairing_required: true,
            online: false,
        });
    }
    Ok(peers)
}

pub fn insert_session(pool: &DbPool, session: &TransferSessionDto) -> AppResult<()> {
    let conn = pool.get()?;
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
            session.id,
            session.direction.as_str(),
            session.peer_device_id,
            session.peer_name,
            session.status.as_str(),
            to_db_i64(session.total_bytes),
            to_db_i64(session.transferred_bytes),
            to_db_i64(session.avg_speed_bps),
            session.save_dir,
            session.created_at,
            session.started_at,
            session.finished_at,
            session.error_code,
            session.error_message,
            session.cleanup_after_at,
        ],
    )?;
    Ok(())
}

pub fn insert_or_update_file(
    pool: &DbPool,
    file: &TransferFileDto,
    completed_bitmap: &[u8],
) -> AppResult<()> {
    let conn = pool.get()?;
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
            file.id,
            file.session_id,
            file.relative_path,
            file.source_path,
            file.target_path,
            to_db_i64(file.size_bytes),
            to_db_i64(file.transferred_bytes),
            file.chunk_size,
            file.chunk_count,
            completed_bitmap,
            file.blake3,
            file.mime_type,
            file.preview_kind,
            file.preview_data,
            file.status.as_str(),
            if file.is_folder_archive { 1 } else { 0 },
        ],
    )?;
    Ok(())
}

pub fn upsert_files_batch(pool: &DbPool, items: &[TransferFilePersistItem]) -> AppResult<()> {
    if items.is_empty() {
        return Ok(());
    }

    let mut conn = pool.get()?;
    let transaction = conn.transaction()?;
    {
        let mut statement = transaction.prepare(
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
        )?;

        for item in items {
            let file = &item.file;
            statement.execute(params![
                file.id,
                file.session_id,
                file.relative_path,
                file.source_path,
                file.target_path,
                to_db_i64(file.size_bytes),
                to_db_i64(file.transferred_bytes),
                file.chunk_size,
                file.chunk_count,
                item.completed_bitmap.as_slice(),
                file.blake3,
                file.mime_type,
                file.preview_kind,
                file.preview_data,
                file.status.as_str(),
                if file.is_folder_archive { 1 } else { 0 },
            ])?;
        }
    }

    transaction.commit()?;
    Ok(())
}

pub fn upsert_session_progress(pool: &DbPool, session: &TransferSessionDto) -> AppResult<()> {
    let mut conn = pool.get()?;
    let transaction = conn.transaction()?;
    transaction.execute(
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
            session.id,
            session.direction.as_str(),
            session.peer_device_id,
            session.peer_name,
            session.status.as_str(),
            to_db_i64(session.total_bytes),
            to_db_i64(session.transferred_bytes),
            to_db_i64(session.avg_speed_bps),
            session.save_dir,
            session.created_at,
            session.started_at,
            session.finished_at,
            session.error_code,
            session.error_message,
            session.cleanup_after_at,
        ],
    )?;
    transaction.commit()?;
    Ok(())
}

pub fn get_file_bitmap(
    pool: &DbPool,
    session_id: &str,
    file_id: &str,
) -> AppResult<Option<Vec<u8>>> {
    let conn = pool.get()?;
    let value = conn
        .query_row(
            "SELECT completed_bitmap FROM transfer_files WHERE session_id = ?1 AND id = ?2 LIMIT 1",
            params![session_id, file_id],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()?;
    Ok(value)
}

pub fn get_session(pool: &DbPool, session_id: &str) -> AppResult<Option<TransferSessionDto>> {
    let conn = pool.get()?;
    let session_row = conn
        .query_row(
            "SELECT id, direction, peer_device_id, peer_name, status, total_bytes, transferred_bytes, avg_speed_bps, save_dir,
                    created_at, started_at, finished_at, error_code, error_message, cleanup_after_at
             FROM transfer_sessions
             WHERE id = ?1
             LIMIT 1",
            params![session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<i64>>(14)?,
                ))
            },
        )
        .optional()?;

    let Some((
        id,
        direction_raw,
        peer_device_id,
        peer_name,
        status_raw,
        total_bytes,
        transferred_bytes,
        avg_speed_bps,
        save_dir,
        created_at,
        started_at,
        finished_at,
        error_code,
        error_message,
        cleanup_after_at,
    )) = session_row
    else {
        return Ok(None);
    };
    let mut session = TransferSessionDto {
        id,
        direction: parse_transfer_direction(direction_raw, "transfer_sessions.direction")?,
        peer_device_id,
        peer_name,
        status: parse_transfer_status(status_raw, "transfer_sessions.status")?,
        total_bytes: from_db_i64(total_bytes),
        transferred_bytes: from_db_i64(transferred_bytes),
        avg_speed_bps: from_db_i64(avg_speed_bps),
        save_dir,
        created_at,
        started_at,
        finished_at,
        error_code,
        error_message,
        cleanup_after_at,
        files: Vec::new(),
    };

    session.files = list_session_files(pool, session.id.as_str())?;
    Ok(Some(session))
}

pub fn list_session_files(pool: &DbPool, session_id: &str) -> AppResult<Vec<TransferFileDto>> {
    let conn = pool.get()?;
    let mut statement = conn.prepare(
        "SELECT id, session_id, relative_path, source_path, target_path, size_bytes, transferred_bytes, chunk_size, chunk_count,
                status, blake3, mime_type, preview_kind, preview_data, is_folder_archive
         FROM transfer_files
         WHERE session_id = ?1
         ORDER BY relative_path ASC",
    )?;

    let rows = statement.query_map(params![session_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, u32>(7)?,
            row.get::<_, u32>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<String>>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, Option<String>>(13)?,
            row.get::<_, i64>(14)? == 1,
        ))
    })?;

    let mut files = Vec::new();
    for row in rows {
        let (
            id,
            session_id,
            relative_path,
            source_path,
            target_path,
            size_bytes,
            transferred_bytes,
            chunk_size,
            chunk_count,
            status_raw,
            blake3,
            mime_type,
            preview_kind,
            preview_data,
            is_folder_archive,
        ) = row?;
        files.push(TransferFileDto {
            id,
            session_id,
            relative_path,
            source_path,
            target_path,
            size_bytes: from_db_i64(size_bytes),
            transferred_bytes: from_db_i64(transferred_bytes),
            chunk_size,
            chunk_count,
            status: parse_transfer_status(status_raw, "transfer_files.status")?,
            blake3,
            mime_type,
            preview_kind,
            preview_data,
            is_folder_archive,
        });
    }
    Ok(files)
}

fn list_files_for_sessions(
    pool: &DbPool,
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

    let conn = pool.get()?;
    let mut statement = conn.prepare(sql.as_str())?;
    let rows = statement.query_map(params_from_iter(session_ids.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, u32>(7)?,
            row.get::<_, u32>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<String>>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, Option<String>>(13)?,
            row.get::<_, i64>(14)? == 1,
        ))
    })?;

    let mut grouped = std::collections::HashMap::<String, Vec<TransferFileDto>>::new();
    for row in rows {
        let (
            id,
            session_id,
            relative_path,
            source_path,
            target_path,
            size_bytes,
            transferred_bytes,
            chunk_size,
            chunk_count,
            status_raw,
            blake3,
            mime_type,
            preview_kind,
            preview_data,
            is_folder_archive,
        ) = row?;
        let file = TransferFileDto {
            id,
            session_id: session_id.clone(),
            relative_path,
            source_path,
            target_path,
            size_bytes: from_db_i64(size_bytes),
            transferred_bytes: from_db_i64(transferred_bytes),
            chunk_size,
            chunk_count,
            status: parse_transfer_status(status_raw, "transfer_files.status")?,
            blake3,
            mime_type,
            preview_kind,
            preview_data,
            is_folder_archive,
        };
        grouped
            .entry(session_id)
            .or_default()
            .push(file);
    }
    Ok(grouped)
}

pub fn list_history(
    pool: &DbPool,
    filter: &TransferHistoryFilterDto,
) -> AppResult<TransferHistoryPageDto> {
    let conn = pool.get()?;
    let limit = filter.limit.unwrap_or(30).clamp(1, HISTORY_LIMIT_MAX) as i64;
    let cursor = filter.cursor.clone().unwrap_or_default();
    let status = filter
        .status
        .map(|value| value.as_str().to_string())
        .unwrap_or_default();
    let peer = filter.peer_device_id.clone().unwrap_or_default();

    let mut statement = conn.prepare(
        "SELECT id, direction, peer_device_id, peer_name, status, total_bytes, transferred_bytes, avg_speed_bps, save_dir,
                created_at, started_at, finished_at, error_code, error_message, cleanup_after_at
         FROM transfer_sessions
         WHERE (?1 = '' OR created_at < CAST(?1 AS INTEGER))
           AND (?2 = '' OR status = ?2)
           AND (?3 = '' OR peer_device_id = ?3)
         ORDER BY created_at DESC
         LIMIT ?4",
    )?;

    let rows = statement.query_map(params![cursor, status, peer, limit], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, i64>(9)?,
            row.get::<_, Option<i64>>(10)?,
            row.get::<_, Option<i64>>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, Option<String>>(13)?,
            row.get::<_, Option<i64>>(14)?,
        ))
    })?;

    let mut items = Vec::new();
    for row in rows {
        let (
            id,
            direction_raw,
            peer_device_id,
            peer_name,
            status_raw,
            total_bytes,
            transferred_bytes,
            avg_speed_bps,
            save_dir,
            created_at,
            started_at,
            finished_at,
            error_code,
            error_message,
            cleanup_after_at,
        ) = row?;
        items.push(TransferSessionDto {
            id,
            direction: parse_transfer_direction(direction_raw, "transfer_sessions.direction")?,
            peer_device_id,
            peer_name,
            status: parse_transfer_status(status_raw, "transfer_sessions.status")?,
            total_bytes: from_db_i64(total_bytes),
            transferred_bytes: from_db_i64(transferred_bytes),
            avg_speed_bps: from_db_i64(avg_speed_bps),
            save_dir,
            created_at,
            started_at,
            finished_at,
            error_code,
            error_message,
            cleanup_after_at,
            files: Vec::new(),
        });
    }

    let session_ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    let mut files_by_session = list_files_for_sessions(pool, session_ids.as_slice())?;
    for item in &mut items {
        item.files = files_by_session
            .remove(item.id.as_str())
            .unwrap_or_default();
    }

    let next_cursor = items.last().map(|value| value.created_at.to_string());
    Ok(TransferHistoryPageDto { items, next_cursor })
}

pub fn clear_history(pool: &DbPool, all: bool, older_than_days: u32) -> AppResult<()> {
    let conn = pool.get()?;
    if all {
        conn.execute("DELETE FROM transfer_files", [])?;
        conn.execute("DELETE FROM transfer_sessions", [])?;
        return Ok(());
    }

    let threshold = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default())
        - i64::from(older_than_days.clamp(1, 365)) * 86_400_000;

    conn.execute(
        "DELETE FROM transfer_files WHERE session_id IN (SELECT id FROM transfer_sessions WHERE created_at < ?1)",
        params![threshold],
    )?;
    conn.execute(
        "DELETE FROM transfer_sessions WHERE created_at < ?1",
        params![threshold],
    )?;
    Ok(())
}

pub fn cleanup_expired(pool: &DbPool, now_millis: i64) -> AppResult<()> {
    let conn = pool.get()?;
    conn.execute(
        "DELETE FROM transfer_files WHERE session_id IN (SELECT id FROM transfer_sessions WHERE cleanup_after_at IS NOT NULL AND cleanup_after_at <= ?1)",
        params![now_millis],
    )?;
    conn.execute(
        "DELETE FROM transfer_sessions WHERE cleanup_after_at IS NOT NULL AND cleanup_after_at <= ?1",
        params![now_millis],
    )?;
    Ok(())
}

pub fn list_failed_sessions(
    pool: &DbPool,
    session_id: &str,
) -> AppResult<Option<TransferSessionDto>> {
    let session = get_session(pool, session_id)?;
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

pub fn ensure_session_exists(pool: &DbPool, session_id: &str) -> AppResult<TransferSessionDto> {
    get_session(pool, session_id)?.ok_or_else(|| {
        AppError::new("transfer_session_not_found", "未找到对应传输会话")
            .with_context("sessionId", session_id.to_string())
    })
}

#[cfg(test)]
#[path = "../../../../tests/infrastructure/transfer/store_tests.rs"]
mod tests;

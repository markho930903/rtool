use crate::core::models::{ClipboardFilterDto, ClipboardItemDto};
use crate::core::{AppError, AppResult};
use crate::infrastructure::clipboard::derive_content_key;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Error as SqliteError, OptionalExtension, Row, params};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

pub const CLIPBOARD_MAX_ITEMS_KEY: &str = "clipboard.maxItems";
pub const CLIPBOARD_SIZE_CLEANUP_ENABLED_KEY: &str = "clipboard.sizeCleanupEnabled";
pub const CLIPBOARD_MAX_TOTAL_SIZE_MB_KEY: &str = "clipboard.maxTotalSizeMb";
const CLIPBOARD_LIST_LIMIT_MAX: u32 = 10_000;

#[derive(Debug, Clone)]
pub struct PrunedClipboardItem {
    pub id: String,
    pub preview_path: Option<String>,
}

pub type DbPool = Pool<SqliteConnectionManager>;

fn is_duplicate_column_error(error: SqliteError) -> AppResult<()> {
    match error {
        SqliteError::SqliteFailure(_, Some(message)) if message.contains("duplicate column") => {
            Ok(())
        }
        other => Err(other.into()),
    }
}

fn map_clipboard_item_row(row: &Row<'_>) -> Result<ClipboardItemDto, SqliteError> {
    Ok(ClipboardItemDto {
        id: row.get(0)?,
        content_key: row.get(1)?,
        item_type: row.get(2)?,
        plain_text: row.get(3)?,
        source_app: row.get(4)?,
        preview_path: row.get(5)?,
        preview_data_url: row.get(6)?,
        created_at: row.get(7)?,
        pinned: row.get::<_, i64>(8)? == 1,
    })
}

fn backfill_clipboard_content_keys(conn: &Connection) -> AppResult<()> {
    let mut statement = conn.prepare(
        "SELECT id, item_type, plain_text, preview_path, preview_data_url
         FROM clipboard_items
         WHERE content_key IS NULL OR TRIM(content_key) = ''",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;

    let mut updates = Vec::new();
    for row in rows {
        let (id, item_type, plain_text, preview_path, preview_data_url) = row?;
        let content_key = derive_content_key(
            &item_type,
            &plain_text,
            preview_path.as_deref(),
            preview_data_url.as_deref(),
            Some(id.as_str()),
        );
        updates.push((id, content_key));
    }

    for (id, content_key) in updates {
        conn.execute(
            "UPDATE clipboard_items SET content_key = ?1 WHERE id = ?2",
            params![content_key, id],
        )?;
    }

    Ok(())
}

fn deduplicate_clipboard_items_by_content_key(conn: &Connection) -> AppResult<()> {
    #[derive(Default)]
    struct DuplicateGroup {
        winner_id: String,
        winner_pinned: bool,
        pinned_any: bool,
        duplicate_ids: Vec<String>,
    }

    let mut groups: HashMap<String, DuplicateGroup> = HashMap::new();

    let mut statement = conn.prepare(
        "SELECT id, content_key, pinned
         FROM clipboard_items
         WHERE content_key IS NOT NULL AND TRIM(content_key) != ''
         ORDER BY content_key ASC, created_at DESC, id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)? == 1,
        ))
    })?;

    for row in rows {
        let (id, content_key, pinned) = row?;
        if let Some(group) = groups.get_mut(content_key.as_str()) {
            group.pinned_any |= pinned;
            group.duplicate_ids.push(id);
            continue;
        }

        groups.insert(
            content_key,
            DuplicateGroup {
                winner_id: id,
                winner_pinned: pinned,
                pinned_any: pinned,
                duplicate_ids: Vec::new(),
            },
        );
    }

    for group in groups.values() {
        if group.pinned_any && !group.winner_pinned {
            conn.execute(
                "UPDATE clipboard_items SET pinned = 1 WHERE id = ?1",
                params![group.winner_id],
            )?;
        }

        for duplicate_id in &group.duplicate_ids {
            conn.execute(
                "DELETE FROM clipboard_items WHERE id = ?1",
                params![duplicate_id],
            )?;
        }
    }

    Ok(())
}

pub fn new_db_pool(db_path: &Path) -> AppResult<DbPool> {
    let manager = SqliteConnectionManager::file(db_path);
    Ok(Pool::builder().max_size(8).build(manager)?)
}

pub fn init_db(db_path: &Path) -> AppResult<()> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA temp_store = MEMORY;
        PRAGMA busy_timeout = 3000;
        "#,
    )?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS clipboard_items (
            id TEXT PRIMARY KEY,
            content_key TEXT,
            item_type TEXT NOT NULL,
            plain_text TEXT NOT NULL,
            source_app TEXT,
            preview_path TEXT,
            preview_data_url TEXT,
            created_at INTEGER NOT NULL,
            pinned INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS command_history (
            id TEXT PRIMARY KEY,
            action_id TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS log_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            level TEXT NOT NULL,
            scope TEXT NOT NULL,
            event TEXT NOT NULL,
            request_id TEXT NOT NULL,
            window_label TEXT,
            message TEXT NOT NULL,
            metadata TEXT,
            raw_ref TEXT,
            aggregated_count INTEGER
        );

        CREATE TABLE IF NOT EXISTS transfer_sessions (
            id TEXT PRIMARY KEY,
            direction TEXT NOT NULL,
            peer_device_id TEXT NOT NULL,
            peer_name TEXT NOT NULL,
            status TEXT NOT NULL,
            total_bytes INTEGER NOT NULL,
            transferred_bytes INTEGER NOT NULL,
            avg_speed_bps INTEGER NOT NULL,
            save_dir TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            started_at INTEGER,
            finished_at INTEGER,
            error_code TEXT,
            error_message TEXT,
            cleanup_after_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS transfer_files (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            source_path TEXT,
            target_path TEXT,
            size_bytes INTEGER NOT NULL,
            transferred_bytes INTEGER NOT NULL,
            chunk_size INTEGER NOT NULL,
            chunk_count INTEGER NOT NULL,
            completed_bitmap BLOB,
            blake3 TEXT,
            mime_type TEXT,
            preview_kind TEXT,
            preview_data TEXT,
            status TEXT NOT NULL,
            is_folder_archive INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY(session_id) REFERENCES transfer_sessions(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS transfer_peers (
            device_id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            last_seen_at INTEGER NOT NULL,
            paired_at INTEGER,
            trust_level TEXT NOT NULL DEFAULT 'unknown',
            failed_attempts INTEGER NOT NULL DEFAULT 0,
            blocked_until INTEGER
        );

        CREATE INDEX IF NOT EXISTS idx_clipboard_created_at ON clipboard_items(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_clipboard_type_created_at ON clipboard_items(item_type, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_log_ts ON log_entries(timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_log_level_ts ON log_entries(level, timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_log_scope_ts ON log_entries(scope, timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_log_request_id ON log_entries(request_id);
        CREATE INDEX IF NOT EXISTS idx_transfer_sessions_created_at ON transfer_sessions(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_transfer_sessions_status ON transfer_sessions(status, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_transfer_sessions_cleanup ON transfer_sessions(cleanup_after_at);
        CREATE INDEX IF NOT EXISTS idx_transfer_files_session_id ON transfer_files(session_id);
        CREATE INDEX IF NOT EXISTS idx_transfer_peers_last_seen ON transfer_peers(last_seen_at DESC);
        "#,
    )?;

    if let Err(error) = conn.execute(
        "ALTER TABLE clipboard_items ADD COLUMN preview_path TEXT",
        [],
    ) {
        is_duplicate_column_error(error)?;
    }

    if let Err(error) = conn.execute(
        "ALTER TABLE clipboard_items ADD COLUMN preview_data_url TEXT",
        [],
    ) {
        is_duplicate_column_error(error)?;
    }

    if let Err(error) = conn.execute(
        "ALTER TABLE clipboard_items ADD COLUMN content_key TEXT",
        [],
    ) {
        is_duplicate_column_error(error)?;
    }

    backfill_clipboard_content_keys(&conn)?;
    deduplicate_clipboard_items_by_content_key(&conn)?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_clipboard_content_key_unique ON clipboard_items(content_key)",
        [],
    )?;

    Ok(())
}

pub fn insert_clipboard_item(
    pool: &DbPool,
    item: &ClipboardItemDto,
) -> AppResult<ClipboardItemDto> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO clipboard_items (id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(content_key) DO UPDATE SET
             item_type = excluded.item_type,
             plain_text = excluded.plain_text,
             source_app = excluded.source_app,
             preview_path = COALESCE(excluded.preview_path, clipboard_items.preview_path),
             preview_data_url = COALESCE(excluded.preview_data_url, clipboard_items.preview_data_url),
             created_at = excluded.created_at",
        params![
            item.id,
            item.content_key,
            item.item_type,
            item.plain_text,
            item.source_app,
            item.preview_path,
            item.preview_data_url,
            item.created_at,
            if item.pinned { 1 } else { 0 },
        ],
    )?;

    let mut statement = conn.prepare(
        "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
         FROM clipboard_items
         WHERE content_key = ?1
         LIMIT 1",
    )?;
    let mut rows = statement.query(params![item.content_key.as_str()])?;
    if let Some(row) = rows.next()? {
        return Ok(map_clipboard_item_row(row)?);
    }

    Err(AppError::new(
        "clipboard_upsert_not_found",
        "写入剪贴板记录后读取失败",
    ))
}

pub fn list_clipboard_items(
    pool: &DbPool,
    filter: &ClipboardFilterDto,
) -> AppResult<Vec<ClipboardItemDto>> {
    let conn = pool.get()?;
    let limit = filter
        .limit
        .unwrap_or(100)
        .clamp(1, CLIPBOARD_LIST_LIMIT_MAX) as i64;
    let query = filter.query.clone().unwrap_or_default();

    let mut statement = conn.prepare(
        "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
         FROM clipboard_items
         WHERE (?1 = '' OR item_type = ?1)
           AND (?2 = '' OR plain_text LIKE ?3)
           AND (?4 = 0 OR pinned = 1)
         ORDER BY pinned DESC, created_at DESC
         LIMIT ?5",
    )?;

    let rows = statement.query_map(
        params![
            filter.item_type.clone().unwrap_or_default(),
            query,
            format!("%{}%", filter.query.clone().unwrap_or_default()),
            if filter.only_pinned.unwrap_or(false) {
                1
            } else {
                0
            },
            limit,
        ],
        map_clipboard_item_row,
    )?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }

    Ok(items)
}

pub fn get_clipboard_item(pool: &DbPool, id: &str) -> AppResult<Option<ClipboardItemDto>> {
    let conn = pool.get()?;
    let mut statement = conn.prepare(
        "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
         FROM clipboard_items
         WHERE id = ?1
         LIMIT 1",
    )?;

    let mut rows = statement.query(params![id])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(map_clipboard_item_row(row)?));
    }

    Ok(None)
}

pub fn pin_clipboard_item(pool: &DbPool, id: &str, pinned: bool) -> AppResult<()> {
    let conn = pool.get()?;
    conn.execute(
        "UPDATE clipboard_items SET pinned = ?1 WHERE id = ?2",
        params![if pinned { 1 } else { 0 }, id],
    )?;
    Ok(())
}

pub fn touch_clipboard_item(
    pool: &DbPool,
    id: &str,
    created_at: i64,
) -> AppResult<Option<ClipboardItemDto>> {
    let conn = pool.get()?;
    conn.execute(
        "UPDATE clipboard_items SET created_at = ?1 WHERE id = ?2",
        params![created_at, id],
    )?;

    let mut statement = conn.prepare(
        "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
         FROM clipboard_items
         WHERE id = ?1
         LIMIT 1",
    )?;
    let mut rows = statement.query(params![id])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(map_clipboard_item_row(row)?));
    }

    Ok(None)
}

pub fn delete_clipboard_item(pool: &DbPool, id: &str) -> AppResult<Option<String>> {
    let conn = pool.get()?;
    let preview_path = conn
        .query_row(
            "SELECT preview_path FROM clipboard_items WHERE id = ?1 LIMIT 1",
            params![id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
    Ok(preview_path)
}

pub fn clear_all_clipboard_items(pool: &DbPool) -> AppResult<Vec<String>> {
    let conn = pool.get()?;
    let mut statement = conn.prepare("SELECT preview_path FROM clipboard_items")?;
    let rows = statement.query_map([], |row| row.get::<_, Option<String>>(0))?;

    let mut preview_paths = Vec::new();
    for row in rows {
        if let Some(path) = row? {
            preview_paths.push(path);
        }
    }

    conn.execute("DELETE FROM clipboard_items", [])?;
    Ok(preview_paths)
}

fn preview_file_size_bytes(preview_path: Option<&str>) -> u64 {
    let Some(path) = preview_path else {
        return 0;
    };
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return 0;
    }

    match std::fs::metadata(trimmed) {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                tracing::warn!(
                    event = "clipboard_preview_size_missing",
                    preview_path = trimmed
                );
            } else {
                tracing::warn!(
                    event = "clipboard_preview_size_failed",
                    preview_path = trimmed,
                    error = error.to_string()
                );
            }
            0
        }
    }
}

fn clipboard_row_size_bytes(
    plain_text: &str,
    preview_data_url: Option<&str>,
    preview_path: Option<&str>,
) -> u64 {
    plain_text.len() as u64
        + preview_data_url
            .map(|value| value.len() as u64)
            .unwrap_or(0)
        + preview_file_size_bytes(preview_path)
}

pub fn prune_clipboard_items(
    pool: &DbPool,
    max_items: u32,
    max_total_size_bytes: Option<u64>,
) -> AppResult<Vec<PrunedClipboardItem>> {
    let mut conn = pool.get()?;
    let transaction = conn.transaction()?;

    if max_total_size_bytes.is_none() {
        let total: i64 =
            transaction.query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))?;
        let overflow = total - i64::from(max_items);
        if overflow <= 0 {
            transaction.commit()?;
            return Ok(Vec::new());
        }

        let mut to_remove = Vec::new();
        {
            let mut statement = transaction.prepare(
                "SELECT id, preview_path
                 FROM clipboard_items
                 ORDER BY pinned ASC, created_at ASC, id ASC
                 LIMIT ?1",
            )?;
            let rows = statement.query_map(params![overflow], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })?;

            for row in rows {
                let (id, preview_path) = row?;
                to_remove.push(PrunedClipboardItem { id, preview_path });
            }
        }

        for item in &to_remove {
            transaction.execute(
                "DELETE FROM clipboard_items WHERE id = ?1",
                params![item.id],
            )?;
        }

        transaction.commit()?;
        return Ok(to_remove);
    }

    let size_limit = max_total_size_bytes.unwrap_or(u64::MAX);
    let mut total_count: u64 = 0;
    let mut total_size: u64 = 0;
    let mut candidates = Vec::new();
    {
        let mut statement = transaction.prepare(
            "SELECT id, preview_path, plain_text, preview_data_url
             FROM clipboard_items
             ORDER BY pinned ASC, created_at ASC, id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;

        for row in rows {
            let (id, preview_path, plain_text, preview_data_url) = row?;
            let size_bytes = clipboard_row_size_bytes(
                plain_text.as_str(),
                preview_data_url.as_deref(),
                preview_path.as_deref(),
            );
            total_count += 1;
            total_size = total_size.saturating_add(size_bytes);
            candidates.push((id, preview_path, size_bytes));
        }
    }

    if total_count <= u64::from(max_items) && total_size <= size_limit {
        transaction.commit()?;
        return Ok(Vec::new());
    }

    let mut to_remove = Vec::new();
    for (id, preview_path, size_bytes) in candidates {
        if total_count <= u64::from(max_items) && total_size <= size_limit {
            break;
        }
        total_count = total_count.saturating_sub(1);
        total_size = total_size.saturating_sub(size_bytes);
        to_remove.push(PrunedClipboardItem { id, preview_path });
    }

    for item in &to_remove {
        transaction.execute(
            "DELETE FROM clipboard_items WHERE id = ?1",
            params![item.id],
        )?;
    }

    transaction.commit()?;
    Ok(to_remove)
}

pub fn get_clipboard_max_items(pool: &DbPool) -> AppResult<Option<u32>> {
    let conn = pool.get()?;
    let value = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1 LIMIT 1",
            params![CLIPBOARD_MAX_ITEMS_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    Ok(value.and_then(|raw| raw.parse::<u32>().ok()))
}

pub fn set_clipboard_max_items(pool: &DbPool, max_items: u32) -> AppResult<()> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![CLIPBOARD_MAX_ITEMS_KEY, max_items.to_string()],
    )?;
    Ok(())
}

pub fn get_clipboard_size_cleanup_enabled(pool: &DbPool) -> AppResult<Option<bool>> {
    let value = get_app_setting(pool, CLIPBOARD_SIZE_CLEANUP_ENABLED_KEY)?;
    Ok(value.and_then(|raw| raw.parse::<bool>().ok()))
}

pub fn set_clipboard_size_cleanup_enabled(pool: &DbPool, enabled: bool) -> AppResult<()> {
    set_app_setting(
        pool,
        CLIPBOARD_SIZE_CLEANUP_ENABLED_KEY,
        if enabled { "true" } else { "false" },
    )
}

pub fn get_clipboard_max_total_size_mb(pool: &DbPool) -> AppResult<Option<u32>> {
    let value = get_app_setting(pool, CLIPBOARD_MAX_TOTAL_SIZE_MB_KEY)?;
    Ok(value.and_then(|raw| raw.parse::<u32>().ok()))
}

pub fn set_clipboard_max_total_size_mb(pool: &DbPool, max_total_size_mb: u32) -> AppResult<()> {
    set_app_setting(
        pool,
        CLIPBOARD_MAX_TOTAL_SIZE_MB_KEY,
        &max_total_size_mb.to_string(),
    )
}

pub fn get_app_setting(pool: &DbPool, key: &str) -> AppResult<Option<String>> {
    let conn = pool.get()?;
    let value = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1 LIMIT 1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    Ok(value)
}

pub fn set_app_setting(pool: &DbPool, key: &str, value: &str) -> AppResult<()> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/infrastructure/db_tests.rs"]
mod tests;

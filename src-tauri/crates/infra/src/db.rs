use crate::clipboard::derive_content_key;
use app_core::models::{ClipboardFilterDto, ClipboardItemDto};
use app_core::{AppError, AppResult};
use libsql::{Builder, Connection, Error as LibsqlError, Row, params, params_from_iter};
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

pub type DbConn = Connection;

fn is_duplicate_column_error(error: LibsqlError) -> AppResult<()> {
    let message = error.to_string();
    if message.contains("duplicate column") {
        return Ok(());
    }
    Err(error.into())
}

fn map_clipboard_item_row(row: &Row) -> AppResult<ClipboardItemDto> {
    Ok(ClipboardItemDto {
        id: row.get(0)?,
        content_key: row.get(1)?,
        item_type: row.get(2)?,
        plain_text: row.get(3)?,
        source_app: row.get(4)?,
        preview_path: row.get(5)?,
        preview_data_url: row.get(6)?,
        created_at: row.get(7)?,
        pinned: row.get::<i64>(8)? == 1,
    })
}

async fn backfill_clipboard_content_keys(conn: &DbConn) -> AppResult<()> {
    let tx = conn.transaction().await?;
    let mut rows = tx
        .query(
            "SELECT id, item_type, plain_text, preview_path, preview_data_url
             FROM clipboard_items
             WHERE content_key IS NULL OR TRIM(content_key) = ''",
            (),
        )
        .await?;

    let mut updates = Vec::new();
    while let Some(row) = rows.next().await? {
        let id = row.get::<String>(0)?;
        let item_type = row.get::<String>(1)?;
        let plain_text = row.get::<String>(2)?;
        let preview_path = row.get::<Option<String>>(3)?;
        let preview_data_url = row.get::<Option<String>>(4)?;

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
        tx.execute(
            "UPDATE clipboard_items SET content_key = ?1 WHERE id = ?2",
            params![content_key, id],
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn has_missing_clipboard_content_keys(conn: &DbConn) -> AppResult<bool> {
    let mut rows = conn
        .query(
            "SELECT EXISTS(
                SELECT 1 FROM clipboard_items
                WHERE content_key IS NULL OR TRIM(content_key) = ''
                LIMIT 1
            )",
            (),
        )
        .await?;
    if let Some(row) = rows.next().await? {
        return Ok(row.get::<i64>(0)? == 1);
    }
    Ok(false)
}

async fn deduplicate_clipboard_items_by_content_key(conn: &DbConn) -> AppResult<()> {
    #[derive(Default)]
    struct DuplicateGroup {
        winner_id: String,
        winner_pinned: bool,
        pinned_any: bool,
        duplicate_ids: Vec<String>,
    }

    let mut groups: HashMap<String, DuplicateGroup> = HashMap::new();

    let tx = conn.transaction().await?;
    let mut rows = tx
        .query(
            "SELECT id, content_key, pinned
             FROM clipboard_items
             WHERE content_key IS NOT NULL AND TRIM(content_key) != ''
             ORDER BY content_key ASC, created_at DESC, id DESC",
            (),
        )
        .await?;

    while let Some(row) = rows.next().await? {
        let id = row.get::<String>(0)?;
        let content_key = row.get::<String>(1)?;
        let pinned = row.get::<i64>(2)? == 1;

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
            tx.execute(
                "UPDATE clipboard_items SET pinned = 1 WHERE id = ?1",
                params![group.winner_id.as_str()],
            )
            .await?;
        }

        for duplicate_id in &group.duplicate_ids {
            tx.execute(
                "DELETE FROM clipboard_items WHERE id = ?1",
                params![duplicate_id.as_str()],
            )
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

async fn has_duplicate_clipboard_content_keys(conn: &DbConn) -> AppResult<bool> {
    let mut rows = conn
        .query(
            "SELECT EXISTS(
                SELECT 1 FROM (
                    SELECT content_key
                    FROM clipboard_items
                    WHERE content_key IS NOT NULL AND TRIM(content_key) != ''
                    GROUP BY content_key
                    HAVING COUNT(*) > 1
                )
                LIMIT 1
            )",
            (),
        )
        .await?;
    if let Some(row) = rows.next().await? {
        return Ok(row.get::<i64>(0)? == 1);
    }
    Ok(false)
}

pub async fn open_db(db_path: &Path) -> AppResult<DbConn> {
    let database = Builder::new_local(db_path).build().await?;
    let conn = database.connect()?;
    Ok(conn)
}

async fn ensure_log_entries_fts_backfilled(conn: &DbConn) -> AppResult<()> {
    let mut rows = conn
        .query(
            "SELECT
                (SELECT COUNT(*) FROM log_entries),
                (SELECT COUNT(*) FROM log_entries_fts)",
            (),
        )
        .await?;
    let Some(row) = rows.next().await? else {
        return Ok(());
    };
    let log_count = row.get::<i64>(0)?.max(0);
    let fts_count = row.get::<i64>(1)?.max(0);
    if log_count > 0 && fts_count == 0 {
        conn.execute(
            "INSERT INTO log_entries_fts(log_entries_fts) VALUES ('rebuild')",
            (),
        )
        .await?;
    }
    Ok(())
}

pub async fn init_db(conn: &DbConn) -> AppResult<()> {
    let _ = conn
        .execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA busy_timeout = 3000;
            "#,
        )
        .await?;

    let _ = conn
        .execute_batch(
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

            CREATE VIRTUAL TABLE IF NOT EXISTS log_entries_fts USING fts5(
                message,
                event,
                scope,
                request_id,
                window_label,
                metadata,
                content='log_entries',
                content_rowid='id',
                tokenize='unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS log_entries_ai
            AFTER INSERT ON log_entries
            BEGIN
                INSERT INTO log_entries_fts(
                    rowid,
                    message,
                    event,
                    scope,
                    request_id,
                    window_label,
                    metadata
                ) VALUES (
                    NEW.id,
                    NEW.message,
                    NEW.event,
                    NEW.scope,
                    NEW.request_id,
                    COALESCE(NEW.window_label, ''),
                    COALESCE(NEW.metadata, '')
                );
            END;

            CREATE TRIGGER IF NOT EXISTS log_entries_ad
            AFTER DELETE ON log_entries
            BEGIN
                INSERT INTO log_entries_fts(
                    log_entries_fts,
                    rowid,
                    message,
                    event,
                    scope,
                    request_id,
                    window_label,
                    metadata
                ) VALUES (
                    'delete',
                    OLD.id,
                    OLD.message,
                    OLD.event,
                    OLD.scope,
                    OLD.request_id,
                    COALESCE(OLD.window_label, ''),
                    COALESCE(OLD.metadata, '')
                );
            END;

            CREATE TRIGGER IF NOT EXISTS log_entries_au
            AFTER UPDATE ON log_entries
            BEGIN
                INSERT INTO log_entries_fts(
                    log_entries_fts,
                    rowid,
                    message,
                    event,
                    scope,
                    request_id,
                    window_label,
                    metadata
                ) VALUES (
                    'delete',
                    OLD.id,
                    OLD.message,
                    OLD.event,
                    OLD.scope,
                    OLD.request_id,
                    COALESCE(OLD.window_label, ''),
                    COALESCE(OLD.metadata, '')
                );
                INSERT INTO log_entries_fts(
                    rowid,
                    message,
                    event,
                    scope,
                    request_id,
                    window_label,
                    metadata
                ) VALUES (
                    NEW.id,
                    NEW.message,
                    NEW.event,
                    NEW.scope,
                    NEW.request_id,
                    COALESCE(NEW.window_label, ''),
                    COALESCE(NEW.metadata, '')
                );
            END;

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

            CREATE TABLE IF NOT EXISTS launcher_index_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS launcher_index_entries (
                path TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                parent TEXT NOT NULL,
                ext TEXT,
                mtime INTEGER,
                size INTEGER,
                source_root TEXT NOT NULL,
                searchable_text TEXT NOT NULL,
                scan_token TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS launcher_index_entries_fts USING fts5(
                name,
                parent,
                path,
                ext,
                searchable_text,
                content='launcher_index_entries',
                content_rowid='rowid',
                tokenize='unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS launcher_index_entries_ai
            AFTER INSERT ON launcher_index_entries
            BEGIN
                INSERT INTO launcher_index_entries_fts(
                    rowid,
                    name,
                    parent,
                    path,
                    ext,
                    searchable_text
                ) VALUES (
                    NEW.rowid,
                    NEW.name,
                    NEW.parent,
                    NEW.path,
                    COALESCE(NEW.ext, ''),
                    NEW.searchable_text
                );
            END;

            CREATE TRIGGER IF NOT EXISTS launcher_index_entries_ad
            AFTER DELETE ON launcher_index_entries
            BEGIN
                INSERT INTO launcher_index_entries_fts(
                    launcher_index_entries_fts,
                    rowid,
                    name,
                    parent,
                    path,
                    ext,
                    searchable_text
                ) VALUES (
                    'delete',
                    OLD.rowid,
                    OLD.name,
                    OLD.parent,
                    OLD.path,
                    COALESCE(OLD.ext, ''),
                    OLD.searchable_text
                );
            END;

            CREATE TRIGGER IF NOT EXISTS launcher_index_entries_au
            AFTER UPDATE ON launcher_index_entries
            BEGIN
                INSERT INTO launcher_index_entries_fts(
                    launcher_index_entries_fts,
                    rowid,
                    name,
                    parent,
                    path,
                    ext,
                    searchable_text
                ) VALUES (
                    'delete',
                    OLD.rowid,
                    OLD.name,
                    OLD.parent,
                    OLD.path,
                    COALESCE(OLD.ext, ''),
                    OLD.searchable_text
                );
                INSERT INTO launcher_index_entries_fts(
                    rowid,
                    name,
                    parent,
                    path,
                    ext,
                    searchable_text
                ) VALUES (
                    NEW.rowid,
                    NEW.name,
                    NEW.parent,
                    NEW.path,
                    COALESCE(NEW.ext, ''),
                    NEW.searchable_text
                );
            END;

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
            CREATE INDEX IF NOT EXISTS idx_launcher_index_kind_name ON launcher_index_entries(kind, name COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_launcher_index_source_root_name ON launcher_index_entries(source_root, name COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_launcher_index_scan_token ON launcher_index_entries(scan_token);
            CREATE INDEX IF NOT EXISTS idx_launcher_index_source_root_scan_token ON launcher_index_entries(source_root, scan_token);
            "#,
        )
        .await?;

    if let Err(error) = conn
        .execute(
            "ALTER TABLE clipboard_items ADD COLUMN preview_path TEXT",
            (),
        )
        .await
    {
        is_duplicate_column_error(error)?;
    }

    if let Err(error) = conn
        .execute(
            "ALTER TABLE clipboard_items ADD COLUMN preview_data_url TEXT",
            (),
        )
        .await
    {
        is_duplicate_column_error(error)?;
    }

    if let Err(error) = conn
        .execute(
            "ALTER TABLE clipboard_items ADD COLUMN content_key TEXT",
            (),
        )
        .await
    {
        is_duplicate_column_error(error)?;
    }

    if has_missing_clipboard_content_keys(conn).await? {
        backfill_clipboard_content_keys(conn).await?;
    }
    if has_duplicate_clipboard_content_keys(conn).await? {
        deduplicate_clipboard_items_by_content_key(conn).await?;
    }
    ensure_log_entries_fts_backfilled(conn).await?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_clipboard_content_key_unique ON clipboard_items(content_key)",
        (),
    )
    .await?;
    conn.execute_batch("PRAGMA optimize;").await?;
    Ok(())
}

pub async fn insert_clipboard_item(
    conn: &DbConn,
    item: &ClipboardItemDto,
) -> AppResult<ClipboardItemDto> {
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
            item.id.as_str(),
            item.content_key.as_str(),
            item.item_type.as_str(),
            item.plain_text.as_str(),
            item.source_app.as_deref(),
            item.preview_path.as_deref(),
            item.preview_data_url.as_deref(),
            item.created_at,
            if item.pinned { 1 } else { 0 },
        ],
    )
    .await?;

    let mut rows = conn
        .query(
            "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
             FROM clipboard_items
             WHERE content_key = ?1
             LIMIT 1",
            params![item.content_key.as_str()],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        return map_clipboard_item_row(&row);
    }

    Err(AppError::new(
        "clipboard_upsert_not_found",
        "写入剪贴板记录后读取失败",
    ))
}

pub async fn list_clipboard_items(
    conn: &DbConn,
    filter: &ClipboardFilterDto,
) -> AppResult<Vec<ClipboardItemDto>> {
    let limit = filter
        .limit
        .unwrap_or(100)
        .clamp(1, CLIPBOARD_LIST_LIMIT_MAX) as i64;
    let query = filter.query.clone().unwrap_or_default();

    let mut rows = conn
        .query(
            "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
             FROM clipboard_items
             WHERE (?1 = '' OR item_type = ?1)
               AND (?2 = '' OR plain_text LIKE ?3)
               AND (?4 = 0 OR pinned = 1)
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?5",
            params![
                filter.item_type.clone().unwrap_or_default(),
                query,
                format!("%{}%", filter.query.clone().unwrap_or_default()),
                if filter.only_pinned.unwrap_or(false) { 1 } else { 0 },
                limit,
            ],
        )
        .await?;

    let mut items = Vec::new();
    while let Some(row) = rows.next().await? {
        items.push(map_clipboard_item_row(&row)?);
    }

    Ok(items)
}

pub async fn get_clipboard_item(conn: &DbConn, id: &str) -> AppResult<Option<ClipboardItemDto>> {
    let mut rows = conn
        .query(
            "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
             FROM clipboard_items
             WHERE id = ?1
             LIMIT 1",
            params![id],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        return Ok(Some(map_clipboard_item_row(&row)?));
    }

    Ok(None)
}

pub async fn pin_clipboard_item(conn: &DbConn, id: &str, pinned: bool) -> AppResult<()> {
    conn.execute(
        "UPDATE clipboard_items SET pinned = ?1 WHERE id = ?2",
        params![if pinned { 1 } else { 0 }, id],
    )
    .await?;
    Ok(())
}

pub async fn touch_clipboard_item(
    conn: &DbConn,
    id: &str,
    created_at: i64,
) -> AppResult<Option<ClipboardItemDto>> {
    conn.execute(
        "UPDATE clipboard_items SET created_at = ?1 WHERE id = ?2",
        params![created_at, id],
    )
    .await?;

    get_clipboard_item(conn, id).await
}

pub async fn delete_clipboard_item(conn: &DbConn, id: &str) -> AppResult<Option<String>> {
    let mut rows = conn
        .query(
            "SELECT preview_path FROM clipboard_items WHERE id = ?1 LIMIT 1",
            params![id],
        )
        .await?;
    let preview_path = if let Some(row) = rows.next().await? {
        row.get::<Option<String>>(0)?
    } else {
        None
    };

    conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])
        .await?;
    Ok(preview_path)
}

pub async fn clear_all_clipboard_items(conn: &DbConn) -> AppResult<Vec<String>> {
    let mut rows = conn
        .query("SELECT preview_path FROM clipboard_items", ())
        .await?;
    let mut preview_paths = Vec::new();
    while let Some(row) = rows.next().await? {
        if let Some(path) = row.get::<Option<String>>(0)? {
            preview_paths.push(path);
        }
    }

    conn.execute("DELETE FROM clipboard_items", ()).await?;
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

pub async fn prune_clipboard_items(
    conn: &DbConn,
    max_items: u32,
    max_total_size_bytes: Option<u64>,
) -> AppResult<Vec<PrunedClipboardItem>> {
    let transaction = conn.transaction().await?;

    if max_total_size_bytes.is_none() {
        let mut rows = transaction
            .query("SELECT COUNT(*) FROM clipboard_items", ())
            .await?;
        let total = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            0
        };

        let overflow = total - i64::from(max_items);
        if overflow <= 0 {
            transaction.commit().await?;
            return Ok(Vec::new());
        }

        let mut to_remove = Vec::new();
        let mut rows = transaction
            .query(
                "SELECT id, preview_path
                 FROM clipboard_items
                 ORDER BY pinned ASC, created_at ASC, id ASC
                 LIMIT ?1",
                params![overflow],
            )
            .await?;

        while let Some(row) = rows.next().await? {
            to_remove.push(PrunedClipboardItem {
                id: row.get::<String>(0)?,
                preview_path: row.get::<Option<String>>(1)?,
            });
        }

        for item in &to_remove {
            transaction
                .execute(
                    "DELETE FROM clipboard_items WHERE id = ?1",
                    params![item.id.as_str()],
                )
                .await?;
        }

        transaction.commit().await?;
        return Ok(to_remove);
    }

    let size_limit = max_total_size_bytes.unwrap_or(u64::MAX);
    let mut total_count: u64 = 0;
    let mut total_size: u64 = 0;
    let mut candidates = Vec::new();

    let mut rows = transaction
        .query(
            "SELECT id, preview_path, plain_text, preview_data_url
             FROM clipboard_items
             ORDER BY pinned ASC, created_at ASC, id ASC",
            (),
        )
        .await?;

    while let Some(row) = rows.next().await? {
        let id = row.get::<String>(0)?;
        let preview_path = row.get::<Option<String>>(1)?;
        let plain_text = row.get::<String>(2)?;
        let preview_data_url = row.get::<Option<String>>(3)?;
        let size_bytes = clipboard_row_size_bytes(
            plain_text.as_str(),
            preview_data_url.as_deref(),
            preview_path.as_deref(),
        );

        total_count += 1;
        total_size = total_size.saturating_add(size_bytes);
        candidates.push((id, preview_path, size_bytes));
    }

    if total_count <= u64::from(max_items) && total_size <= size_limit {
        transaction.commit().await?;
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
        transaction
            .execute(
                "DELETE FROM clipboard_items WHERE id = ?1",
                params![item.id.as_str()],
            )
            .await?;
    }

    transaction.commit().await?;
    Ok(to_remove)
}

pub async fn get_app_setting(conn: &DbConn, key: &str) -> AppResult<Option<String>> {
    let mut rows = conn
        .query(
            "SELECT value FROM app_settings WHERE key = ?1 LIMIT 1",
            params![key],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        return Ok(Some(row.get::<String>(0)?));
    }

    Ok(None)
}

pub async fn set_app_setting(conn: &DbConn, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .await?;
    Ok(())
}

pub async fn get_app_settings_batch(
    conn: &DbConn,
    keys: &[&str],
) -> AppResult<HashMap<String, String>> {
    if keys.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = (1..=keys.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT key, value FROM app_settings WHERE key IN ({placeholders})");
    let mut rows = conn
        .query(sql.as_str(), params_from_iter(keys.iter().copied()))
        .await?;

    let mut values = HashMap::with_capacity(keys.len());
    while let Some(row) = rows.next().await? {
        let key = row.get::<String>(0)?;
        let value = row.get::<String>(1)?;
        values.insert(key, value);
    }
    Ok(values)
}

pub async fn set_app_settings_batch(conn: &DbConn, entries: &[(&str, &str)]) -> AppResult<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction().await?;
    for (key, value) in entries {
        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![*key, *value],
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn delete_app_settings(conn: &DbConn, keys: &[&str]) -> AppResult<()> {
    if keys.is_empty() {
        return Ok(());
    }

    let placeholders = (1..=keys.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("DELETE FROM app_settings WHERE key IN ({placeholders})");
    conn.execute(sql.as_str(), params_from_iter(keys.iter().copied()))
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/infrastructure/db_tests.rs"]
mod tests;

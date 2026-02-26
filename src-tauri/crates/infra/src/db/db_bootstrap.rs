use super::DbConn;
use crate::clipboard::derive_content_key;
use crate::db_error::DbResult;
use libsql::{Builder, Error as LibsqlError, params};
use std::collections::HashMap;
use std::path::Path;

fn is_duplicate_column_error(error: LibsqlError) -> DbResult<()> {
    let message = error.to_string();
    if message.contains("duplicate column") {
        return Ok(());
    }
    Err(error.into())
}

async fn backfill_clipboard_content_keys(conn: &DbConn) -> DbResult<()> {
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

async fn has_missing_clipboard_content_keys(conn: &DbConn) -> DbResult<bool> {
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

async fn deduplicate_clipboard_items_by_content_key(conn: &DbConn) -> DbResult<()> {
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

async fn has_duplicate_clipboard_content_keys(conn: &DbConn) -> DbResult<bool> {
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

pub async fn open_db(db_path: &Path) -> DbResult<DbConn> {
    let database = Builder::new_local(db_path).build().await?;
    let conn = database.connect()?;
    Ok(conn)
}

async fn ensure_log_entries_fts_backfilled(conn: &DbConn) -> DbResult<()> {
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

pub async fn init_db(conn: &DbConn) -> DbResult<()> {
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

use super::*;
use libsql::params;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_db_path(prefix: &str) -> std::path::PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_millis();
    std::env::temp_dir().join(format!("rtool-{prefix}-{}-{now}.db", std::process::id()))
}

async fn setup_temp_db(prefix: &str) -> (DbConn, std::path::PathBuf) {
    let path = unique_temp_db_path(prefix);
    let conn = open_db(path.as_path()).await.expect("open db");
    init_db(&conn).await.expect("init db");
    (conn, path)
}

async fn insert_raw_item(conn: &DbConn, id: &str, plain_text: &str, created_at: i64, pinned: bool) {
    conn.execute(
        "INSERT INTO clipboard_items (id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned)
             VALUES (?1, ?2, 'text', ?3, NULL, NULL, NULL, ?4, ?5)",
        params![
            id,
            format!("key-{id}"),
            plain_text,
            created_at,
            if pinned { 1 } else { 0 },
        ],
    )
    .await
    .expect("insert clipboard item");
}

async fn count_clipboard_items(conn: &DbConn) -> i64 {
    let mut rows = conn
        .query("SELECT COUNT(*) FROM clipboard_items", ())
        .await
        .expect("query count");
    let row = rows.next().await.expect("next row").expect("row missing");
    row.get::<i64>(0).expect("count value")
}

async fn list_clipboard_ids(conn: &DbConn) -> Vec<String> {
    let mut rows = conn
        .query("SELECT id FROM clipboard_items ORDER BY created_at ASC", ())
        .await
        .expect("query ids");
    let mut ids = Vec::new();
    while let Some(row) = rows.next().await.expect("next row") {
        ids.push(row.get::<String>(0).expect("id"));
    }
    ids
}

#[tokio::test]
async fn prune_by_size_should_remove_oldest_items() {
    let (conn, db_path) = setup_temp_db("prune-size").await;
    let item_1 = "a".repeat(100);
    let item_2 = "b".repeat(100);
    let item_3 = "c".repeat(100);
    insert_raw_item(&conn, "item-1", item_1.as_str(), 1, false).await;
    insert_raw_item(&conn, "item-2", item_2.as_str(), 2, false).await;
    insert_raw_item(&conn, "item-3", item_3.as_str(), 3, false).await;

    let removed = prune_clipboard_items(&conn, 10, Some(150))
        .await
        .expect("prune by size");
    let removed_ids: Vec<String> = removed.into_iter().map(|item| item.id).collect();
    assert_eq!(
        removed_ids,
        vec!["item-1".to_string(), "item-2".to_string()]
    );

    assert_eq!(count_clipboard_items(&conn).await, 1);
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn prune_should_apply_count_limit_when_size_cleanup_disabled() {
    let (conn, db_path) = setup_temp_db("prune-count").await;
    insert_raw_item(&conn, "item-1", "a", 1, false).await;
    insert_raw_item(&conn, "item-2", "b", 2, false).await;
    insert_raw_item(&conn, "item-3", "c", 3, false).await;

    let removed = prune_clipboard_items(&conn, 2, None)
        .await
        .expect("prune by count");
    let removed_ids: Vec<String> = removed.into_iter().map(|item| item.id).collect();
    assert_eq!(removed_ids, vec!["item-1".to_string()]);

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn prune_should_apply_count_and_size_constraints_together() {
    let (conn, db_path) = setup_temp_db("prune-combined").await;
    let item_1 = "x".repeat(120);
    let item_2 = "y".repeat(120);
    let item_3 = "z".repeat(120);
    insert_raw_item(&conn, "item-1", item_1.as_str(), 1, false).await;
    insert_raw_item(&conn, "item-2", item_2.as_str(), 2, false).await;
    insert_raw_item(&conn, "item-3", item_3.as_str(), 3, false).await;

    let removed = prune_clipboard_items(&conn, 2, Some(220))
        .await
        .expect("prune by count and size");
    let removed_ids: Vec<String> = removed.into_iter().map(|item| item.id).collect();
    assert_eq!(
        removed_ids,
        vec!["item-1".to_string(), "item-2".to_string()]
    );
    assert_eq!(list_clipboard_ids(&conn).await, vec!["item-3".to_string()]);

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn init_db_should_be_idempotent_and_keep_launcher_fts_available() {
    let db_path = unique_temp_db_path("init-db-idempotent");
    let conn = open_db(db_path.as_path()).await.expect("open db");
    init_db(&conn).await.expect("init db first");
    init_db(&conn).await.expect("init db second");

    let mut pragma_rows = conn
        .query("PRAGMA foreign_keys", ())
        .await
        .expect("query pragma foreign_keys");
    let pragma_row = pragma_rows
        .next()
        .await
        .expect("next pragma row")
        .expect("pragma row missing");
    assert_eq!(
        pragma_row
            .get::<i64>(0)
            .expect("pragma foreign_keys value should exist"),
        1
    );

    conn.execute(
        "INSERT INTO launcher_index_entries (path, kind, name, parent, ext, mtime, size, source_root, searchable_text, scan_token)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, ?5, ?6, ?7)",
        params![
            "/tmp/rtool.txt",
            "file",
            "rtool.txt",
            "/tmp",
            "/tmp",
            "rtool txt",
            "token-1"
        ],
    )
    .await
    .expect("insert launcher index entry");

    let mut rows = conn
        .query(
            "SELECT path FROM launcher_index_entries_fts WHERE launcher_index_entries_fts MATCH ?1 LIMIT 1",
            params!["rtool*"],
        )
        .await
        .expect("query launcher fts");
    let row = rows.next().await.expect("next row");
    assert!(row.is_some(), "launcher FTS should return indexed row");

    conn.execute(
        "INSERT INTO log_entries (timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL)",
        params![
            100_i64,
            "info",
            "test_scope",
            "db_init_event",
            "req-1",
            "window-main",
            "fts should index this message",
            "{\"hello\":\"world\"}",
        ],
    )
    .await
    .expect("insert log entry");

    let mut log_rows = conn
        .query(
            "SELECT rowid FROM log_entries_fts WHERE log_entries_fts MATCH ?1 LIMIT 1",
            params!["fts* AND message*"],
        )
        .await
        .expect("query log entries fts");
    let log_row = log_rows.next().await.expect("next log row");
    assert!(log_row.is_some(), "log FTS should return indexed row");

    let _ = std::fs::remove_file(db_path);
}

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_db_path(prefix: &str) -> std::path::PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_millis();
    std::env::temp_dir().join(format!("rtool-{prefix}-{}-{now}.db", std::process::id()))
}

fn setup_temp_db(prefix: &str) -> (DbPool, std::path::PathBuf) {
    let path = unique_temp_db_path(prefix);
    init_db(path.as_path()).expect("init db");
    let pool = new_db_pool(path.as_path()).expect("new db pool");
    (pool, path)
}

fn insert_raw_item(pool: &DbPool, id: &str, plain_text: &str, created_at: i64, pinned: bool) {
    let conn = pool.get().expect("db conn");
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
    .expect("insert clipboard item");
}

#[test]
fn prune_by_size_should_remove_oldest_items() {
    let (pool, db_path) = setup_temp_db("prune-size");
    let item_1 = "a".repeat(100);
    let item_2 = "b".repeat(100);
    let item_3 = "c".repeat(100);
    insert_raw_item(&pool, "item-1", item_1.as_str(), 1, false);
    insert_raw_item(&pool, "item-2", item_2.as_str(), 2, false);
    insert_raw_item(&pool, "item-3", item_3.as_str(), 3, false);

    let removed = prune_clipboard_items(&pool, 10, Some(150)).expect("prune by size");
    let removed_ids: Vec<String> = removed.into_iter().map(|item| item.id).collect();
    assert_eq!(
        removed_ids,
        vec!["item-1".to_string(), "item-2".to_string()]
    );

    let conn = pool.get().expect("db conn");
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))
        .expect("count remaining");
    assert_eq!(remaining, 1);

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn prune_should_apply_count_limit_when_size_cleanup_disabled() {
    let (pool, db_path) = setup_temp_db("prune-count");
    insert_raw_item(&pool, "item-1", "a", 1, false);
    insert_raw_item(&pool, "item-2", "b", 2, false);
    insert_raw_item(&pool, "item-3", "c", 3, false);

    let removed = prune_clipboard_items(&pool, 2, None).expect("prune by count");
    let removed_ids: Vec<String> = removed.into_iter().map(|item| item.id).collect();
    assert_eq!(removed_ids, vec!["item-1".to_string()]);

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn prune_should_apply_count_and_size_constraints_together() {
    let (pool, db_path) = setup_temp_db("prune-combined");
    let item_1 = "x".repeat(120);
    let item_2 = "y".repeat(120);
    let item_3 = "z".repeat(120);
    insert_raw_item(&pool, "item-1", item_1.as_str(), 1, false);
    insert_raw_item(&pool, "item-2", item_2.as_str(), 2, false);
    insert_raw_item(&pool, "item-3", item_3.as_str(), 3, false);

    let removed = prune_clipboard_items(&pool, 2, Some(220)).expect("prune by count and size");
    let removed_ids: Vec<String> = removed.into_iter().map(|item| item.id).collect();
    assert_eq!(
        removed_ids,
        vec!["item-1".to_string(), "item-2".to_string()]
    );

    let conn = pool.get().expect("db conn");
    let remaining_ids: Vec<String> = conn
        .prepare("SELECT id FROM clipboard_items ORDER BY created_at ASC")
        .expect("prepare query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query map")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect ids");
    assert_eq!(remaining_ids, vec!["item-3".to_string()]);

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn init_db_should_be_idempotent_and_keep_launcher_fts_available() {
    let db_path = unique_temp_db_path("init-db-idempotent");
    init_db(db_path.as_path()).expect("init db first");
    init_db(db_path.as_path()).expect("init db second");

    let conn = Connection::open(db_path.as_path()).expect("open db");
    let launcher_table_exists: i64 = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'launcher_index_entries')",
            [],
            |row| row.get(0),
        )
        .expect("query launcher_index_entries");
    let launcher_fts_exists: i64 = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'launcher_index_entries_fts')",
            [],
            |row| row.get(0),
        )
        .expect("query launcher_index_entries_fts");

    assert_eq!(launcher_table_exists, 1);
    assert_eq!(launcher_fts_exists, 1);
    let _ = std::fs::remove_file(db_path);
}

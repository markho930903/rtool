use super::*;
use crate::infrastructure::db::{init_db, new_db_pool};

fn setup_temp_db(prefix: &str) -> (DbPool, std::path::PathBuf) {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time before epoch")
        .as_millis();
    let path = std::env::temp_dir().join(format!("rtool-transfer-{prefix}-{millis}.db"));
    init_db(path.as_path()).expect("init db");
    let pool = new_db_pool(path.as_path()).expect("new db pool");
    (pool, path)
}

#[test]
fn load_settings_should_include_tuning_defaults() {
    let (pool, path) = setup_temp_db("settings-defaults");
    let settings =
        load_settings(&pool, "/tmp/downloads".to_string()).expect("load transfer settings");
    assert!(settings.pipeline_v2_enabled);
    assert!(settings.codec_v2_enabled);
    assert_eq!(settings.db_flush_interval_ms, 400);
    assert_eq!(settings.event_emit_interval_ms, 250);
    assert_eq!(settings.ack_batch_size, 64);
    assert_eq!(settings.ack_flush_interval_ms, 20);
    let _ = std::fs::remove_file(path);
}

#[test]
fn upsert_files_batch_should_persist_multi_rows() {
    let (pool, path) = setup_temp_db("batch-upsert");
    let session = TransferSessionDto {
        id: "session-batch".to_string(),
        direction: TransferDirection::Send,
        peer_device_id: "peer-1".to_string(),
        peer_name: "peer".to_string(),
        status: TransferStatus::Running,
        total_bytes: 10,
        transferred_bytes: 0,
        avg_speed_bps: 0,
        save_dir: "/tmp".to_string(),
        created_at: 1,
        started_at: Some(1),
        finished_at: None,
        error_code: None,
        error_message: None,
        cleanup_after_at: None,
        files: Vec::new(),
    };
    insert_session(&pool, &session).expect("insert session");

    let file_a = TransferFileDto {
        id: "file-a".to_string(),
        session_id: session.id.clone(),
        relative_path: "a.txt".to_string(),
        source_path: Some("/tmp/a.txt".to_string()),
        target_path: None,
        size_bytes: 8,
        transferred_bytes: 4,
        chunk_size: 4,
        chunk_count: 2,
        status: TransferStatus::Running,
        blake3: None,
        mime_type: None,
        preview_kind: None,
        preview_data: None,
        is_folder_archive: false,
    };
    let file_b = TransferFileDto {
        id: "file-b".to_string(),
        session_id: session.id.clone(),
        relative_path: "b.txt".to_string(),
        source_path: Some("/tmp/b.txt".to_string()),
        target_path: None,
        size_bytes: 8,
        transferred_bytes: 8,
        chunk_size: 4,
        chunk_count: 2,
        status: TransferStatus::Success,
        blake3: None,
        mime_type: None,
        preview_kind: None,
        preview_data: None,
        is_folder_archive: false,
    };

    let items = vec![
        TransferFilePersistItem {
            file: file_a,
            completed_bitmap: vec![0b0000_0001],
        },
        TransferFilePersistItem {
            file: file_b,
            completed_bitmap: vec![0b0000_0011],
        },
    ];
    upsert_files_batch(&pool, items.as_slice()).expect("batch upsert");

    let conn = pool.get().expect("db conn");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transfer_files", [], |row| row.get(0))
        .expect("count files");
    assert_eq!(count, 2);

    let _ = std::fs::remove_file(path);
}

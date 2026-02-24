use super::*;
use crate::db::{init_db, new_db_pool};

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

#[test]
fn upsert_peer_should_keep_existing_block_state() {
    let (pool, path) = setup_temp_db("peer-block-state");
    let blocked_until = 1_735_000_000_000_i64;

    mark_peer_pair_failure(&pool, "peer-locked", Some(blocked_until)).expect("mark pair failure");
    let incoming_online_peer = TransferPeerDto {
        device_id: "peer-locked".to_string(),
        display_name: "Peer Locked".to_string(),
        address: "127.0.0.1".to_string(),
        listen_port: 53321,
        last_seen_at: blocked_until + 1000,
        paired_at: None,
        trust_level: TransferPeerTrustLevel::Online,
        failed_attempts: 0,
        blocked_until: None,
        pairing_required: true,
        online: true,
    };
    upsert_peer(&pool, &incoming_online_peer).expect("upsert online peer");

    let stored = get_peer_by_device_id(&pool, "peer-locked")
        .expect("query peer by device id")
        .expect("peer exists");
    assert_eq!(stored.failed_attempts, 1);
    assert_eq!(stored.blocked_until, Some(blocked_until));

    let _ = std::fs::remove_file(path);
}

#[test]
fn list_history_should_attach_files_for_multiple_sessions() {
    let (pool, path) = setup_temp_db("history-batch-files");

    let session_a = TransferSessionDto {
        id: "session-a".to_string(),
        direction: TransferDirection::Send,
        peer_device_id: "peer-a".to_string(),
        peer_name: "Peer A".to_string(),
        status: TransferStatus::Success,
        total_bytes: 8,
        transferred_bytes: 8,
        avg_speed_bps: 10,
        save_dir: "/tmp".to_string(),
        created_at: 200,
        started_at: Some(190),
        finished_at: Some(210),
        error_code: None,
        error_message: None,
        cleanup_after_at: None,
        files: Vec::new(),
    };
    let session_b = TransferSessionDto {
        id: "session-b".to_string(),
        direction: TransferDirection::Receive,
        peer_device_id: "peer-b".to_string(),
        peer_name: "Peer B".to_string(),
        status: TransferStatus::Running,
        total_bytes: 16,
        transferred_bytes: 4,
        avg_speed_bps: 5,
        save_dir: "/tmp".to_string(),
        created_at: 100,
        started_at: Some(90),
        finished_at: None,
        error_code: None,
        error_message: None,
        cleanup_after_at: None,
        files: Vec::new(),
    };
    insert_session(&pool, &session_a).expect("insert session a");
    insert_session(&pool, &session_b).expect("insert session b");

    insert_or_update_file(
        &pool,
        &TransferFileDto {
            id: "file-a".to_string(),
            session_id: "session-a".to_string(),
            relative_path: "a.txt".to_string(),
            source_path: Some("/tmp/a.txt".to_string()),
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
        },
        &[0b0000_0011],
    )
    .expect("insert file a");
    insert_or_update_file(
        &pool,
        &TransferFileDto {
            id: "file-b".to_string(),
            session_id: "session-b".to_string(),
            relative_path: "b.txt".to_string(),
            source_path: Some("/tmp/b.txt".to_string()),
            target_path: None,
            size_bytes: 16,
            transferred_bytes: 4,
            chunk_size: 4,
            chunk_count: 4,
            status: TransferStatus::Running,
            blake3: None,
            mime_type: None,
            preview_kind: None,
            preview_data: None,
            is_folder_archive: false,
        },
        &[0b0000_0001],
    )
    .expect("insert file b");

    let page = list_history(&pool, &TransferHistoryFilterDto::default()).expect("list history");
    assert_eq!(page.items.len(), 2);
    assert!(page.items.iter().all(|item| item.files.len() == 1));

    let _ = std::fs::remove_file(path);
}

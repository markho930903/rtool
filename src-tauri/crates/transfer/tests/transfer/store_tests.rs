use super::*;
use crate::db::{DbConn, init_db, open_db};

async fn setup_temp_db(prefix: &str) -> (DbConn, std::path::PathBuf) {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time before epoch")
        .as_millis();
    let path = std::env::temp_dir().join(format!("rtool-transfer-{prefix}-{millis}.db"));
    let conn = open_db(path.as_path()).await.expect("open db");
    init_db(&conn).await.expect("init db");
    (conn, path)
}

async fn count_transfer_files(conn: &DbConn) -> i64 {
    let mut rows = conn
        .query("SELECT COUNT(*) FROM transfer_files", ())
        .await
        .expect("query count");
    let row = rows.next().await.expect("next row").expect("row missing");
    row.get::<i64>(0).expect("count value")
}

async fn count_transfer_sessions(conn: &DbConn) -> i64 {
    let mut rows = conn
        .query("SELECT COUNT(*) FROM transfer_sessions", ())
        .await
        .expect("query count");
    let row = rows.next().await.expect("next row").expect("row missing");
    row.get::<i64>(0).expect("count value")
}

#[tokio::test]
async fn load_settings_should_include_tuning_defaults() {
    let (conn, path) = setup_temp_db("settings-defaults").await;
    let settings = load_settings(&conn, "/tmp/downloads".to_string())
        .await
        .expect("load transfer settings");
    assert_eq!(settings.db_flush_interval_ms, 400);
    assert_eq!(settings.event_emit_interval_ms, 250);
    assert_eq!(settings.ack_batch_size, 64);
    assert_eq!(settings.ack_flush_interval_ms, 20);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn upsert_files_batch_should_persist_multi_rows() {
    let (conn, path) = setup_temp_db("batch-upsert").await;
    let session = TransferSessionDto {
        id: "session-batch".to_string(),
        direction: TransferDirection::Send,
        peer_device_id: "peer-1".to_string(),
        peer_name: "peer".to_string(),
        status: TransferStatus::Running,
        total_bytes: 10,
        transferred_bytes: 0,
        avg_speed_bps: 0,
        rtt_ms_p50: None,
        rtt_ms_p95: None,
        save_dir: "/tmp".to_string(),
        created_at: 1,
        started_at: Some(1),
        finished_at: None,
        error_code: None,
        error_message: None,
        cleanup_after_at: None,
        files: Vec::new(),
    };
    insert_session(&conn, &session)
        .await
        .expect("insert session");

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
    upsert_files_batch(&conn, items.as_slice())
        .await
        .expect("batch upsert");

    assert_eq!(count_transfer_files(&conn).await, 2);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn upsert_peer_should_keep_existing_block_state() {
    let (conn, path) = setup_temp_db("peer-block-state").await;
    let blocked_until = 1_735_000_000_000_i64;

    mark_peer_pair_failure(&conn, "peer-locked", Some(blocked_until))
        .await
        .expect("mark pair failure");
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
    upsert_peer(&conn, &incoming_online_peer)
        .await
        .expect("upsert online peer");

    let stored = get_peer_by_device_id(&conn, "peer-locked")
        .await
        .expect("query peer by device id")
        .expect("peer exists");
    assert_eq!(stored.failed_attempts, 1);
    assert_eq!(stored.blocked_until, Some(blocked_until));

    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn list_history_should_attach_files_for_multiple_sessions() {
    let (conn, path) = setup_temp_db("history-batch-files").await;

    let session_a = TransferSessionDto {
        id: "session-a".to_string(),
        direction: TransferDirection::Send,
        peer_device_id: "peer-a".to_string(),
        peer_name: "Peer A".to_string(),
        status: TransferStatus::Success,
        total_bytes: 8,
        transferred_bytes: 8,
        avg_speed_bps: 10,
        rtt_ms_p50: None,
        rtt_ms_p95: None,
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
        rtt_ms_p50: None,
        rtt_ms_p95: None,
        save_dir: "/tmp".to_string(),
        created_at: 100,
        started_at: Some(90),
        finished_at: None,
        error_code: None,
        error_message: None,
        cleanup_after_at: None,
        files: Vec::new(),
    };
    insert_session(&conn, &session_a)
        .await
        .expect("insert session a");
    insert_session(&conn, &session_b)
        .await
        .expect("insert session b");

    insert_or_update_file(
        &conn,
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
    .await
    .expect("insert file a");
    insert_or_update_file(
        &conn,
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
    .await
    .expect("insert file b");

    let page = list_history(&conn, &TransferHistoryFilterDto::default())
        .await
        .expect("list history");
    assert_eq!(page.items.len(), 2);
    assert!(page.items.iter().all(|item| item.files.len() == 1));

    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn list_history_cursor_should_remain_stable_for_same_timestamp() {
    let (conn, path) = setup_temp_db("history-stable-cursor").await;

    let sessions = [
        ("session-c", 300_i64),
        ("session-b", 300_i64),
        ("session-a", 300_i64),
    ];
    for (id, created_at) in sessions {
        insert_session(
            &conn,
            &TransferSessionDto {
                id: id.to_string(),
                direction: TransferDirection::Send,
                peer_device_id: "peer".to_string(),
                peer_name: "Peer".to_string(),
                status: TransferStatus::Success,
                total_bytes: 1,
                transferred_bytes: 1,
                avg_speed_bps: 1,
                rtt_ms_p50: None,
                rtt_ms_p95: None,
                save_dir: "/tmp".to_string(),
                created_at,
                started_at: Some(created_at),
                finished_at: Some(created_at + 1),
                error_code: None,
                error_message: None,
                cleanup_after_at: None,
                files: Vec::new(),
            },
        )
        .await
        .expect("insert session");
    }

    let page_1 = list_history(
        &conn,
        &TransferHistoryFilterDto {
            cursor: None,
            limit: Some(2),
            status: None,
            peer_device_id: None,
        },
    )
    .await
    .expect("list history page 1");
    assert_eq!(page_1.items.len(), 2);
    let cursor = page_1.next_cursor.clone().expect("cursor should exist");

    let page_2 = list_history(
        &conn,
        &TransferHistoryFilterDto {
            cursor: Some(cursor),
            limit: Some(2),
            status: None,
            peer_device_id: None,
        },
    )
    .await
    .expect("list history page 2");
    assert_eq!(page_2.items.len(), 1);
    assert_eq!(page_2.items[0].id, "session-a");

    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn cleanup_expired_should_cascade_delete_transfer_files() {
    let (conn, path) = setup_temp_db("cleanup-expired-cascade").await;
    let now = 2_000_i64;
    let cleanup_after_at = now - 1;
    let session_id = "expired-session";
    insert_session(
        &conn,
        &TransferSessionDto {
            id: session_id.to_string(),
            direction: TransferDirection::Send,
            peer_device_id: "peer".to_string(),
            peer_name: "Peer".to_string(),
            status: TransferStatus::Success,
            total_bytes: 8,
            transferred_bytes: 8,
            avg_speed_bps: 1,
            rtt_ms_p50: None,
            rtt_ms_p95: None,
            save_dir: "/tmp".to_string(),
            created_at: 1,
            started_at: Some(1),
            finished_at: Some(2),
            error_code: None,
            error_message: None,
            cleanup_after_at: Some(cleanup_after_at),
            files: Vec::new(),
        },
    )
    .await
    .expect("insert session");

    insert_or_update_file(
        &conn,
        &TransferFileDto {
            id: "expired-file".to_string(),
            session_id: session_id.to_string(),
            relative_path: "expired.txt".to_string(),
            source_path: Some("/tmp/expired.txt".to_string()),
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
    .await
    .expect("insert file");

    cleanup_expired(&conn, now).await.expect("cleanup expired");
    assert_eq!(count_transfer_sessions(&conn).await, 0);
    assert_eq!(count_transfer_files(&conn).await, 0);

    let _ = std::fs::remove_file(path);
}

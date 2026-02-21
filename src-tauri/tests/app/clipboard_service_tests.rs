use super::*;
use crate::infrastructure::db;

fn unique_temp_db_path(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_millis();
    std::env::temp_dir().join(format!("rtool-{prefix}-{}-{now}.db", std::process::id()))
}

#[test]
fn should_load_default_clipboard_settings() {
    let db_path = unique_temp_db_path("clipboard-settings-default");
    db::init_db(db_path.as_path()).expect("init db");
    let db_pool = db::new_db_pool(db_path.as_path()).expect("new db pool");
    let service = ClipboardService::new(db_pool, db_path.clone()).expect("new clipboard service");

    let settings = service.get_settings();
    assert_eq!(settings.max_items, CLIPBOARD_MAX_ITEMS_DEFAULT);
    assert!(settings.size_cleanup_enabled);
    assert_eq!(
        settings.max_total_size_mb,
        CLIPBOARD_MAX_TOTAL_SIZE_MB_DEFAULT
    );

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn should_allow_when_disk_space_metric_is_missing() {
    let result = ensure_available_space(None, CLIPBOARD_MIN_FREE_DISK_BYTES);
    assert!(result.is_ok());
}

#[test]
fn should_allow_when_disk_space_is_enough() {
    let result = ensure_available_space(
        Some(CLIPBOARD_MIN_FREE_DISK_BYTES),
        CLIPBOARD_MIN_FREE_DISK_BYTES,
    );
    assert!(result.is_ok());
}

#[test]
fn should_reject_when_disk_space_is_low() {
    let result = ensure_available_space(
        Some(CLIPBOARD_MIN_FREE_DISK_BYTES - 1),
        CLIPBOARD_MIN_FREE_DISK_BYTES,
    );
    assert!(result.is_err());
    assert_eq!(
        result.expect_err("expected low disk error").code,
        "clipboard_disk_space_low"
    );
}

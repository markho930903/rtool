use super::*;
use std::io::Write;

fn unique_temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!("rtool-log-test-{}", now_millis()))
}

#[test]
fn should_cleanup_only_expired_logs() {
    let log_dir = unique_temp_dir();
    fs::create_dir_all(&log_dir).expect("failed to create temp log dir");

    let old_file = log_dir.join("old.log");
    let mut old_writer = fs::File::create(&old_file).expect("failed to create old log");
    writeln!(old_writer, "old").expect("failed to write old log");
    std::thread::sleep(Duration::from_millis(40));

    let new_file = log_dir.join("new.log");
    let mut new_writer = fs::File::create(&new_file).expect("failed to create new log");
    writeln!(new_writer, "new").expect("failed to write new log");

    let removed = cleanup_expired_logs_with_duration(
        &log_dir,
        Duration::from_millis(20),
        SystemTime::now(),
    )
    .expect("cleanup failed");

    assert_eq!(removed, 1);
    assert!(!old_file.exists());
    assert!(new_file.exists());

    let _ = fs::remove_file(new_file);
    let _ = fs::remove_dir_all(log_dir);
}

#[test]
fn should_sanitize_sensitive_json_fields() {
    let payload = serde_json::json!({
        "clipboardText": "super-secret-content",
        "filePath": "/Users/demo/Desktop/report.txt",
        "hostName": "my-macbook",
        "dataUrl": "data:image/png;base64,AAAA"
    });

    let sanitized = sanitize_json_value(&payload);
    let text = sanitized
        .get("clipboardText")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let path = sanitized
        .get("filePath")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let host = sanitized
        .get("hostName")
        .and_then(|value| value.as_str())
        .unwrap_or_default();

    assert!(text.starts_with("[redacted-text"));
    assert!(path.starts_with("[path:report.txt"));
    assert!(host.starts_with("[host hash="));
}

#[test]
fn should_sanitize_data_url() {
    let result = sanitize_for_log("data:image/png;base64,AAAAA");
    assert!(result.starts_with("[data-url redacted"));
}

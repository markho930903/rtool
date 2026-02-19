use super::*;

#[test]
fn should_normalize_file_uri_for_compare() {
    let value = normalize_path_for_compare("file:///tmp/example.txt");
    assert_eq!(value, "/tmp/example.txt");
}

#[test]
fn should_return_verify_failed_when_paths_mismatch() {
    let expected = vec!["/tmp/a.txt".to_string()];
    let actual = vec!["/tmp/b.txt".to_string()];
    let result = ensure_expected_and_actual_file_paths(&expected, &actual);
    assert!(result.is_err());
    assert_eq!(
        result.expect_err("expected mismatch error").code,
        "clipboard_set_files_verify_failed"
    );
}

#[test]
fn should_build_macos_script_with_escaped_path() {
    let file_paths = vec![
        "/tmp/space file.txt".to_string(),
        "/tmp/中文\"test\".txt".to_string(),
    ];
    let script = build_macos_copy_files_script(&file_paths);
    assert!(script.starts_with("set the clipboard to {"));
    assert!(script.contains("POSIX file \"/tmp/space file.txt\""));
    assert!(script.contains("POSIX file \"/tmp/中文\\\"test\\\".txt\""));
}

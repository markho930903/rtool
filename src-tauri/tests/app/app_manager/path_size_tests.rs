use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("rtool-{prefix}-{unique}"));
    fs::create_dir_all(path.as_path()).expect("temp dir should be created");
    path
}

#[test]
fn exact_path_size_counts_deep_nested_files() {
    let root = make_temp_dir("size-exact");
    let deep = root.join("a").join("b").join("c").join("d").join("e");
    fs::create_dir_all(deep.as_path()).expect("deep dir should be created");
    fs::write(root.join("top.bin"), vec![0u8; 7]).expect("top file should be written");
    fs::write(deep.join("deep.bin"), vec![0u8; 13]).expect("deep file should be written");

    let exact = exact_path_size_bytes(root.as_path()).expect("exact size should be computed");
    let estimated =
        try_get_path_size_bytes(root.as_path()).expect("estimated size should be computed");
    assert_eq!(exact, 20);
    assert!(estimated <= exact);

    fs::remove_dir_all(root).expect("temp dir should be removed");
}

#[test]
fn resolve_app_size_path_uses_parent_for_single_file() {
    let root = make_temp_dir("size-root");
    let file_path = root.join("binary");
    fs::write(file_path.as_path(), vec![0u8; 4]).expect("file should be written");

    let resolved = resolve_app_size_path(file_path.as_path());
    assert_eq!(resolved, root);

    fs::remove_dir_all(resolved).expect("temp dir should be removed");
}

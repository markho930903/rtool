use super::*;

use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rtool-{prefix}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[test]
fn should_scan_directory_and_file_entries() {
    let root = create_temp_dir("launcher-index-scan");
    let nested_dir = root.join("nested");
    let nested_file = root.join("note.txt");
    fs::create_dir_all(&nested_dir).expect("create nested dir");
    fs::write(&nested_file, "hello launcher index").expect("write nested file");

    let entries = scan_index_root(root.as_path(), 4, 100, root.to_string_lossy().as_ref());
    let nested_dir_text = nested_dir.to_string_lossy();
    let nested_file_text = nested_file.to_string_lossy();
    assert!(entries.iter().any(|entry| {
        entry.kind == IndexedEntryKind::Directory && entry.path == nested_dir_text
    }));
    assert!(
        entries
            .iter()
            .any(|entry| entry.kind == IndexedEntryKind::File && entry.path == nested_file_text)
    );

    fs::remove_dir_all(&root).expect("cleanup temp dir");
}

#[test]
fn should_build_file_entry_with_extension() {
    let root = create_temp_dir("launcher-index-entry");
    let file_path = root.join("archive.tar.gz");
    fs::write(&file_path, "payload").expect("write temp file");

    let entry = build_index_entry(
        file_path.as_path(),
        IndexedEntryKind::File,
        root.to_string_lossy().as_ref(),
    )
    .expect("build entry");
    assert_eq!(entry.kind, IndexedEntryKind::File);
    assert_eq!(entry.ext.as_deref(), Some("gz"));
    assert_eq!(entry.name, "archive.tar.gz");
    assert_eq!(entry.source_root, root.to_string_lossy().to_string());

    fs::remove_dir_all(&root).expect("cleanup temp dir");
}

#[test]
fn should_escape_sql_like_pattern_meta_chars() {
    let escaped = escape_like_pattern(r#"100%_done\ok"#);
    assert_eq!(escaped, r#"100\%\_done\\ok"#);
}

#[test]
fn should_parse_index_entry_kind_from_db_value() {
    assert_eq!(
        IndexedEntryKind::from_db("directory"),
        Some(IndexedEntryKind::Directory)
    );
    assert_eq!(
        IndexedEntryKind::from_db("FILE"),
        Some(IndexedEntryKind::File)
    );
    assert_eq!(IndexedEntryKind::from_db("unknown"), None);
}

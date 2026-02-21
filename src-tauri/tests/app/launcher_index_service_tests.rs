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

#[test]
fn legacy_default_full_disk_settings_should_migrate() {
    let legacy = LauncherSearchSettingsRecord::default().normalize();
    assert!(is_legacy_default_full_disk_profile(&legacy));
    assert_eq!(legacy.max_items_per_root, DEFAULT_MAX_ITEMS_PER_ROOT);

    let migrated = migrate_legacy_default_full_disk_profile(legacy.clone());
    assert_eq!(migrated.max_items_per_root, legacy.max_total_items);
    assert_eq!(migrated.max_total_items, legacy.max_total_items);
    assert_eq!(migrated.roots, legacy.roots);
    assert_eq!(migrated.exclude_patterns, legacy.exclude_patterns);

    let custom = LauncherSearchSettingsRecord {
        max_items_per_root: DEFAULT_MAX_ITEMS_PER_ROOT + 1,
        ..legacy.clone()
    };
    let custom_migrated = migrate_legacy_default_full_disk_profile(custom.clone());
    assert_eq!(custom_migrated, custom);
}

#[test]
fn single_root_full_disk_uses_effective_total_budget() {
    let settings = migrate_legacy_default_full_disk_profile(
        LauncherSearchSettingsRecord::default().normalize(),
    );
    let remaining_total = 12_345usize;

    let effective = resolve_effective_max_items_per_root(
        settings.max_items_per_root as usize,
        remaining_total,
        is_default_full_disk_profile(&settings),
    );

    assert_eq!(effective, remaining_total);
}

#[test]
fn scan_order_should_be_deterministic() {
    let root = create_temp_dir("launcher-index-order");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha");
    fs::create_dir_all(&beta).expect("create beta");
    fs::write(alpha.join("z-last.txt"), "a").expect("write alpha file");
    fs::write(alpha.join("a-first.txt"), "a").expect("write alpha file");
    fs::write(beta.join("middle.txt"), "b").expect("write beta file");
    fs::write(root.join("root-file.txt"), "root").expect("write root file");

    let first = scan_index_root(root.as_path(), 6, 1000, root.to_string_lossy().as_ref())
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    let second = scan_index_root(root.as_path(), 6, 1000, root.to_string_lossy().as_ref())
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();

    assert_eq!(first, second);
    fs::remove_dir_all(&root).expect("cleanup temp dir");
}

#[test]
fn top_level_priority_should_prefer_home_and_apps() {
    let home = "/users/tester";
    assert_eq!(scan_priority_for_path("/users", true, Some(home)), 0);
    assert_eq!(scan_priority_for_path("/applications", true, Some(home)), 1);
    assert_eq!(scan_priority_for_path("/system", true, Some(home)), 2);
    assert_eq!(scan_priority_for_path("/opt", true, Some(home)), 3);
    assert_eq!(scan_priority_for_path("/users", false, Some(home)), 3);
}

#[test]
fn truncation_log_classification_should_be_expected_for_default_profile() {
    let legacy = LauncherSearchSettingsRecord::default().normalize();
    assert_eq!(
        classify_truncation_log_level(&legacy),
        TruncationLogLevel::Info
    );

    let migrated = migrate_legacy_default_full_disk_profile(legacy.clone());
    assert_eq!(
        classify_truncation_log_level(&migrated),
        TruncationLogLevel::Info
    );

    let custom = LauncherSearchSettingsRecord {
        roots: vec!["/tmp".to_string()],
        ..migrated
    };
    assert_eq!(
        classify_truncation_log_level(&custom),
        TruncationLogLevel::Warn
    );
}

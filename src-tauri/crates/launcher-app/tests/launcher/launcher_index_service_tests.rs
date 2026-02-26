use super::*;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use app_infra::db::{DbConn, get_app_setting, init_db, open_db, set_app_setting};
use uuid::Uuid;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rtool-{prefix}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn unique_temp_db_path(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_millis();
    std::env::temp_dir().join(format!("rtool-{prefix}-{}-{now}.db", std::process::id()))
}

async fn setup_temp_db(prefix: &str) -> (DbConn, PathBuf) {
    let path = unique_temp_db_path(prefix);
    let conn = open_db(path.as_path()).await.expect("open db");
    init_db(&conn).await.expect("init db");
    (conn, path)
}

fn normalized_path(path: &Path) -> String {
    normalize_path_for_match(path)
}

fn normalized_paths(candidates: &[PathBuf]) -> Vec<String> {
    candidates
        .iter()
        .map(|candidate| normalized_path(candidate.as_path()))
        .collect()
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
        None,
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
fn single_root_full_disk_uses_effective_total_budget() {
    let settings = LauncherSearchSettingsRecord {
        roots: vec!["/".to_string()],
        ..LauncherSearchSettingsRecord::default().normalize()
    };
    let remaining_total = 12_345usize;

    let effective = resolve_effective_max_items_per_root(
        settings.max_items_per_root as usize,
        remaining_total,
        has_single_system_root_scope(&settings),
    );

    assert_eq!(effective, remaining_total);
}

#[test]
fn default_root_candidates_should_cover_all_platform_profiles() {
    let home = PathBuf::from("/Users/tester");
    let app_data = PathBuf::from("C:/Users/tester/AppData/Roaming");
    let program_data = PathBuf::from("C:/ProgramData");
    let local_app_data = PathBuf::from("C:/Users/tester/AppData/Local");

    let macos = build_default_search_root_candidates(
        ScopePlatform::Macos,
        Some(home.as_path()),
        None,
        None,
        None,
    );
    assert_eq!(
        normalized_paths(&macos),
        vec![
            normalized_path(home.join("Applications").as_path()),
            normalized_path(Path::new("/Applications")),
            normalized_path(home.join("Desktop").as_path()),
            normalized_path(home.join("Documents").as_path()),
            normalized_path(home.join("Downloads").as_path()),
        ]
    );

    let windows = build_default_search_root_candidates(
        ScopePlatform::Windows,
        Some(Path::new("C:/Users/tester")),
        Some(app_data.as_path()),
        Some(program_data.as_path()),
        Some(local_app_data.as_path()),
    );
    assert_eq!(
        normalized_paths(&windows),
        vec![
            normalized_path(
                app_data
                    .join("Microsoft/Windows/Start Menu/Programs")
                    .as_path()
            ),
            normalized_path(
                program_data
                    .join("Microsoft/Windows/Start Menu/Programs")
                    .as_path()
            ),
            normalized_path(Path::new("C:/Users/tester/Desktop")),
            normalized_path(Path::new("C:/Users/tester/Documents")),
            normalized_path(Path::new("C:/Users/tester/Downloads")),
            normalized_path(local_app_data.join("Programs").as_path()),
        ]
    );

    let linux = build_default_search_root_candidates(
        ScopePlatform::Linux,
        Some(home.as_path()),
        None,
        None,
        None,
    );
    assert_eq!(
        normalized_paths(&linux),
        vec![
            normalized_path(home.join(".local/share/applications").as_path()),
            normalized_path(Path::new("/usr/share/applications")),
            normalized_path(Path::new("/usr/local/share/applications")),
            normalized_path(home.join("Desktop").as_path()),
            normalized_path(home.join("Documents").as_path()),
            normalized_path(home.join("Downloads").as_path()),
        ]
    );
}

#[tokio::test]
async fn load_or_init_settings_should_preserve_existing_roots_and_update_scope_policy_version() {
    let (db_conn, db_path) = setup_temp_db("launcher-scope-policy").await;

    let custom = LauncherSearchSettingsRecord {
        roots: vec!["/tmp/custom-root".to_string()],
        exclude_patterns: vec!["node_modules".to_string()],
        max_scan_depth: 6,
        max_items_per_root: 9_999,
        max_total_items: 18_888,
        refresh_interval_secs: 777,
    }
    .normalize();
    let serialized = serde_json::to_string(&custom).expect("serialize settings");
    set_app_setting(&db_conn, SEARCH_SETTINGS_KEY, serialized.as_str())
        .await
        .expect("seed settings");
    set_app_setting(&db_conn, LAUNCHER_SCOPE_POLICY_VERSION_KEY, "1")
        .await
        .expect("seed scope state");

    let loaded = load_or_init_settings(&db_conn)
        .await
        .expect("load settings");
    assert_eq!(loaded, custom);

    let stored_state = get_app_setting(&db_conn, LAUNCHER_SCOPE_POLICY_VERSION_KEY)
        .await
        .expect("read scope state");
    assert_eq!(
        stored_state.as_deref(),
        Some(LAUNCHER_SCOPE_POLICY_VERSION_VALUE)
    );

    let second = load_or_init_settings(&db_conn).await.expect("second read");
    assert_eq!(second, loaded);

    let _ = fs::remove_file(db_path);
}

#[tokio::test]
async fn reset_search_settings_should_restore_default_profile_and_version() {
    let (db_conn, db_path) = setup_temp_db("launcher-reset-settings").await;

    let custom = LauncherSearchSettingsRecord {
        roots: vec!["/tmp/custom-root".to_string()],
        exclude_patterns: vec!["node_modules".to_string()],
        max_scan_depth: 8,
        max_items_per_root: 6_000,
        max_total_items: 12_000,
        refresh_interval_secs: 300,
    }
    .normalize();
    save_settings(&db_conn, &custom)
        .await
        .expect("seed settings");
    set_app_setting(&db_conn, LAUNCHER_SCOPE_POLICY_VERSION_KEY, "1")
        .await
        .expect("seed scope policy");

    let reset = reset_search_settings_async(&db_conn)
        .await
        .expect("reset settings");
    let expected = LauncherSearchSettingsRecord::default().normalize();
    assert_eq!(reset.roots, expected.roots);
    assert_eq!(reset.exclude_patterns, expected.exclude_patterns);
    assert_eq!(reset.max_scan_depth, expected.max_scan_depth);
    assert_eq!(reset.max_items_per_root, expected.max_items_per_root);
    assert_eq!(reset.max_total_items, expected.max_total_items);
    assert_eq!(reset.refresh_interval_secs, expected.refresh_interval_secs);

    let stored_state = get_app_setting(&db_conn, LAUNCHER_SCOPE_POLICY_VERSION_KEY)
        .await
        .expect("read scope policy");
    assert_eq!(
        stored_state.as_deref(),
        Some(LAUNCHER_SCOPE_POLICY_VERSION_VALUE)
    );

    let _ = fs::remove_file(db_path);
}

#[test]
fn scan_warning_aggregator_should_limit_sample_paths() {
    let mut aggregator = ScanWarningAggregator::default();
    for index in 0..10 {
        let path = PathBuf::from(format!("/tmp/sample-{index}"));
        aggregator.record(ScanWarningKind::ReadDir, path.as_path());
    }

    assert_eq!(aggregator.read_dir_failed, 10);
    assert_eq!(aggregator.read_dir_samples.len(), SCAN_WARNING_SAMPLE_LIMIT);
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

    let custom = LauncherSearchSettingsRecord {
        roots: vec!["/tmp".to_string()],
        ..legacy
    };
    assert_eq!(
        classify_truncation_log_level(&custom),
        TruncationLogLevel::Warn
    );
}

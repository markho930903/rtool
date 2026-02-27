use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn candidate_with_confidence(
    confidence: AppManagerResidueConfidence,
    evidence_len: usize,
) -> ResidueCandidate {
    ResidueCandidate {
        path: PathBuf::from("/tmp/a"),
        scope: AppManagerScope::User,
        kind: AppManagerResidueKind::Cache,
        exists: true,
        filesystem: true,
        match_reason: AppManagerResidueMatchReason::RelatedRoot,
        confidence,
        evidence: (0..evidence_len).map(|idx| format!("e{idx}")).collect(),
        risk_level: AppManagerRiskLevel::Low,
        recommended: true,
        readonly_reason_code: None,
    }
}

#[test]
fn residue_candidate_replace_prefers_higher_confidence() {
    let current = candidate_with_confidence(AppManagerResidueConfidence::High, 1);
    let next = candidate_with_confidence(AppManagerResidueConfidence::Exact, 1);
    assert!(should_replace_residue_candidate(&current, &next));
    assert!(!should_replace_residue_candidate(&next, &current));
}

#[test]
fn residue_candidate_replace_prefers_more_evidence_when_confidence_equal() {
    let current = candidate_with_confidence(AppManagerResidueConfidence::High, 1);
    let next = candidate_with_confidence(AppManagerResidueConfidence::High, 2);
    assert!(should_replace_residue_candidate(&current, &next));
}

#[test]
fn append_scan_size_warnings_keeps_structured_fields_and_deduplicates() {
    let mut warnings = Vec::new();
    let mut warning_keys = HashSet::new();

    let warning = PathSizeWarning {
        code: AppManagerScanWarningCode::AppManagerSizeReadDirFailed,
        path: "/tmp/example".to_string(),
        detail_code: AppManagerScanWarningDetailCode::PermissionDenied,
    };

    append_scan_size_warnings(&mut warnings, &mut warning_keys, vec![warning.clone()]);
    append_scan_size_warnings(&mut warnings, &mut warning_keys, vec![warning]);

    assert_eq!(warnings.len(), 1);
    let first = &warnings[0];
    assert_eq!(
        first.code,
        AppManagerScanWarningCode::AppManagerSizeReadDirFailed
    );
    assert_eq!(first.path.as_deref(), Some("/tmp/example"));
    assert_eq!(
        first.detail_code,
        Some(AppManagerScanWarningDetailCode::PermissionDenied)
    );
}

#[test]
fn append_scan_size_warnings_keeps_distinct_detail_codes() {
    let mut warnings = Vec::new();
    let mut warning_keys = HashSet::new();

    append_scan_size_warnings(
        &mut warnings,
        &mut warning_keys,
        vec![
            PathSizeWarning {
                code: AppManagerScanWarningCode::AppManagerSizeReadDirFailed,
                path: "/tmp/example".to_string(),
                detail_code: AppManagerScanWarningDetailCode::PermissionDenied,
            },
            PathSizeWarning {
                code: AppManagerScanWarningCode::AppManagerSizeReadDirFailed,
                path: "/tmp/example".to_string(),
                detail_code: AppManagerScanWarningDetailCode::TimedOut,
            },
        ],
    );

    assert_eq!(warnings.len(), 2);
}

fn unique_temp_path(prefix: &str, leaf: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "rtool-{prefix}-{}-{stamp}-{leaf}",
        std::process::id()
    ))
}

#[test]
fn detect_path_type_existing_directory_with_dots_is_directory() {
    let dir = unique_temp_path("dir", "complex.name.data.dir");
    fs::create_dir_all(&dir).expect("create temp directory for path type test");

    let result = detect_path_type(dir.as_path(), AppManagerResidueKind::SavedState, true);

    assert_eq!(result, AppManagerPathType::Directory);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn detect_path_type_existing_file_is_file() {
    let file = unique_temp_path("file", "example.config.json");
    fs::write(&file, b"{}").expect("create temp file for path type test");

    let result = detect_path_type(file.as_path(), AppManagerResidueKind::Preferences, true);

    assert_eq!(result, AppManagerPathType::File);
    let _ = fs::remove_file(&file);
}

#[test]
fn detect_path_type_missing_saved_state_with_dots_stays_directory() {
    let path = unique_temp_path("missing-dir", "session.v1.savedstate");
    let result = detect_path_type(path.as_path(), AppManagerResidueKind::SavedState, true);
    assert_eq!(result, AppManagerPathType::Directory);
}

#[test]
fn detect_path_type_missing_preferences_plist_is_file() {
    let path = unique_temp_path("missing-file", "com.example.app.plist");
    let result = detect_path_type(path.as_path(), AppManagerResidueKind::Preferences, true);
    assert_eq!(result, AppManagerPathType::File);
}

#[test]
fn detect_path_type_registry_key_non_filesystem_is_directory() {
    let path = PathBuf::from(r"HKCU\Software\Example");
    let result = detect_path_type(path.as_path(), AppManagerResidueKind::RegistryKey, false);
    assert_eq!(result, AppManagerPathType::Directory);
}

#[test]
fn detect_path_type_registry_value_non_filesystem_is_file() {
    let path = PathBuf::from(r"HKCU\Software\Example::RunAtLogin");
    let result = detect_path_type(path.as_path(), AppManagerResidueKind::RegistryValue, false);
    assert_eq!(result, AppManagerPathType::File);
}

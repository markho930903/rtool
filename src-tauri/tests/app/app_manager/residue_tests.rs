use super::*;

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

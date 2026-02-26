use super::*;

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}-{}-{now}", std::process::id()))
}

#[test]
fn discovery_pattern_matching_contract() {
    assert_eq!(
        match_pattern("com.tencent.xinwechat", "com.tencent.xinwechat"),
        Some(DiscoveryPatternMatchKind::Exact)
    );
    assert_eq!(
        match_pattern(
            "com.tencent.xinwechat.wechatfileproviderextension",
            "com.tencent.xinwechat"
        ),
        Some(DiscoveryPatternMatchKind::PrefixOrSuffix)
    );
    assert_eq!(
        match_pattern(
            "5a4re8sf68.com.tencent.xinwechat",
            "com.tencent.xinwechat"
        ),
        Some(DiscoveryPatternMatchKind::PrefixOrSuffix)
    );
    assert_eq!(
        match_pattern("short", "com.tencent.xinwechat"),
        None
    );
}

#[test]
fn discovery_token_overlap_contract() {
    let name_tokens = split_discovery_tokens("wechat.tencent.cache");
    let token_aliases = vec![
        "wechat".to_string(),
        "tencent".to_string(),
        "xinwechat".to_string(),
    ];
    let overlap = token_overlap_score(name_tokens.as_slice(), token_aliases.as_slice());
    assert!(overlap >= 2);
}

#[test]
fn discovery_finds_exact_contains_and_token_matches() {
    let root = unique_temp_dir("rtool-discovery-match");
    fs::create_dir_all(root.join("com.tencent.xinWeChat")).expect("create exact dir");
    fs::create_dir_all(root.join("5A4RE8SF68.com.tencent.xinWeChat"))
        .expect("create team dir");
    fs::create_dir_all(root.join("wechat.tencent.cache")).expect("create token dir");

    let identifiers = vec![
        ResidueIdentifier {
            value: "com.tencent.xinWeChat".to_string(),
            match_reason: AppManagerResidueMatchReason::BundleId,
        },
        ResidueIdentifier {
            value: "5A4RE8SF68.com.tencent.xinWeChat".to_string(),
            match_reason: AppManagerResidueMatchReason::EntitlementGroup,
        },
    ];
    let token_aliases = vec![
        "wechat".to_string(),
        "tencent".to_string(),
        "xinwechat".to_string(),
    ];

    let result = discover_from_root_for_test(
        root.clone(),
        AppManagerScope::User,
        AppManagerResidueKind::AppScript,
        identifiers.as_slice(),
        token_aliases.as_slice(),
    );
    let paths = result
        .candidates
        .iter()
        .map(|candidate| candidate.path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    assert!(
        paths.iter().any(|path| path.ends_with("com.tencent.xinWeChat")),
        "should match exact bundle id directory"
    );
    assert!(
        paths.iter().any(|path| path.ends_with("5A4RE8SF68.com.tencent.xinWeChat")),
        "should match entitlement group directory"
    );
    assert!(
        result
            .candidates
            .iter()
            .any(|candidate| candidate.match_reason == AppManagerResidueMatchReason::KeywordToken),
        "should include token overlap match"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn discovery_truncates_large_roots_with_warning() {
    let root = unique_temp_dir("rtool-discovery-limit");
    fs::create_dir_all(root.as_path()).expect("create root");
    for idx in 0..3_010u32 {
        let path = root.join(format!("entry-{idx:04}"));
        fs::create_dir_all(path).expect("create child");
    }

    let result = discover_from_root_for_test(
        root.clone(),
        AppManagerScope::User,
        AppManagerResidueKind::AppSupport,
        &[],
        &[],
    );
    assert!(
        result.warnings.iter().any(|warning| {
            warning.code == AppManagerScanWarningCode::AppManagerSizeEstimateTruncated
                && warning.detail_code == Some(AppManagerScanWarningDetailCode::LimitReached)
        }),
        "should emit truncation warning when root entry count exceeds 3000"
    );

    let _ = fs::remove_dir_all(root);
}

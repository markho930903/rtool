use super::*;

fn test_app(bundle_or_app_id: Option<&str>) -> ManagedAppDto {
    ManagedAppDto {
        id: "mac.test.app".to_string(),
        name: "AppCleaner.app".to_string(),
        path: "/Applications/AppCleaner.app".to_string(),
        bundle_or_app_id: bundle_or_app_id.map(ToString::to_string),
        version: None,
        publisher: None,
        platform: AppManagerPlatform::Macos,
        source: AppManagerSource::Application,
        icon_kind: AppManagerIconKind::Iconify,
        icon_value: "i-noto:desktop-computer".to_string(),
        size_bytes: None,
        size_accuracy: AppManagerSizeAccuracy::Estimated,
        size_computed_at: None,
        startup_enabled: false,
        startup_scope: AppManagerStartupScope::None,
        startup_editable: true,
        readonly_reason_code: None,
        uninstall_supported: true,
        uninstall_kind: Some(AppManagerUninstallKind::FinderTrash),
        capabilities: build_app_capabilities(true, true, true),
        identity: build_app_identity(
            bundle_or_app_id.unwrap_or("net.freemacsoft.AppCleaner"),
            vec![
                "AppCleaner".to_string(),
                "net.freemacsoft.AppCleaner".to_string(),
            ],
            AppManagerIdentitySource::BundleId,
        ),
        risk_level: AppManagerRiskLevel::Low,
        fingerprint: "fp".to_string(),
    }
}

fn has_root_path(roots: &[RelatedRootSpec], expected: &Path) -> bool {
    let expected_key = normalize_path_key(expected.to_string_lossy().as_ref());
    roots
        .iter()
        .any(|root| normalize_path_key(root.path.to_string_lossy().as_ref()) == expected_key)
}

fn has_candidate_path(candidates: &[ResidueCandidate], expected: &Path) -> bool {
    let expected_key = normalize_path_key(expected.to_string_lossy().as_ref());
    candidates.iter().any(|candidate| {
        normalize_path_key(candidate.path.to_string_lossy().as_ref()) == expected_key
    })
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}-{}-{now}", std::process::id()))
}

#[test]
fn collect_related_root_specs_includes_http_storages_bundle_path() {
    let app = test_app(Some("net.freemacsoft.AppCleaner"));
    let roots = collect_related_root_specs(&app);
    let home = home_dir().expect("home dir should exist for mac tests");
    let expected = home
        .join("Library")
        .join("HTTPStorages")
        .join("net.freemacsoft.AppCleaner");
    assert!(
        has_root_path(roots.as_slice(), expected.as_path()),
        "expected HTTPStorages path {} to be included",
        expected.to_string_lossy()
    );
}

#[test]
fn collect_related_root_specs_includes_temp_cache_alias_paths() {
    let app = test_app(Some("net.freemacsoft.AppCleaner"));
    let roots = collect_related_root_specs(&app);
    let expected_paths = mac_collect_temp_alias_roots("net.freemacsoft.AppCleaner");
    assert!(!expected_paths.is_empty(), "expected temp candidate paths");
    for expected in expected_paths {
        assert!(
            has_root_path(roots.as_slice(), expected.as_path()),
            "expected temp cache path {} to be included",
            expected.to_string_lossy()
        );
    }
}

#[test]
fn resolve_app_size_path_promotes_bundle_root_on_macos() {
    let executable = Path::new("/Applications/TestApp.app/Contents/MacOS/TestApp");
    let resolved = resolve_app_size_path(executable);
    assert_eq!(resolved, PathBuf::from("/Applications/TestApp.app"));
}

#[test]
fn identity_profile_extracts_extension_bundle_ids() {
    let root = unique_temp_dir("rtool-macos-profile");
    let app_path = root.join("WeChat.app");
    let plugin_root = app_path
        .join("Contents")
        .join("PlugIns")
        .join("WeChatFileProviderExtension.appex")
        .join("Contents");
    fs::create_dir_all(plugin_root.as_path()).expect("create plugin root");
    fs::create_dir_all(app_path.join("Contents")).expect("create app content");

    fs::write(
        app_path.join("Contents").join("Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.tencent.xinWeChat</string>
</dict></plist>"#,
    )
    .expect("write app info plist");

    fs::write(
        plugin_root.join("Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.tencent.xinWeChat.WeChatFileProviderExtension</string>
</dict></plist>"#,
    )
    .expect("write extension plist");

    let mut app = test_app(Some("com.tencent.xinWeChat"));
    app.path = app_path.to_string_lossy().to_string();
    let profile = build_residue_identity_profile(&app);
    assert!(
        profile
            .extension_bundle_ids
            .iter()
            .any(|id| id == "com.tencent.xinWeChat.WeChatFileProviderExtension")
    );
    assert!(profile.has_file_provider_extension);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn identity_profile_extracts_entitlement_group_and_team() {
    let mut profile = ResidueIdentityProfile::default();
    apply_entitlements_from_text(
        &mut profile,
        r#"
<plist version="1.0"><dict>
<key>com.apple.security.application-groups</key>
<array>
  <string>5A4RE8SF68.com.tencent.xinWeChat</string>
</array>
<key>com.apple.developer.team-identifier</key>
<string>5A4RE8SF68</string>
</dict></plist>
"#,
    );
    assert!(
        profile
            .app_group_ids
            .iter()
            .any(|value| value == "5A4RE8SF68.com.tencent.xinWeChat")
    );
    assert!(profile.team_ids.iter().any(|value| value == "5A4RE8SF68"));
}

#[test]
fn quick_templates_include_scripts_containers_and_group_containers() {
    let app = test_app(Some("com.tencent.xinWeChat"));
    let profile = build_residue_identity_profile(&app);
    let candidates = collect_quick_residue_candidates(&app, &profile);
    let home = home_dir().expect("home dir should exist");
    assert!(has_candidate_path(
        candidates.as_slice(),
        home.join("Library/Application Scripts/com.tencent.xinWeChat")
            .as_path()
    ));
    assert!(has_candidate_path(
        candidates.as_slice(),
        home.join("Library/Containers/com.tencent.xinWeChat")
            .as_path()
    ));
    assert!(has_candidate_path(
        candidates.as_slice(),
        home.join("Library/Group Containers/com.tencent.xinWeChat")
            .as_path()
    ));
}

#[test]
fn discovery_matches_team_prefixed_group_container() {
    let root = unique_temp_dir("rtool-macos-discovery");
    let team_dir = root.join("5A4RE8SF68.com.tencent.xinWeChat");
    fs::create_dir_all(team_dir.as_path()).expect("create team dir");

    let profile = ResidueIdentityProfile {
        app_group_ids: vec!["5A4RE8SF68.com.tencent.xinWeChat".to_string()],
        ..ResidueIdentityProfile::default()
    };
    let identifiers = profile_identifiers(&profile);
    let result = discover_from_root_for_test(
        root.clone(),
        AppManagerScope::User,
        AppManagerResidueKind::GroupContainer,
        identifiers.as_slice(),
        profile.token_aliases.as_slice(),
    );
    assert!(
        result.candidates.iter().any(|candidate| {
            candidate
                .path
                .to_string_lossy()
                .ends_with("5A4RE8SF68.com.tencent.xinWeChat")
        }),
        "deep discovery should include team-prefixed group container path"
    );

    let _ = fs::remove_dir_all(root);
}

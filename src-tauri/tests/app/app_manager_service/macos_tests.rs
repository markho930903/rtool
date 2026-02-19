use super::*;

fn test_app(bundle_or_app_id: Option<&str>) -> ManagedAppDto {
    ManagedAppDto {
        id: "mac.test.app".to_string(),
        name: "AppCleaner.app".to_string(),
        path: "/Applications/AppCleaner.app".to_string(),
        bundle_or_app_id: bundle_or_app_id.map(ToString::to_string),
        version: None,
        publisher: None,
        platform: "macos".to_string(),
        source: "application".to_string(),
        icon_kind: "iconify".to_string(),
        icon_value: "i-noto:desktop-computer".to_string(),
        estimated_size_bytes: None,
        startup_enabled: false,
        startup_scope: "none".to_string(),
        startup_editable: true,
        readonly_reason_code: None,
        uninstall_supported: true,
        uninstall_kind: None,
        capabilities: build_app_capabilities(true, true, true),
        identity: build_app_identity(
            bundle_or_app_id.unwrap_or("net.freemacsoft.AppCleaner"),
            vec![
                "AppCleaner".to_string(),
                "net.freemacsoft.AppCleaner".to_string(),
            ],
            "bundle_id",
        ),
        risk_level: "low".to_string(),
        fingerprint: "fp".to_string(),
    }
}

fn has_root_path(roots: &[RelatedRootSpec], expected: &Path) -> bool {
    let expected_key = normalize_path_key(expected.to_string_lossy().as_ref());
    roots
        .iter()
        .any(|root| normalize_path_key(root.path.to_string_lossy().as_ref()) == expected_key)
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

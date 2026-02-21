use super::*;
use crate::core::models::{
    AppManagerActionCode, AppManagerCategory, AppManagerCleanupReasonCode,
    AppManagerScanWarningDetailCode,
};

fn test_app(source: AppManagerSource, startup_enabled: bool) -> ManagedAppDto {
    ManagedAppDto {
        id: format!("test-app-{}", source.sort_rank()),
        name: "Test App".to_string(),
        path: "/Applications/Test.app".to_string(),
        bundle_or_app_id: Some("com.example.test".to_string()),
        version: Some("1.0.0".to_string()),
        publisher: Some("Example".to_string()),
        platform: AppManagerPlatform::Macos,
        source,
        icon_kind: AppManagerIconKind::Iconify,
        icon_value: "i-noto:desktop-computer".to_string(),
        estimated_size_bytes: Some(1024),
        startup_enabled,
        startup_scope: if startup_enabled {
            AppManagerStartupScope::User
        } else {
            AppManagerStartupScope::None
        },
        startup_editable: true,
        readonly_reason_code: None,
        uninstall_supported: true,
        uninstall_kind: Some(AppManagerUninstallKind::FinderTrash),
        capabilities: build_app_capabilities(true, true, true),
        identity: build_app_identity(
            "com.example.test",
            vec!["Test App".to_string()],
            AppManagerIdentitySource::BundleId,
        ),
        risk_level: AppManagerRiskLevel::Low,
        fingerprint: "fp".to_string(),
    }
}

#[test]
fn app_manager_query_default_category_is_all() {
    let query = AppManagerQueryDto::default();
    assert_eq!(query.category, AppManagerCategory::All);
}

#[test]
fn app_manager_category_matches_item_contract() {
    let rtool_app = test_app(AppManagerSource::Rtool, false);
    let startup_app = test_app(AppManagerSource::Application, true);

    assert!(AppManagerCategory::All.matches_item(&rtool_app));
    assert!(AppManagerCategory::Unknown.matches_item(&rtool_app));

    assert!(AppManagerCategory::Rtool.matches_item(&rtool_app));
    assert!(!AppManagerCategory::Application.matches_item(&rtool_app));
    assert!(!AppManagerCategory::Startup.matches_item(&rtool_app));

    assert!(!AppManagerCategory::Rtool.matches_item(&startup_app));
    assert!(AppManagerCategory::Application.matches_item(&startup_app));
    assert!(AppManagerCategory::Startup.matches_item(&startup_app));
}

#[test]
fn app_manager_source_sort_rank_contract() {
    assert!(AppManagerSource::Application.sort_rank() < AppManagerSource::Rtool.sort_rank());
    assert!(AppManagerSource::Rtool.sort_rank() < AppManagerSource::Unknown.sort_rank());
}

#[test]
fn app_manager_cleanup_reason_code_from_error_code_contract() {
    assert_eq!(
        AppManagerCleanupReasonCode::from_error_code("app_manager_cleanup_delete_failed"),
        AppManagerCleanupReasonCode::AppManagerCleanupDeleteFailed
    );
    assert_eq!(
        AppManagerCleanupReasonCode::from_error_code("app_manager_cleanup_not_found"),
        AppManagerCleanupReasonCode::AppManagerCleanupNotFound
    );
    assert_eq!(
        AppManagerCleanupReasonCode::from_error_code("app_manager_cleanup_path_invalid"),
        AppManagerCleanupReasonCode::AppManagerCleanupPathInvalid
    );
    assert_eq!(
        AppManagerCleanupReasonCode::from_error_code("app_manager_cleanup_not_supported"),
        AppManagerCleanupReasonCode::AppManagerCleanupNotSupported
    );
    assert_eq!(
        AppManagerCleanupReasonCode::from_error_code("app_manager_uninstall_failed"),
        AppManagerCleanupReasonCode::AppManagerUninstallFailed
    );
    assert_eq!(
        AppManagerCleanupReasonCode::from_error_code("app_manager_future_error"),
        AppManagerCleanupReasonCode::Unknown
    );
}

#[test]
fn app_manager_cleanup_reason_code_serde_unknown_fallback_contract() {
    let parsed_known: AppManagerCleanupReasonCode =
        serde_json::from_str("\"managed_by_policy\"").expect("known value should deserialize");
    assert_eq!(parsed_known, AppManagerCleanupReasonCode::ManagedByPolicy);

    let parsed_unknown: AppManagerCleanupReasonCode =
        serde_json::from_str("\"future_reason_code\"").expect("unknown value should deserialize");
    assert_eq!(parsed_unknown, AppManagerCleanupReasonCode::Unknown);
}

#[test]
fn app_manager_action_code_serde_unknown_fallback_contract() {
    let serialized = serde_json::to_string(&AppManagerActionCode::AppManagerRefreshed)
        .expect("known action code should serialize");
    assert_eq!(serialized, "\"app_manager_refreshed\"");

    let parsed_known: AppManagerActionCode =
        serde_json::from_str("\"app_manager_uninstall_started\"")
            .expect("known action code should deserialize");
    assert_eq!(
        parsed_known,
        AppManagerActionCode::AppManagerUninstallStarted
    );

    let parsed_unknown: AppManagerActionCode =
        serde_json::from_str("\"app_manager_future_action\"")
            .expect("unknown action code should deserialize");
    assert_eq!(parsed_unknown, AppManagerActionCode::Unknown);
}

#[test]
fn app_manager_scan_warning_detail_code_from_io_error_kind_contract() {
    assert_eq!(
        AppManagerScanWarningDetailCode::from_io_error_kind(std::io::ErrorKind::PermissionDenied),
        AppManagerScanWarningDetailCode::PermissionDenied
    );
    assert_eq!(
        AppManagerScanWarningDetailCode::from_io_error_kind(std::io::ErrorKind::InvalidData),
        AppManagerScanWarningDetailCode::InvalidData
    );
    assert_eq!(
        AppManagerScanWarningDetailCode::from_io_error_kind(std::io::ErrorKind::Other),
        AppManagerScanWarningDetailCode::IoOther
    );
}

#[test]
fn app_manager_scan_warning_detail_code_serde_unknown_fallback_contract() {
    let parsed_known: AppManagerScanWarningDetailCode =
        serde_json::from_str("\"limit_reached\"").expect("known detail code should deserialize");
    assert_eq!(parsed_known, AppManagerScanWarningDetailCode::LimitReached);

    let parsed_unknown: AppManagerScanWarningDetailCode =
        serde_json::from_str("\"future_detail_code\"")
            .expect("unknown detail code should deserialize");
    assert_eq!(parsed_unknown, AppManagerScanWarningDetailCode::Unknown);
}

#[test]
fn app_manager_error_code_as_str_contract() {
    assert_eq!(
        AppManagerErrorCode::NotFound.as_str(),
        "app_manager_not_found"
    );
    assert_eq!(
        AppManagerErrorCode::CleanupDeleteFailed.as_str(),
        "app_manager_cleanup_delete_failed"
    );
    assert_eq!(
        AppManagerErrorCode::UninstallFailed.as_str(),
        "app_manager_uninstall_failed"
    );
}

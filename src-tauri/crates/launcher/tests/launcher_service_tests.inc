use super::*;
use crate::host::{AppPackageInfo, LauncherHost, LauncherWindow};
use protocol::models::ClipboardWindowModeAppliedDto;
use serde_json::Value;
use std::path::PathBuf;

struct MockLauncherHost {
    locale: Option<String>,
    app_data_dir: PathBuf,
}

struct MockLauncherWindow;

impl LauncherWindow for MockLauncherWindow {
    fn show(&self) -> protocol::AppResult<()> {
        Ok(())
    }

    fn set_focus(&self) -> protocol::AppResult<()> {
        Ok(())
    }
}

impl LauncherHost for MockLauncherHost {
    fn emit(&self, _event: &str, _payload: Value) -> protocol::AppResult<()> {
        Ok(())
    }

    fn get_webview_window(&self, _label: &str) -> Option<Box<dyn LauncherWindow>> {
        Some(Box::new(MockLauncherWindow))
    }

    fn app_data_dir(&self) -> protocol::AppResult<PathBuf> {
        Ok(self.app_data_dir.clone())
    }

    fn package_info(&self) -> AppPackageInfo {
        AppPackageInfo {
            name: "rtool-tests".to_string(),
            version: "0.0.0".to_string(),
        }
    }

    fn resolved_locale(&self) -> Option<String> {
        self.locale.clone()
    }

    fn apply_clipboard_window_mode(
        &self,
        compact: bool,
        _source: &str,
    ) -> protocol::AppResult<ClipboardWindowModeAppliedDto> {
        Ok(ClipboardWindowModeAppliedDto {
            compact,
            applied_width_logical: 480.0,
            applied_height_logical: 360.0,
            scale_factor: 1.0,
        })
    }
}

#[test]
fn should_score_exact_match_higher_than_contains() {
    let exact = calculate_match_score("工具：Base64", "工具：base64");
    let partial = calculate_match_score("打开 Base64 编码工具", "base64");
    assert!(exact > partial);
}

#[test]
fn should_prioritize_builtin_category_weight() {
    let builtin = category_weight("builtin");
    let directory = category_weight("directory");
    let file = category_weight("file");
    assert!(builtin > directory);
    assert!(directory > file);
}

#[test]
fn should_rank_directory_between_application_and_file() {
    let application_rank = category_rank("application");
    let directory_rank = category_rank("directory");
    let file_rank = category_rank("file");
    assert!(application_rank < directory_rank);
    assert!(directory_rank < file_rank);
}

#[test]
fn should_show_directory_when_query_empty() {
    let directory_item = LauncherItemDto {
        id: "dir.docs".into(),
        title: "Documents".into(),
        subtitle: "/Users/example".into(),
        category: "directory".into(),
        source: Some("目录".into()),
        shortcut: None,
        score: 0,
        icon_kind: "iconify".into(),
        icon_value: "i-noto:file-folder".into(),
        action: LauncherActionDto::OpenDirectory {
            path: "/Users/example/Documents".into(),
        },
    };

    let scored = score_item(directory_item, "", "zh-CN");
    assert!(scored.is_some());

    let scored = scored.expect("directory should be visible on empty query");
    assert_eq!(scored.category, "directory");
}

#[test]
fn should_filter_non_matching_item() {
    let item = LauncherItemDto {
        id: "x".into(),
        title: "打开工具箱".into(),
        subtitle: "系统页面".into(),
        category: "builtin".into(),
        source: Some("内置".into()),
        shortcut: None,
        score: 0,
        icon_kind: "iconify".into(),
        icon_value: "i-noto:hammer-and-wrench".into(),
        action: LauncherActionDto::OpenBuiltinRoute {
            route: "/tools".into(),
        },
    };

    let found = score_item(item.clone(), "工具", "zh-CN");
    let not_found = score_item(item, "not-exist-token", "zh-CN");
    assert!(found.is_some());
    assert!(not_found.is_none());
}

#[test]
fn should_match_builtin_alias_terms_across_languages() {
    let item = LauncherItemDto {
        id: "builtin.tools".into(),
        title: "打开工具箱".into(),
        subtitle: "跳转到工具箱页面".into(),
        category: "builtin".into(),
        source: Some("内置".into()),
        shortcut: None,
        score: 0,
        icon_kind: "iconify".into(),
        icon_value: "i-noto:hammer-and-wrench".into(),
        action: LauncherActionDto::OpenBuiltinRoute {
            route: "/tools".into(),
        },
    };

    let matched = score_item(item, "open tools", "zh-CN");
    assert!(matched.is_some());
}

#[test]
fn should_hide_builtin_tools_when_query_empty() {
    let hidden_tool = LauncherItemDto {
        id: "builtin.tool.base64".into(),
        title: "Base64 编解码".into(),
        subtitle: "打开 Base64 工具".into(),
        category: "builtin".into(),
        source: Some("内置".into()),
        shortcut: None,
        score: 0,
        icon_kind: "iconify".into(),
        icon_value: "i-noto:input-symbols".into(),
        action: LauncherActionDto::OpenBuiltinTool {
            tool_id: "base64".into(),
        },
    };

    let visible_builtin = LauncherItemDto {
        id: "builtin.tools".into(),
        title: "工具箱".into(),
        subtitle: "打开工具箱".into(),
        category: "builtin".into(),
        source: Some("内置".into()),
        shortcut: None,
        score: 0,
        icon_kind: "iconify".into(),
        icon_value: "i-noto:hammer-and-wrench".into(),
        action: LauncherActionDto::OpenBuiltinRoute {
            route: "/tools".into(),
        },
    };

    assert!(score_item(hidden_tool.clone(), "", "zh-CN").is_none());
    assert!(score_item(hidden_tool, "base64", "zh-CN").is_some());
    assert!(score_item(visible_builtin, "", "zh-CN").is_some());
}

#[test]
fn should_use_host_locale_and_fallback_to_default_locale() {
    let app_data_dir = std::env::temp_dir().join("rtool-launcher-service-test");

    let host = MockLauncherHost {
        locale: Some("en-US".to_string()),
        app_data_dir: app_data_dir.clone(),
    };
    assert_eq!(current_locale(&host), "en-US");

    let host_with_blank_locale = MockLauncherHost {
        locale: Some("   ".to_string()),
        app_data_dir: app_data_dir.clone(),
    };
    assert_eq!(
        current_locale(&host_with_blank_locale),
        DEFAULT_RESOLVED_LOCALE
    );

    let host_without_locale = MockLauncherHost {
        locale: None,
        app_data_dir,
    };
    assert_eq!(
        current_locale(&host_without_locale),
        DEFAULT_RESOLVED_LOCALE
    );
}

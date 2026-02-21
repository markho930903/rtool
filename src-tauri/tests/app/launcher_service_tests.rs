use super::*;

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
fn should_execute_palette_legacy_supported_action() {
    let result = execute_palette_legacy("builtin.tools");
    assert!(result.is_ok());
    assert_eq!(result.ok(), Some("route:/tools".to_string()));
}

#[test]
fn should_reject_palette_legacy_unknown_action() {
    let result = execute_palette_legacy("unknown.action");
    assert!(result.is_err());
    let error = result.expect_err("palette action should return error");
    assert_eq!(error.code, "palette_action_unsupported");
}

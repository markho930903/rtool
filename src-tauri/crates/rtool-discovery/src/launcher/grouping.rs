use rtool_contracts::models::{LauncherActionDto, LauncherItemDto};

const GROUP_TOOL: &str = "tool";
const GROUP_SOCIAL: &str = "social";
const GROUP_EFFICIENCY: &str = "efficiency";
const GROUP_OTHER: &str = "other";

const SOCIAL_TERMS: &[&str] = &[
    "wechat",
    "weixin",
    "dingtalk",
    "钉钉",
    "qq",
    "telegram",
    "wecom",
    "企业微信",
    "slack",
    "discord",
    "teams",
    "zoom",
    "飞书",
    "lark",
    "message",
    "messages",
    "messenger",
    "chat",
    "im",
    "社交",
    "聊天",
    "通讯",
    "消息",
];

const TOOL_TERMS: &[&str] = &[
    "code",
    "studio",
    "terminal",
    "warp",
    "chrome",
    "browser",
    "dev",
    "sdk",
    "xcode",
    "git",
    "docker",
    "tool",
    "tools",
    "utility",
    "utilities",
    "开发",
    "工具",
    "浏览器",
];

const EFFICIENCY_TERMS: &[&str] = &[
    "calendar",
    "clock",
    "notes",
    "reminder",
    "mail",
    "calculator",
    "maps",
    "weather",
    "stock",
    "office",
    "文稿",
    "备忘录",
    "日历",
    "时钟",
    "提醒",
    "通讯录",
    "地图",
    "天气",
    "效率",
    "生产力",
    "todo",
    "tasks",
    "备忘",
    "待办",
    "提醒事项",
];

fn normalized_item_corpus(item: &LauncherItemDto) -> String {
    let mut parts: Vec<String> = vec![
        item.id.to_ascii_lowercase(),
        item.title.to_ascii_lowercase(),
        item.subtitle.to_ascii_lowercase(),
        item.category.to_ascii_lowercase(),
    ];

    if let Some(source) = &item.source {
        parts.push(source.to_ascii_lowercase());
    }

    match &item.action {
        LauncherActionDto::OpenBuiltinRoute { route } => {
            parts.push(route.to_ascii_lowercase());
        }
        LauncherActionDto::OpenBuiltinTool { tool_id } => {
            parts.push(tool_id.to_ascii_lowercase());
        }
        LauncherActionDto::OpenBuiltinWindow { window_label } => {
            parts.push(window_label.to_ascii_lowercase());
        }
        LauncherActionDto::OpenDirectory { path }
        | LauncherActionDto::OpenFile { path }
        | LauncherActionDto::OpenApplication { path } => {
            parts.push(path.to_ascii_lowercase());
        }
    }

    parts.join(" ")
}

fn contains_any(content: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| content.contains(term))
}

fn builtin_group(item: &LauncherItemDto) -> Option<&'static str> {
    match item.id.as_str() {
        "builtin.tools"
        | "builtin.tool.base64"
        | "builtin.tool.regex"
        | "builtin.tool.timestamp" => Some(GROUP_TOOL),
        "builtin.clipboard" | "builtin.screenshot" => Some(GROUP_EFFICIENCY),
        _ => None,
    }
}

pub fn resolve_launcher_group(item: &LauncherItemDto) -> &'static str {
    if item.category.eq_ignore_ascii_case("builtin")
        && let Some(group) = builtin_group(item)
    {
        return group;
    }

    let corpus = normalized_item_corpus(item);

    if contains_any(corpus.as_str(), SOCIAL_TERMS) {
        return GROUP_SOCIAL;
    }

    if contains_any(corpus.as_str(), TOOL_TERMS) {
        return GROUP_TOOL;
    }

    if contains_any(corpus.as_str(), EFFICIENCY_TERMS) {
        return GROUP_EFFICIENCY;
    }

    GROUP_OTHER
}

pub fn with_launcher_group(mut item: LauncherItemDto) -> LauncherItemDto {
    item.group = resolve_launcher_group(&item).to_string();
    item
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item(
        title: &str,
        subtitle: &str,
        category: &str,
        id: &str,
        action: LauncherActionDto,
    ) -> LauncherItemDto {
        LauncherItemDto {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            category: category.to_string(),
            group: String::new(),
            source: None,
            shortcut: None,
            score: 0,
            icon_kind: "iconify".to_string(),
            icon_value: "i-noto:card-index-dividers".to_string(),
            action,
        }
    }

    #[test]
    fn builtin_tool_maps_to_tool_group() {
        let item = sample_item(
            "Tool: Base64",
            "Open tool",
            "builtin",
            "builtin.tool.base64",
            LauncherActionDto::OpenBuiltinTool {
                tool_id: "base64".to_string(),
            },
        );
        assert_eq!(resolve_launcher_group(&item), GROUP_TOOL);
    }

    #[test]
    fn builtin_clipboard_maps_to_efficiency_group() {
        let item = sample_item(
            "Clipboard",
            "Open clipboard window",
            "builtin",
            "builtin.clipboard",
            LauncherActionDto::OpenBuiltinWindow {
                window_label: "clipboard_history".to_string(),
            },
        );
        assert_eq!(resolve_launcher_group(&item), GROUP_EFFICIENCY);
    }

    #[test]
    fn social_keyword_maps_to_social_group() {
        let item = sample_item(
            "WeChat",
            "chat app",
            "application",
            "app.wechat",
            LauncherActionDto::OpenApplication {
                path: "/Applications/WeChat.app".to_string(),
            },
        );
        assert_eq!(resolve_launcher_group(&item), GROUP_SOCIAL);
    }

    #[test]
    fn tool_keyword_maps_to_tool_group() {
        let item = sample_item(
            "Visual Studio Code",
            "developer tool",
            "application",
            "app.vscode",
            LauncherActionDto::OpenApplication {
                path: "/Applications/Visual Studio Code.app".to_string(),
            },
        );
        assert_eq!(resolve_launcher_group(&item), GROUP_TOOL);
    }

    #[test]
    fn efficiency_keyword_maps_to_efficiency_group() {
        let item = sample_item(
            "Calendar",
            "events and reminders",
            "application",
            "app.calendar",
            LauncherActionDto::OpenApplication {
                path: "/Applications/Calendar.app".to_string(),
            },
        );
        assert_eq!(resolve_launcher_group(&item), GROUP_EFFICIENCY);
    }

    #[test]
    fn unmatched_maps_to_other_group() {
        let item = sample_item(
            "Unknown",
            "random",
            "application",
            "app.unknown",
            LauncherActionDto::OpenApplication {
                path: "/Applications/Unknown.app".to_string(),
            },
        );
        assert_eq!(resolve_launcher_group(&item), GROUP_OTHER);
    }
}

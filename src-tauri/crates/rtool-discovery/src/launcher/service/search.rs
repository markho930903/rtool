use crate::host::LauncherHost;
use crate::launcher::grouping::with_launcher_group;
use crate::launcher::icon::resolve_builtin_icon;
use crate::launcher::index::search_indexed_items_async;
use rtool_contracts::models::{LauncherActionDto, LauncherItemDto};
use rtool_data::db::DbConn;
use rtool_kernel::i18n::{DEFAULT_RESOLVED_LOCALE, ResolvedAppLocale, t};
use std::time::Instant;

const DEFAULT_RESULT_LIMIT: usize = 60;
const MAX_RESULT_LIMIT: usize = 120;

#[derive(Debug, Clone, Default)]
pub struct LauncherSearchDiagnostics {
    pub index_used: bool,
    pub index_failed: bool,
    pub index_query_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocaleKind {
    Zh,
    En,
    Other,
}

impl LocaleKind {
    fn from_resolved(locale: &str) -> Self {
        let lower = locale.to_ascii_lowercase();
        if lower.starts_with("zh") {
            return Self::Zh;
        }
        if lower.starts_with("en") {
            return Self::En;
        }
        Self::Other
    }
}

#[derive(Debug, Clone)]
struct QueryPattern<'a> {
    text: &'a str,
    tokens: Vec<&'a str>,
}

impl<'a> QueryPattern<'a> {
    fn new(text: &'a str) -> Self {
        let tokens = text
            .split_whitespace()
            .filter(|token| !token.is_empty())
            .collect();
        Self { text, tokens }
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

fn current_locale(app: &dyn LauncherHost) -> ResolvedAppLocale {
    app.resolved_locale()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_RESOLVED_LOCALE.to_string())
}

fn elapsed_ms(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub async fn search_launcher_async(
    app: &dyn LauncherHost,
    db_conn: &DbConn,
    query: &str,
    limit: Option<u16>,
) -> (Vec<LauncherItemDto>, LauncherSearchDiagnostics) {
    let normalized = normalize_query(query);
    let query_pattern = QueryPattern::new(&normalized);
    let locale = current_locale(app);
    let locale_kind = LocaleKind::from_resolved(&locale);
    let result_limit = limit
        .map(usize::from)
        .unwrap_or(DEFAULT_RESULT_LIMIT)
        .clamp(1, MAX_RESULT_LIMIT);

    let mut diagnostics = LauncherSearchDiagnostics::default();
    let mut items = builtin_items(&locale);

    let index_started_at = Instant::now();
    let index_result =
        search_indexed_items_async(app, db_conn, &normalized, &locale, result_limit).await;
    diagnostics.index_query_duration_ms = Some(elapsed_ms(index_started_at));

    match index_result {
        Ok(index_result) => {
            if index_result.ready {
                diagnostics.index_used = true;
                items.extend(index_result.items);
            } else {
                tracing::info!(event = "launcher_index_not_ready");
            }
        }
        Err(error) => {
            diagnostics.index_failed = true;
            tracing::warn!(
                event = "launcher_index_query_failed",
                error = error.to_string()
            );
        }
    }

    let mut matched: Vec<LauncherItemDto> = items
        .into_iter()
        .filter_map(|item| score_item(item, &query_pattern, locale_kind))
        .collect();

    matched.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| category_rank(&left.category).cmp(&category_rank(&right.category)))
            .then_with(|| left.title.cmp(&right.title))
    });

    matched.truncate(result_limit);
    (matched, diagnostics)
}

fn should_hide_item_without_query(item: &LauncherItemDto) -> bool {
    matches!(&item.action, LauncherActionDto::OpenBuiltinTool { .. })
}

fn builtin_items(locale: &str) -> Vec<LauncherItemDto> {
    vec![
        build_builtin_route_item(
            locale,
            "builtin.tools",
            t(locale, "launcher.builtin.tools.title"),
            t(locale, "launcher.builtin.tools.subtitle"),
            "/tools",
            "i-noto:hammer-and-wrench",
            None,
        ),
        build_builtin_window_item(
            locale,
            "builtin.clipboard",
            t(locale, "launcher.builtin.clipboard.title"),
            t(locale, "launcher.builtin.clipboard.subtitle"),
            "clipboard_history",
            "i-noto:clipboard",
            Some("Alt + V"),
        ),
        build_builtin_window_item(
            locale,
            "builtin.screenshot",
            t(locale, "launcher.builtin.screenshot.title"),
            t(locale, "launcher.builtin.screenshot.subtitle"),
            "screenshot_overlay",
            "i-noto:framed-picture",
            Some("Alt + Shift + S"),
        ),
        build_builtin_tool_item(
            locale,
            "builtin.tool.base64",
            t(locale, "launcher.builtin.tool.base64.title"),
            t(locale, "launcher.builtin.tool.base64.subtitle"),
            "base64",
            "i-noto:input-symbols",
        ),
        build_builtin_tool_item(
            locale,
            "builtin.tool.regex",
            t(locale, "launcher.builtin.tool.regex.title"),
            t(locale, "launcher.builtin.tool.regex.subtitle"),
            "regex",
            "i-noto:magnifying-glass-tilted-right",
        ),
        build_builtin_tool_item(
            locale,
            "builtin.tool.timestamp",
            t(locale, "launcher.builtin.tool.timestamp.title"),
            t(locale, "launcher.builtin.tool.timestamp.subtitle"),
            "timestamp",
            "i-noto:mantelpiece-clock",
        ),
    ]
}

fn build_builtin_route_item(
    locale: &str,
    id: &str,
    title: String,
    subtitle: String,
    route: &str,
    icon: &str,
    shortcut: Option<&str>,
) -> LauncherItemDto {
    let payload = resolve_builtin_icon(icon);
    with_launcher_group(LauncherItemDto {
        id: id.to_string(),
        title,
        subtitle,
        category: "builtin".to_string(),
        group: String::new(),
        source: Some(t(locale, "launcher.source.builtin")),
        shortcut: shortcut.map(ToString::to_string),
        score: 0,
        icon_kind: payload.kind,
        icon_value: payload.value,
        action: LauncherActionDto::OpenBuiltinRoute {
            route: route.to_string(),
        },
    })
}

fn build_builtin_tool_item(
    locale: &str,
    id: &str,
    title: String,
    subtitle: String,
    tool_id: &str,
    icon: &str,
) -> LauncherItemDto {
    let payload = resolve_builtin_icon(icon);
    with_launcher_group(LauncherItemDto {
        id: id.to_string(),
        title,
        subtitle,
        category: "builtin".to_string(),
        group: String::new(),
        source: Some(t(locale, "launcher.source.builtin")),
        shortcut: None,
        score: 0,
        icon_kind: payload.kind,
        icon_value: payload.value,
        action: LauncherActionDto::OpenBuiltinTool {
            tool_id: tool_id.to_string(),
        },
    })
}

fn build_builtin_window_item(
    locale: &str,
    id: &str,
    title: String,
    subtitle: String,
    window_label: &str,
    icon: &str,
    shortcut: Option<&str>,
) -> LauncherItemDto {
    let payload = resolve_builtin_icon(icon);
    with_launcher_group(LauncherItemDto {
        id: id.to_string(),
        title,
        subtitle,
        category: "builtin".to_string(),
        group: String::new(),
        source: Some(t(locale, "launcher.source.builtin")),
        shortcut: shortcut.map(ToString::to_string),
        score: 0,
        icon_kind: payload.kind,
        icon_value: payload.value,
        action: LauncherActionDto::OpenBuiltinWindow {
            window_label: window_label.to_string(),
        },
    })
}

fn score_item(
    mut item: LauncherItemDto,
    query: &QueryPattern<'_>,
    locale_kind: LocaleKind,
) -> Option<LauncherItemDto> {
    if query.is_empty() && should_hide_item_without_query(&item) {
        return None;
    }

    let base = category_weight(&item.category);
    if query.is_empty() {
        item.score = base;
        return Some(item);
    }

    let title_score = calculate_match_score(&item.title, query);
    let subtitle_score = calculate_match_score(&item.subtitle, query);
    let alias_score = calculate_alias_score(&item, query, locale_kind);

    let best = title_score.max(subtitle_score).max(alias_score);
    if best <= 0 {
        return None;
    }

    let tail = title_score.min(subtitle_score) / 4;
    item.score = base + best + tail + alias_score / 5;
    Some(item)
}

fn calculate_alias_score(
    item: &LauncherItemDto,
    query: &QueryPattern<'_>,
    locale_kind: LocaleKind,
) -> i32 {
    if query.is_empty() {
        return 0;
    }

    let mut best = 0;
    for alias in alias_terms(item.id.as_str(), locale_kind) {
        let score = calculate_match_score(alias, query);
        if score > best {
            best = score;
        }
    }
    best
}

fn alias_terms(id: &str, locale_kind: LocaleKind) -> &'static [&'static str] {
    match id {
        "builtin.tools" | "action.open-tools" if locale_kind == LocaleKind::Zh => {
            &["open tools", "toolbox", "tools", "utilities"]
        }
        "builtin.tools" | "action.open-tools" if locale_kind == LocaleKind::En => {
            &["打开工具箱", "工具箱", "工具", "实用工具"]
        }
        "builtin.clipboard" if locale_kind == LocaleKind::Zh => {
            &["clipboard", "clipboard history", "clip history"]
        }
        "builtin.clipboard" if locale_kind == LocaleKind::En => {
            &["剪贴板", "剪贴板历史", "复制历史"]
        }
        "builtin.tool.base64" if locale_kind == LocaleKind::Zh => {
            &["base64", "encode", "decode", "tool base64"]
        }
        "builtin.tool.base64" if locale_kind == LocaleKind::En => {
            &["base64", "编码", "解码", "工具"]
        }
        "builtin.tool.regex" if locale_kind == LocaleKind::Zh => {
            &["regex", "regular expression", "regexp"]
        }
        "builtin.tool.regex" if locale_kind == LocaleKind::En => {
            &["正则", "正则表达式", "匹配工具"]
        }
        "builtin.tool.timestamp" if locale_kind == LocaleKind::Zh => {
            &["timestamp", "time", "unix time"]
        }
        "builtin.tool.timestamp" if locale_kind == LocaleKind::En => {
            &["时间戳", "时间", "时间转换"]
        }
        _ => &[],
    }
}

fn calculate_match_score(source: &str, query: &QueryPattern<'_>) -> i32 {
    let normalized = normalize_query(source);
    if normalized.is_empty() || query.is_empty() {
        return 0;
    }

    if normalized == query.text {
        return 140;
    }

    if normalized.starts_with(query.text) {
        return 120;
    }

    if normalized.contains(query.text) {
        return 95;
    }

    let mut score = 0;
    for token in &query.tokens {
        if normalized.starts_with(token) {
            score += 24;
        } else if normalized.contains(token) {
            score += 16;
        }
    }

    score
}

fn normalize_query(value: &str) -> String {
    value.trim().to_lowercase()
}

fn category_weight(category: &str) -> i32 {
    match category {
        "builtin" => 240,
        "application" => 160,
        "directory" => 140,
        "file" => 120,
        _ => 80,
    }
}

fn category_rank(category: &str) -> i32 {
    match category {
        "builtin" => 0,
        "application" => 1,
        "directory" => 2,
        "file" => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item(
        id: &str,
        title: &str,
        subtitle: &str,
        action: LauncherActionDto,
    ) -> LauncherItemDto {
        LauncherItemDto {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            category: "builtin".to_string(),
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
    fn locale_kind_is_detected_once() {
        assert_eq!(LocaleKind::from_resolved("zh-CN"), LocaleKind::Zh);
        assert_eq!(LocaleKind::from_resolved("EN-us"), LocaleKind::En);
        assert_eq!(LocaleKind::from_resolved("ja-JP"), LocaleKind::Other);
    }

    #[test]
    fn query_pattern_keeps_tokens() {
        let query = QueryPattern::new("foo   bar baz");
        assert_eq!(query.tokens, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn builtin_tool_hidden_without_query() {
        let item = sample_item(
            "builtin.tool.base64",
            "Base64",
            "Encode decode",
            LauncherActionDto::OpenBuiltinTool {
                tool_id: "base64".to_string(),
            },
        );
        assert!(score_item(item, &QueryPattern::new(""), LocaleKind::Zh).is_none());
    }

    #[test]
    fn alias_score_uses_locale_bucket() {
        let item = sample_item(
            "builtin.tool.regex",
            "Regex",
            "Regular expression",
            LauncherActionDto::OpenBuiltinTool {
                tool_id: "regex".to_string(),
            },
        );

        let zh_query = QueryPattern::new("regexp");
        assert!(calculate_alias_score(&item, &zh_query, LocaleKind::Zh) > 0);

        let en_query = QueryPattern::new("正则表达式");
        assert!(calculate_alias_score(&item, &en_query, LocaleKind::En) > 0);
    }

    #[test]
    fn match_score_prefers_exact_then_prefix() {
        let exact = calculate_match_score("base64", &QueryPattern::new("base64"));
        let prefix = calculate_match_score("base64 encode", &QueryPattern::new("base64"));
        let contains = calculate_match_score("tool for base64", &QueryPattern::new("base64"));

        assert!(exact > prefix);
        assert!(prefix > contains);
    }
}

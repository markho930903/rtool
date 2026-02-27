use crate::host::LauncherHost;
use crate::launcher::icon::resolve_builtin_icon;
use crate::launcher::index::search_indexed_items_async;
use anyhow::Context;
use protocol::models::{ClipboardWindowOpenedPayload, LauncherActionDto, LauncherItemDto};
use protocol::{AppError, AppResult, ResultExt};
use rtool_db::db::DbConn;
use rtool_i18n::i18n::{DEFAULT_RESOLVED_LOCALE, ResolvedAppLocale, t};
use serde::Serialize;
use serde_json::json;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const DEFAULT_RESULT_LIMIT: usize = 60;
const MAX_RESULT_LIMIT: usize = 120;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NavigatePayload {
    route: String,
}

fn current_locale(app: &dyn LauncherHost) -> ResolvedAppLocale {
    app.resolved_locale()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_RESOLVED_LOCALE.to_string())
}

#[derive(Debug, Clone, Default)]
pub struct LauncherSearchDiagnostics {
    pub index_used: bool,
    pub index_failed: bool,
    pub index_query_duration_ms: Option<u64>,
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
    let locale = current_locale(app);
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
        .filter_map(|item| score_item(item, &normalized, &locale))
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

pub fn execute_launcher_action(
    app: &dyn LauncherHost,
    action: &LauncherActionDto,
) -> AppResult<String> {
    match action {
        LauncherActionDto::OpenBuiltinRoute { route } => {
            open_main_with_route(app, route.clone()).map(|_| format!("route:{route}"))
        }
        LauncherActionDto::OpenBuiltinTool { tool_id } => {
            let route = format!("/tools?tool={tool_id}");
            open_main_with_route(app, route.clone()).map(|_| format!("route:{route}"))
        }
        LauncherActionDto::OpenBuiltinWindow { window_label } => open_window(app, window_label)
            .and_then(|_| {
                if window_label == "clipboard_history" {
                    if let Err(error) = app.apply_clipboard_window_mode(false, "launcher_open") {
                        tracing::warn!(
                            event = "clipboard_window_mode_apply_failed",
                            source = "launcher_open",
                            compact = false,
                            error = error.to_string()
                        );
                    }
                    let payload =
                        serde_json::to_value(ClipboardWindowOpenedPayload { compact: false })
                            .with_context(|| "构造剪贴板窗口事件载荷失败".to_string())
                            .with_code("launcher_emit_payload_failed", "打开窗口失败")?;
                    app.emit("rtool://clipboard-window/opened", payload)
                        .map_err(|error| error.with_context("windowLabel", window_label))?;
                }
                Ok(format!("window:{window_label}"))
            }),
        LauncherActionDto::OpenDirectory { path }
        | LauncherActionDto::OpenFile { path }
        | LauncherActionDto::OpenApplication { path } => {
            open_path(Path::new(path)).map(|_| format!("path:{path}"))
        }
    }
}

fn open_main_with_route(app: &dyn LauncherHost, route: String) -> AppResult<()> {
    open_window(app, "main")?;
    app.emit("rtool://main/navigate", json!(NavigatePayload { route }))
        .map_err(|error| {
            error
                .with_code("launcher_navigate_failed", "打开页面失败")
                .with_context("event", "rtool://main/navigate")
        })
}

fn open_window(app: &dyn LauncherHost, label: &str) -> AppResult<()> {
    let window = app.get_webview_window(label).ok_or_else(|| {
        AppError::new("launcher_window_not_found", "目标窗口不存在").with_context("label", label)
    })?;

    window
        .show()
        .with_context(|| format!("显示窗口失败: {label}"))
        .with_code("launcher_window_show_failed", "打开窗口失败")?;
    window
        .set_focus()
        .with_context(|| format!("聚焦窗口失败: {label}"))
        .with_code("launcher_window_focus_failed", "打开窗口失败")
}

fn open_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(
            AppError::new("launcher_path_not_found", "打开失败：路径不存在")
                .with_context("path", path.to_string_lossy().to_string()),
        );
    }

    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(path).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .status()
    } else {
        Command::new("xdg-open").arg(path).status()
    }
    .with_context(|| format!("执行系统打开命令失败: {}", path.to_string_lossy()))
    .with_code("launcher_path_open_failed", "打开失败")?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::new("launcher_path_open_failed", "打开失败")
            .with_context("status", status.to_string()))
    }
}

fn builtin_items(locale: &str) -> Vec<LauncherItemDto> {
    vec![
        build_builtin_route_item(
            locale,
            "builtin.dashboard",
            t(locale, "launcher.builtin.dashboard.title"),
            t(locale, "launcher.builtin.dashboard.subtitle"),
            "/",
            "i-noto:desktop-computer",
            None,
        ),
        build_builtin_route_item(
            locale,
            "builtin.tools",
            t(locale, "launcher.builtin.tools.title"),
            t(locale, "launcher.builtin.tools.subtitle"),
            "/tools",
            "i-noto:hammer-and-wrench",
            None,
        ),
        build_builtin_route_item(
            locale,
            "builtin.transfer",
            t(locale, "launcher.builtin.transfer.title"),
            t(locale, "launcher.builtin.transfer.subtitle"),
            "/transfer",
            "i-noto:outbox-tray",
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
    LauncherItemDto {
        id: id.to_string(),
        title,
        subtitle,
        category: "builtin".to_string(),
        source: Some(t(locale, "launcher.source.builtin")),
        shortcut: shortcut.map(ToString::to_string),
        score: 0,
        icon_kind: payload.kind,
        icon_value: payload.value,
        action: LauncherActionDto::OpenBuiltinRoute {
            route: route.to_string(),
        },
    }
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
    LauncherItemDto {
        id: id.to_string(),
        title,
        subtitle,
        category: "builtin".to_string(),
        source: Some(t(locale, "launcher.source.builtin")),
        shortcut: None,
        score: 0,
        icon_kind: payload.kind,
        icon_value: payload.value,
        action: LauncherActionDto::OpenBuiltinTool {
            tool_id: tool_id.to_string(),
        },
    }
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
    LauncherItemDto {
        id: id.to_string(),
        title,
        subtitle,
        category: "builtin".to_string(),
        source: Some(t(locale, "launcher.source.builtin")),
        shortcut: shortcut.map(ToString::to_string),
        score: 0,
        icon_kind: payload.kind,
        icon_value: payload.value,
        action: LauncherActionDto::OpenBuiltinWindow {
            window_label: window_label.to_string(),
        },
    }
}

fn score_item(
    mut item: LauncherItemDto,
    normalized_query: &str,
    locale: &str,
) -> Option<LauncherItemDto> {
    if normalized_query.is_empty() && should_hide_item_without_query(&item) {
        return None;
    }

    let base = category_weight(&item.category);
    if normalized_query.is_empty() {
        item.score = base;
        return Some(item);
    }

    let title_score = calculate_match_score(&item.title, normalized_query);
    let subtitle_score = calculate_match_score(&item.subtitle, normalized_query);
    let alias_score = calculate_alias_score(&item, normalized_query, locale);

    let best = title_score.max(subtitle_score).max(alias_score);
    if best <= 0 {
        return None;
    }

    let tail = title_score.min(subtitle_score) / 4;
    item.score = base + best + tail + alias_score / 5;
    Some(item)
}

fn calculate_alias_score(item: &LauncherItemDto, normalized_query: &str, locale: &str) -> i32 {
    if normalized_query.is_empty() {
        return 0;
    }

    let mut best = 0;
    for alias in alias_terms(item.id.as_str(), locale) {
        let score = calculate_match_score(alias, normalized_query);
        if score > best {
            best = score;
        }
    }
    best
}

fn alias_terms(id: &str, locale: &str) -> &'static [&'static str] {
    match id {
        "builtin.dashboard" | "action.open-home" if is_zh_locale(locale) => {
            &["open dashboard", "dashboard", "home", "main page"]
        }
        "builtin.dashboard" | "action.open-home" if is_en_locale(locale) => {
            &["打开仪表盘", "仪表盘", "首页", "主页面"]
        }
        "builtin.tools" | "action.open-tools" if is_zh_locale(locale) => {
            &["open tools", "toolbox", "tools", "utilities"]
        }
        "builtin.tools" | "action.open-tools" if is_en_locale(locale) => {
            &["打开工具箱", "工具箱", "工具", "实用工具"]
        }
        "builtin.transfer" | "action.open-transfer" if is_zh_locale(locale) => {
            &["file transfer", "transfer", "send files", "sync files"]
        }
        "builtin.transfer" | "action.open-transfer" if is_en_locale(locale) => {
            &["文件传输", "传输", "发送文件", "互传"]
        }
        "builtin.clipboard" if is_zh_locale(locale) => {
            &["clipboard", "clipboard history", "clip history"]
        }
        "builtin.clipboard" if is_en_locale(locale) => &["剪贴板", "剪贴板历史", "复制历史"],
        "builtin.tool.base64" if is_zh_locale(locale) => {
            &["base64", "encode", "decode", "tool base64"]
        }
        "builtin.tool.base64" if is_en_locale(locale) => &["base64", "编码", "解码", "工具"],
        "builtin.tool.regex" if is_zh_locale(locale) => &["regex", "regular expression", "regexp"],
        "builtin.tool.regex" if is_en_locale(locale) => &["正则", "正则表达式", "匹配工具"],
        "builtin.tool.timestamp" if is_zh_locale(locale) => &["timestamp", "time", "unix time"],
        "builtin.tool.timestamp" if is_en_locale(locale) => &["时间戳", "时间", "时间转换"],
        _ => &[],
    }
}

fn is_zh_locale(locale: &str) -> bool {
    locale.to_ascii_lowercase().starts_with("zh")
}

fn is_en_locale(locale: &str) -> bool {
    locale.to_ascii_lowercase().starts_with("en")
}

fn calculate_match_score(source: &str, query: &str) -> i32 {
    let normalized = normalize_query(source);
    if normalized.is_empty() || query.is_empty() {
        return 0;
    }

    if normalized == query {
        return 140;
    }

    if normalized.starts_with(query) {
        return 120;
    }

    if normalized.contains(query) {
        return 95;
    }

    let mut score = 0;
    for token in query.split_whitespace() {
        if token.is_empty() {
            continue;
        }

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
#[path = "../../tests/launcher_service_tests.inc"]
mod tests;

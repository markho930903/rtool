use crate::app::icon_service::{
    resolve_application_icon, resolve_builtin_icon, resolve_file_type_icon,
};
use crate::app::state::AppState;
use crate::core::i18n::{DEFAULT_RESOLVED_LOCALE, ResolvedAppLocale, t};
use crate::core::models::{
    ActionResultDto, ClipboardWindowOpenedPayload, LauncherActionDto, LauncherItemDto,
};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const CACHE_TTL: Duration = Duration::from_secs(30);
const DEFAULT_RESULT_LIMIT: usize = 60;
const MAX_RESULT_LIMIT: usize = 120;
const MAX_APP_ITEMS: usize = 300;
const MAX_FILE_ITEMS: usize = 600;
const APP_SCAN_DEPTH: usize = 4;
const FILE_SCAN_DEPTH: usize = 3;

struct LauncherCache {
    refreshed_at: Option<Instant>,
    locale: Option<String>,
    application_items: Vec<LauncherItemDto>,
    file_items: Vec<LauncherItemDto>,
}

impl LauncherCache {
    fn new() -> Self {
        Self {
            refreshed_at: None,
            locale: None,
            application_items: Vec::new(),
            file_items: Vec::new(),
        }
    }

    fn is_stale(&self, locale: &str) -> bool {
        if self.locale.as_deref() != Some(locale) {
            return true;
        }

        match self.refreshed_at {
            None => true,
            Some(instant) => instant.elapsed() >= CACHE_TTL,
        }
    }

    fn refresh(&mut self, app: &AppHandle, locale: &str) {
        self.application_items = collect_application_items(app, locale);
        self.file_items = collect_file_items(app, locale);
        self.refreshed_at = Some(Instant::now());
        self.locale = Some(locale.to_string());
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NavigatePayload {
    route: String,
}

fn launcher_cache() -> &'static Mutex<LauncherCache> {
    static CACHE: OnceLock<Mutex<LauncherCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(LauncherCache::new()))
}

pub fn invalidate_launcher_cache() {
    if let Ok(mut cache) = launcher_cache().lock() {
        cache.refreshed_at = None;
        cache.locale = None;
        cache.application_items.clear();
        cache.file_items.clear();
    }
}

fn current_locale(app: &AppHandle) -> ResolvedAppLocale {
    app.try_state::<AppState>()
        .map(|state| state.resolved_locale())
        .unwrap_or_else(|| DEFAULT_RESOLVED_LOCALE.to_string())
}

pub fn search_launcher(app: &AppHandle, query: &str, limit: Option<u16>) -> Vec<LauncherItemDto> {
    let normalized = normalize_query(query);
    let locale = current_locale(app);
    let mut items = builtin_items(&locale);

    if let Ok(mut cache) = launcher_cache().lock() {
        if cache.is_stale(&locale) {
            cache.refresh(app, &locale);
        }
        items.extend(cache.application_items.iter().cloned());
        items.extend(cache.file_items.iter().cloned());
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

    let limit = limit
        .map(usize::from)
        .unwrap_or(DEFAULT_RESULT_LIMIT)
        .clamp(1, MAX_RESULT_LIMIT);

    matched.truncate(limit);
    matched
}

fn should_hide_item_without_query(item: &LauncherItemDto) -> bool {
    matches!(&item.action, LauncherActionDto::OpenBuiltinTool { .. })
}

pub fn execute_launcher_action(app: &AppHandle, action: &LauncherActionDto) -> ActionResultDto {
    let result = match action {
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
                    if let Err(error) =
                        crate::apply_clipboard_window_mode(app, false, "launcher_open")
                    {
                        tracing::warn!(
                            event = "clipboard_window_mode_apply_failed",
                            source = "launcher_open",
                            compact = false,
                            error = error
                        );
                    }
                    if let Some(state) = app.try_state::<AppState>() {
                        state.set_clipboard_window_compact(false);
                    }
                    app.emit(
                        "rtool://clipboard-window/opened",
                        ClipboardWindowOpenedPayload { compact: false },
                    )
                    .map_err(|error| error.to_string())?;
                }
                Ok(format!("window:{window_label}"))
            }),
        LauncherActionDto::OpenFile { path } | LauncherActionDto::OpenApplication { path } => {
            open_path(Path::new(path)).map(|_| format!("path:{path}"))
        }
    };

    match result {
        Ok(message) => ActionResultDto { ok: true, message },
        Err(message) => ActionResultDto { ok: false, message },
    }
}

pub fn search_palette_legacy(app: &AppHandle, query: &str) -> Vec<LauncherItemDto> {
    let normalized = normalize_query(query);
    let locale = current_locale(app);
    let items = vec![
        LauncherItemDto {
            id: "action.open-tools".to_string(),
            title: t(&locale, "launcher.legacy.tools.title"),
            subtitle: t(&locale, "launcher.legacy.tools.subtitle"),
            category: "action".to_string(),
            source: Some(t(&locale, "launcher.source.builtin")),
            shortcut: None,
            score: 0,
            icon_kind: "iconify".to_string(),
            icon_value: "i-noto:hammer-and-wrench".to_string(),
            action: LauncherActionDto::OpenBuiltinRoute {
                route: "/tools".to_string(),
            },
        },
        LauncherItemDto {
            id: "action.open-home".to_string(),
            title: t(&locale, "launcher.legacy.dashboard.title"),
            subtitle: t(&locale, "launcher.legacy.dashboard.subtitle"),
            category: "action".to_string(),
            source: Some(t(&locale, "launcher.source.builtin")),
            shortcut: None,
            score: 0,
            icon_kind: "iconify".to_string(),
            icon_value: "i-noto:desktop-computer".to_string(),
            action: LauncherActionDto::OpenBuiltinRoute {
                route: "/".to_string(),
            },
        },
    ];

    if normalized.is_empty() {
        return items;
    }

    items
        .into_iter()
        .filter(|item| {
            let title_score = calculate_match_score(&item.title, &normalized);
            let subtitle_score = calculate_match_score(&item.subtitle, &normalized);
            let alias_score = calculate_alias_score(item, &normalized, &locale);
            title_score > 0 || subtitle_score > 0 || alias_score > 0
        })
        .collect()
}

pub fn execute_palette_legacy(action_id: &str) -> ActionResultDto {
    let message = match action_id {
        "action.open-tools" => "route:/tools",
        "action.open-transfer" => "route:/transfer",
        "action.open-home" => "route:/",
        "builtin.tools" => "route:/tools",
        "builtin.transfer" => "route:/transfer",
        "builtin.dashboard" => "route:/",
        _ => "unsupported_action",
    };

    ActionResultDto {
        ok: message != "unsupported_action",
        message: message.to_string(),
    }
}

fn open_main_with_route(app: &AppHandle, route: String) -> Result<(), String> {
    open_window(app, "main")?;
    app.emit("rtool://main/navigate", NavigatePayload { route })
        .map_err(|error| error.to_string())
}

fn open_window(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("window_not_found:{label}"))?;

    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

fn open_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("路径不存在: {}", path.to_string_lossy()));
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
    .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("打开失败: {}", status))
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
        "file" => 120,
        _ => 80,
    }
}

fn category_rank(category: &str) -> i32 {
    match category {
        "builtin" => 0,
        "application" => 1,
        "file" => 2,
        _ => 3,
    }
}

fn collect_application_items(app: &AppHandle, locale: &str) -> Vec<LauncherItemDto> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for root in application_roots() {
        if items.len() >= MAX_APP_ITEMS {
            break;
        }

        scan_root(&root, APP_SCAN_DEPTH, MAX_APP_ITEMS, |path, is_dir| {
            if is_application_candidate(path, is_dir) {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .into_iter()
        .for_each(|path| {
            if items.len() >= MAX_APP_ITEMS {
                return;
            }

            let key = path.to_string_lossy().to_string();
            if !seen.insert(key.clone()) {
                return;
            }

            let title = application_title(&path);
            let icon = resolve_application_icon(app, &path);
            items.push(LauncherItemDto {
                id: stable_id("app", &key),
                title,
                subtitle: key.clone(),
                category: "application".to_string(),
                source: Some(t(locale, "launcher.source.application")),
                shortcut: None,
                score: 0,
                icon_kind: icon.kind,
                icon_value: icon.value,
                action: LauncherActionDto::OpenApplication { path: key },
            });
        });
    }

    items
}

fn collect_file_items(app: &AppHandle, locale: &str) -> Vec<LauncherItemDto> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for root in file_roots() {
        if items.len() >= MAX_FILE_ITEMS {
            break;
        }

        scan_root(&root, FILE_SCAN_DEPTH, MAX_FILE_ITEMS, |path, is_dir| {
            if is_dir || is_hidden(path) {
                return None;
            }
            Some(path.to_path_buf())
        })
        .into_iter()
        .for_each(|path| {
            if items.len() >= MAX_FILE_ITEMS {
                return;
            }

            let key = path.to_string_lossy().to_string();
            if !seen.insert(key.clone()) {
                return;
            }

            let title = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| key.clone());

            let subtitle = path
                .parent()
                .map(|parent| parent.to_string_lossy().to_string())
                .unwrap_or_else(|| key.clone());

            let icon = resolve_file_type_icon(app, &path);
            items.push(LauncherItemDto {
                id: stable_id("file", &key),
                title,
                subtitle,
                category: "file".to_string(),
                source: Some(t(locale, "launcher.source.file")),
                shortcut: None,
                score: 0,
                icon_kind: icon.kind,
                icon_value: icon.value,
                action: LauncherActionDto::OpenFile { path: key },
            });
        });
    }

    items
}

fn scan_root<F>(root: &Path, max_depth: usize, max_items: usize, mut matcher: F) -> Vec<PathBuf>
where
    F: FnMut(&Path, bool) -> Option<PathBuf>,
{
    if !root.exists() {
        return Vec::new();
    }

    let mut queue = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0usize));

    let mut results = Vec::new();
    while let Some((current_dir, depth)) = queue.pop_front() {
        if results.len() >= max_items {
            break;
        }

        let entries = match fs::read_dir(&current_dir) {
            Ok(entries) => entries,
            Err(error) => {
                tracing::debug!(
                    event = "launcher_scan_read_dir_failed",
                    dir = %current_dir.to_string_lossy(),
                    error = error.to_string()
                );
                continue;
            }
        };

        for entry in entries.flatten() {
            if results.len() >= max_items {
                break;
            }

            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    tracing::debug!(
                        event = "launcher_scan_file_type_failed",
                        path = %path.to_string_lossy(),
                        error = error.to_string()
                    );
                    continue;
                }
            };

            let is_dir = file_type.is_dir();
            if let Some(matched_path) = matcher(&path, is_dir) {
                results.push(matched_path);
            }

            if is_dir {
                if is_hidden(&path) {
                    continue;
                }

                if cfg!(target_os = "macos")
                    && path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
                {
                    continue;
                }

                if depth < max_depth {
                    queue.push_back((path, depth + 1));
                }
            }
        }
    }

    results
}

fn application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if cfg!(target_os = "macos") {
        roots.push(PathBuf::from("/Applications"));
        if let Some(home) = home_dir() {
            roots.push(home.join("Applications"));
        }
        return roots;
    }

    if cfg!(target_os = "windows") {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            roots.push(PathBuf::from(app_data).join("Microsoft/Windows/Start Menu/Programs"));
        }
        if let Some(program_data) = std::env::var_os("ProgramData") {
            roots.push(PathBuf::from(program_data).join("Microsoft/Windows/Start Menu/Programs"));
        }
        return roots;
    }

    roots.push(PathBuf::from("/usr/share/applications"));
    roots.push(PathBuf::from("/usr/local/share/applications"));
    if let Some(home) = home_dir() {
        roots.push(home.join(".local/share/applications"));
    }

    roots
}

fn file_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = home_dir() {
        roots.push(home.join("Desktop"));
        roots.push(home.join("Documents"));
        roots.push(home.join("Downloads"));
    }
    roots
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn is_application_candidate(path: &Path, is_dir: bool) -> bool {
    if cfg!(target_os = "macos") {
        return is_dir
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("app"));
    }

    if cfg!(target_os = "windows") {
        return path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "lnk" | "exe" | "url" | "appref-ms"
                )
            });
    }

    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
}

fn application_title(path: &Path) -> String {
    if cfg!(target_os = "linux")
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
    {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if let Some(value) = line.strip_prefix("Name=") {
                    let title = value.trim();
                    if !title.is_empty() {
                        return title.to_string();
                    }
                }
            }
        }
    }

    path.file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn stable_id(prefix: &str, input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{prefix}.{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
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
        let file = category_weight("file");
        assert!(builtin > file);
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
}

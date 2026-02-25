use crate::host::LauncherHost;
use crate::launcher::icon::{
    resolve_application_icon, resolve_builtin_icon, resolve_file_type_icon,
};
use crate::launcher::index::search_indexed_items_async;
use anyhow::Context;
use app_core::i18n::{DEFAULT_RESOLVED_LOCALE, ResolvedAppLocale, t};
use app_core::models::{ClipboardWindowOpenedPayload, LauncherActionDto, LauncherItemDto};
use app_core::{AppError, AppResult, ResultExt};
use app_infra::db::DbConn;
use serde::Serialize;
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(30);
const DEFAULT_RESULT_LIMIT: usize = 60;
const MAX_RESULT_LIMIT: usize = 120;
const MAX_APP_ITEMS: usize = 300;
const MAX_FILE_ITEMS: usize = 2_000;
const APP_SCAN_DEPTH: usize = 4;
const FILE_SCAN_DEPTH: usize = 12;
const FALLBACK_FILE_ITEMS_LIMIT: usize = 800;
const FALLBACK_SCAN_DEPTH: usize = 6;
const SCAN_WARNING_SAMPLE_LIMIT: usize = 5;

#[derive(Debug, Default, Clone)]
struct ScanWarningAggregator {
    read_dir_failed: u64,
    read_dir_entry_failed: u64,
    file_type_failed: u64,
    metadata_failed: u64,
    read_dir_samples: Vec<String>,
    read_dir_entry_samples: Vec<String>,
    file_type_samples: Vec<String>,
    metadata_samples: Vec<String>,
}

impl ScanWarningAggregator {
    fn record_read_dir_failed(&mut self, path: &Path) {
        self.read_dir_failed = self.read_dir_failed.saturating_add(1);
        push_scan_warning_sample(&mut self.read_dir_samples, path);
    }

    fn record_read_dir_entry_failed(&mut self, path: &Path) {
        self.read_dir_entry_failed = self.read_dir_entry_failed.saturating_add(1);
        push_scan_warning_sample(&mut self.read_dir_entry_samples, path);
    }

    fn record_file_type_failed(&mut self, path: &Path) {
        self.file_type_failed = self.file_type_failed.saturating_add(1);
        push_scan_warning_sample(&mut self.file_type_samples, path);
    }

    fn total_warnings(&self) -> u64 {
        self.read_dir_failed
            .saturating_add(self.read_dir_entry_failed)
            .saturating_add(self.file_type_failed)
            .saturating_add(self.metadata_failed)
    }

    fn log_summary(&self, root: &Path) {
        let total_warnings = self.total_warnings();
        if total_warnings == 0 {
            return;
        }

        tracing::info!(
            event = "launcher_scan_warning_summary",
            root = %root.to_string_lossy(),
            reason = "interactive",
            total_warnings,
            read_dir_failed = self.read_dir_failed,
            read_dir_entry_failed = self.read_dir_entry_failed,
            file_type_failed = self.file_type_failed,
            metadata_failed = self.metadata_failed,
            read_dir_samples = self.read_dir_samples.join(" | "),
            read_dir_entry_samples = self.read_dir_entry_samples.join(" | "),
            file_type_samples = self.file_type_samples.join(" | "),
            metadata_samples = self.metadata_samples.join(" | "),
        );
    }
}

fn push_scan_warning_sample(samples: &mut Vec<String>, path: &Path) {
    if samples.len() >= SCAN_WARNING_SAMPLE_LIMIT {
        return;
    }
    samples.push(path.to_string_lossy().to_string());
}

struct LauncherCache {
    refreshed_at: Option<Instant>,
    locale: Option<String>,
    application_items: Vec<LauncherItemDto>,
}

impl LauncherCache {
    fn new() -> Self {
        Self {
            refreshed_at: None,
            locale: None,
            application_items: Vec::new(),
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

    fn refresh(&mut self, app: &dyn LauncherHost, locale: &str) {
        self.application_items = collect_application_items(app, locale);
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
    }
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
    pub fallback_used: bool,
    pub cache_refreshed: bool,
    pub index_query_duration_ms: Option<u64>,
    pub fallback_scan_duration_ms: Option<u64>,
    pub cache_refresh_duration_ms: Option<u64>,
}

fn elapsed_ms(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn force_fallback_scan() -> bool {
    std::env::var("RTOOL_LAUNCHER_FORCE_FALLBACK")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
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

    if let Ok(mut cache) = launcher_cache().lock() {
        if cache.is_stale(&locale) {
            let cache_refresh_started_at = Instant::now();
            cache.refresh(app, &locale);
            diagnostics.cache_refreshed = true;
            diagnostics.cache_refresh_duration_ms = Some(elapsed_ms(cache_refresh_started_at));
        }
        items.extend(cache.application_items.iter().cloned());
    }

    if !force_fallback_scan() {
        let index_started_at = Instant::now();
        match search_indexed_items_async(app, db_conn, &normalized, &locale, result_limit).await {
            Ok(index_result) => {
                diagnostics.index_query_duration_ms = Some(elapsed_ms(index_started_at));
                if index_result.ready {
                    diagnostics.index_used = true;
                    items.extend(index_result.items);
                } else {
                    tracing::info!(event = "launcher_index_not_ready_fallback_scan");
                }
            }
            Err(error) => {
                diagnostics.index_query_duration_ms = Some(elapsed_ms(index_started_at));
                diagnostics.index_failed = true;
                tracing::warn!(
                    event = "launcher_index_query_failed_fallback_scan",
                    error = error.to_string()
                );
            }
        }
    }

    if !diagnostics.index_used {
        let fallback_started_at = Instant::now();
        diagnostics.fallback_used = true;
        tracing::debug!(event = "launcher_fallback_scan_used");
        items.extend(collect_file_items_with_limits(
            app,
            &locale,
            FALLBACK_SCAN_DEPTH,
            FALLBACK_FILE_ITEMS_LIMIT,
        ));
        diagnostics.fallback_scan_duration_ms = Some(elapsed_ms(fallback_started_at));
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

fn collect_application_items(app: &dyn LauncherHost, locale: &str) -> Vec<LauncherItemDto> {
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

fn collect_file_items_with_limits(
    app: &dyn LauncherHost,
    locale: &str,
    scan_depth: usize,
    max_items: usize,
) -> Vec<LauncherItemDto> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let max_items = max_items.max(1).min(MAX_FILE_ITEMS);
    let scan_depth = scan_depth.max(1).min(FILE_SCAN_DEPTH);

    for root in file_roots() {
        if items.len() >= max_items {
            break;
        }

        scan_root(&root, scan_depth, max_items, |path, _is_dir| {
            if is_hidden(path) {
                return None;
            }
            Some(path.to_path_buf())
        })
        .into_iter()
        .for_each(|path| {
            if items.len() >= max_items {
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

            let is_directory = path.is_dir();
            let (category, source, action, icon_kind, icon_value) = if is_directory {
                let icon = resolve_builtin_icon("i-noto:file-folder");
                (
                    "directory".to_string(),
                    t(locale, "launcher.source.directory"),
                    LauncherActionDto::OpenDirectory { path: key.clone() },
                    icon.kind,
                    icon.value,
                )
            } else {
                let icon = resolve_file_type_icon(app, &path);
                (
                    "file".to_string(),
                    t(locale, "launcher.source.file"),
                    LauncherActionDto::OpenFile { path: key.clone() },
                    icon.kind,
                    icon.value,
                )
            };

            items.push(LauncherItemDto {
                id: stable_id(if is_directory { "dir" } else { "file" }, &key),
                title,
                subtitle,
                category,
                source: Some(source),
                shortcut: None,
                score: 0,
                icon_kind,
                icon_value,
                action,
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
    let mut warning_aggregator = ScanWarningAggregator::default();
    while let Some((current_dir, depth)) = queue.pop_front() {
        if results.len() >= max_items {
            break;
        }

        let entries = match fs::read_dir(&current_dir) {
            Ok(entries) => entries,
            Err(_error) => {
                warning_aggregator.record_read_dir_failed(current_dir.as_path());
                continue;
            }
        };

        for entry in entries {
            if results.len() >= max_items {
                break;
            }

            let entry = match entry {
                Ok(entry) => entry,
                Err(_error) => {
                    warning_aggregator.record_read_dir_entry_failed(current_dir.as_path());
                    continue;
                }
            };

            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_error) => {
                    warning_aggregator.record_file_type_failed(path.as_path());
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

    warning_aggregator.log_summary(root);
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
        roots.push(home);
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
        && let Ok(content) = fs::read_to_string(path)
    {
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("Name=") {
                let title = value.trim();
                if !title.is_empty() {
                    return title.to_string();
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
#[path = "../../tests/launcher/launcher_service_tests.rs"]
mod tests;

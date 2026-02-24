use app_core::i18n::t;
use app_core::models::{
    LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto, LauncherRebuildResultDto,
    LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto,
};
use app_core::{AppError, AppResult};
use crate::host::LauncherHost;
use crate::launcher::icon::{resolve_builtin_icon, resolve_file_type_icon};
use app_infra::db::{DbPool, get_app_setting, set_app_setting};
use regex::Regex;
use rusqlite::{OptionalExtension, params, params_from_iter};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const INDEX_READY_KEY: &str = "launcher.index.ready";
const INDEX_LAST_BUILD_MS_KEY: &str = "launcher.index.lastBuildMs";
const INDEX_LAST_DURATION_MS_KEY: &str = "launcher.index.lastDurationMs";
const INDEX_LAST_ITEM_COUNT_KEY: &str = "launcher.index.lastItemCount";
const INDEX_LAST_ROOT_COUNT_KEY: &str = "launcher.index.lastRootCount";
const INDEX_LAST_TRUNCATED_KEY: &str = "launcher.index.lastTruncated";
const INDEX_VERSION_KEY: &str = "launcher.index.version";
const INDEX_LAST_ERROR_KEY: &str = "launcher.index.lastError";
const INDEX_VERSION_VALUE: &str = "2";
const SEARCH_SETTINGS_KEY: &str = "launcher.search.settings";
const LAUNCHER_SCOPE_POLICY_APPLIED_KEY: &str = "launcher.search.scope_policy_applied";
const LAUNCHER_SCOPE_POLICY_APPLIED_VALUE: &str = "applied";

const DEFAULT_MAX_SCAN_DEPTH: u32 = 20;
const DEFAULT_MAX_ITEMS_PER_ROOT: u32 = 200_000;
const DEFAULT_MAX_TOTAL_ITEMS: u32 = 500_000;
const DEFAULT_REFRESH_INTERVAL_SECS: u32 = 600;

const MIN_SCAN_DEPTH: u32 = 2;
const MAX_SCAN_DEPTH: u32 = 32;
const MIN_ITEMS_PER_ROOT: u32 = 500;
const MAX_ITEMS_PER_ROOT: u32 = 1_000_000;
const MIN_TOTAL_ITEMS: u32 = 2_000;
const MAX_TOTAL_ITEMS: u32 = 2_000_000;
const MIN_REFRESH_INTERVAL_SECS: u32 = 60;
const MAX_REFRESH_INTERVAL_SECS: u32 = 86_400;

const QUERY_OVERSCAN_FACTOR: usize = 4;
const MAX_QUERY_CANDIDATE_LIMIT: usize = 1_000;
const SCAN_YIELD_EVERY: usize = 4_000;
const SCAN_YIELD_SLEEP: Duration = Duration::from_millis(2);
const SCAN_WARNING_SAMPLE_LIMIT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopePlatform {
    Macos,
    Windows,
    Linux,
}

fn current_scope_platform() -> ScopePlatform {
    if cfg!(target_os = "macos") {
        return ScopePlatform::Macos;
    }
    if cfg!(target_os = "windows") {
        return ScopePlatform::Windows;
    }
    ScopePlatform::Linux
}

fn scope_platform_name(platform: ScopePlatform) -> &'static str {
    match platform {
        ScopePlatform::Macos => "macos",
        ScopePlatform::Windows => "windows",
        ScopePlatform::Linux => "linux",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanWarningKind {
    ReadDir,
    ReadDirEntry,
    FileType,
    Metadata,
}

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
    fn record(&mut self, kind: ScanWarningKind, path: &Path) {
        let path_text = path.to_string_lossy().to_string();
        match kind {
            ScanWarningKind::ReadDir => {
                self.read_dir_failed = self.read_dir_failed.saturating_add(1);
                push_scan_warning_sample(&mut self.read_dir_samples, path_text);
            }
            ScanWarningKind::ReadDirEntry => {
                self.read_dir_entry_failed = self.read_dir_entry_failed.saturating_add(1);
                push_scan_warning_sample(&mut self.read_dir_entry_samples, path_text);
            }
            ScanWarningKind::FileType => {
                self.file_type_failed = self.file_type_failed.saturating_add(1);
                push_scan_warning_sample(&mut self.file_type_samples, path_text);
            }
            ScanWarningKind::Metadata => {
                self.metadata_failed = self.metadata_failed.saturating_add(1);
                push_scan_warning_sample(&mut self.metadata_samples, path_text);
            }
        }
    }

    fn total_warnings(&self) -> u64 {
        self.read_dir_failed
            .saturating_add(self.read_dir_entry_failed)
            .saturating_add(self.file_type_failed)
            .saturating_add(self.metadata_failed)
    }

    fn log_summary(&self, event_name: &str, root: &str, reason: &str) {
        let total_warnings = self.total_warnings();
        if total_warnings == 0 {
            return;
        }

        tracing::info!(
            event = event_name,
            root,
            reason,
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

fn push_scan_warning_sample(samples: &mut Vec<String>, value: String) {
    if samples.len() >= SCAN_WARNING_SAMPLE_LIMIT {
        return;
    }
    samples.push(value);
}

static INDEXER_STARTED: OnceLock<AtomicBool> = OnceLock::new();
static INDEXER_STOPPED: OnceLock<AtomicBool> = OnceLock::new();
static INDEX_BUILDING: OnceLock<AtomicBool> = OnceLock::new();
static INDEX_REBUILD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn indexer_started_flag() -> &'static AtomicBool {
    INDEXER_STARTED.get_or_init(|| AtomicBool::new(false))
}

fn indexer_stopped_flag() -> &'static AtomicBool {
    INDEXER_STOPPED.get_or_init(|| AtomicBool::new(false))
}

fn index_building_flag() -> &'static AtomicBool {
    INDEX_BUILDING.get_or_init(|| AtomicBool::new(false))
}

fn index_rebuild_lock() -> &'static Mutex<()> {
    INDEX_REBUILD_LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone)]
struct LauncherIndexEntry {
    path: String,
    kind: IndexedEntryKind,
    name: String,
    parent: String,
    ext: Option<String>,
    mtime: Option<i64>,
    size: Option<i64>,
    source_root: String,
    searchable_text: String,
}

#[derive(Debug, Clone)]
struct ScanOutcome {
    entries: Vec<LauncherIndexEntry>,
    truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TruncationLogLevel {
    Info,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct LauncherSearchSettingsRecord {
    roots: Vec<String>,
    exclude_patterns: Vec<String>,
    max_scan_depth: u32,
    max_items_per_root: u32,
    max_total_items: u32,
    refresh_interval_secs: u32,
}

impl Default for LauncherSearchSettingsRecord {
    fn default() -> Self {
        Self {
            roots: default_search_roots(),
            exclude_patterns: default_exclude_patterns(),
            max_scan_depth: DEFAULT_MAX_SCAN_DEPTH,
            max_items_per_root: DEFAULT_MAX_ITEMS_PER_ROOT,
            max_total_items: DEFAULT_MAX_TOTAL_ITEMS,
            refresh_interval_secs: DEFAULT_REFRESH_INTERVAL_SECS,
        }
    }
}

impl LauncherSearchSettingsRecord {
    fn normalize(mut self) -> Self {
        self.roots = sanitize_roots(self.roots);
        if self.roots.is_empty() {
            self.roots = default_search_roots();
        }
        self.exclude_patterns = sanitize_patterns(self.exclude_patterns);
        self.max_scan_depth = self.max_scan_depth.clamp(MIN_SCAN_DEPTH, MAX_SCAN_DEPTH);
        self.max_items_per_root = self
            .max_items_per_root
            .clamp(MIN_ITEMS_PER_ROOT, MAX_ITEMS_PER_ROOT);
        self.max_total_items = self.max_total_items.clamp(MIN_TOTAL_ITEMS, MAX_TOTAL_ITEMS);
        self.refresh_interval_secs = self
            .refresh_interval_secs
            .clamp(MIN_REFRESH_INTERVAL_SECS, MAX_REFRESH_INTERVAL_SECS);
        self
    }

    fn to_dto(&self) -> LauncherSearchSettingsDto {
        LauncherSearchSettingsDto {
            roots: self.roots.clone(),
            exclude_patterns: self.exclude_patterns.clone(),
            max_scan_depth: self.max_scan_depth,
            max_items_per_root: self.max_items_per_root,
            max_total_items: self.max_total_items,
            refresh_interval_secs: self.refresh_interval_secs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexedSearchResult {
    pub items: Vec<LauncherItemDto>,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndexedEntryKind {
    Directory,
    File,
}

impl IndexedEntryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Directory => "directory",
            Self::File => "file",
        }
    }

    fn from_db(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("directory") {
            return Some(Self::Directory);
        }
        if value.eq_ignore_ascii_case("file") {
            return Some(Self::File);
        }
        None
    }
}

#[derive(Debug, Clone)]
enum ExclusionRule {
    Segment(String),
    Prefix(String),
    Subpath(String),
    Wildcard(Regex),
}

#[derive(Debug, Clone, Copy)]
enum RefreshReason {
    Startup,
    Periodic,
    Manual,
}

impl RefreshReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Periodic => "periodic",
            Self::Manual => "manual",
        }
    }
}

fn is_default_scope_profile(settings: &LauncherSearchSettingsRecord) -> bool {
    settings.roots == default_search_roots()
        && settings.max_scan_depth == DEFAULT_MAX_SCAN_DEPTH
        && settings.max_items_per_root == DEFAULT_MAX_ITEMS_PER_ROOT
        && settings.max_total_items == DEFAULT_MAX_TOTAL_ITEMS
        && settings.refresh_interval_secs == DEFAULT_REFRESH_INTERVAL_SECS
        && settings.exclude_patterns == default_exclude_patterns()
}

fn has_single_system_root_scope(settings: &LauncherSearchSettingsRecord) -> bool {
    if settings.roots.len() != 1 {
        return false;
    }
    let normalized = normalize_path_for_match(Path::new(settings.roots[0].as_str()));
    normalized == "/" || is_windows_drive_root(normalized.as_str())
}

fn resolve_effective_max_items_per_root(
    configured_max_items_per_root: usize,
    remaining_total: usize,
    single_system_root_scope: bool,
) -> usize {
    if single_system_root_scope {
        return remaining_total.max(1);
    }
    configured_max_items_per_root.max(1)
}

fn classify_truncation_log_level(settings: &LauncherSearchSettingsRecord) -> TruncationLogLevel {
    if is_default_scope_profile(settings) {
        return TruncationLogLevel::Info;
    }
    TruncationLogLevel::Warn
}

fn log_scan_truncation(
    settings: &LauncherSearchSettingsRecord,
    reason: RefreshReason,
    root: &str,
    effective_max_items: usize,
    indexed_items: usize,
) {
    let configured_max_items_per_root = settings.max_items_per_root as usize;
    let max_total_items = settings.max_total_items as usize;
    match classify_truncation_log_level(settings) {
        TruncationLogLevel::Info => {
            tracing::info!(
                event = "launcher_index_scan_truncated_expected",
                root,
                effective_max_items,
                configured_max_items_per_root,
                max_total_items,
                indexed_items,
                reason = reason.as_str()
            );
        }
        TruncationLogLevel::Warn => {
            tracing::warn!(
                event = "launcher_index_scan_truncated_unexpected",
                root,
                effective_max_items,
                configured_max_items_per_root,
                max_total_items,
                indexed_items,
                reason = reason.as_str()
            );
        }
    }
}

pub fn start_background_indexer(db_pool: DbPool) {
    let started = indexer_started_flag();
    let stopped = indexer_stopped_flag();
    if started.swap(true, Ordering::SeqCst) {
        return;
    }
    stopped.store(false, Ordering::SeqCst);

    let spawn_result = thread::Builder::new()
        .name("launcher-indexer".to_string())
        .spawn(move || {
            index_building_flag().store(true, Ordering::SeqCst);
            let initial_result = refresh_index(&db_pool, RefreshReason::Startup);
            index_building_flag().store(false, Ordering::SeqCst);
            if let Err(error) = initial_result {
                let _ = write_meta(&db_pool, INDEX_READY_KEY, "0");
                let _ = write_meta(&db_pool, INDEX_LAST_ERROR_KEY, error.to_string().as_str());
                tracing::warn!(
                    event = "launcher_index_initial_build_failed",
                    error = error.to_string()
                );
            }

            loop {
                if wait_for_next_refresh(&db_pool, stopped) {
                    break;
                }
                if let Err(error) = refresh_index(&db_pool, RefreshReason::Periodic) {
                    let _ = write_meta(&db_pool, INDEX_LAST_ERROR_KEY, error.to_string().as_str());
                    tracing::warn!(
                        event = "launcher_index_periodic_refresh_failed",
                        error = error.to_string()
                    );
                }
            }
        });

    if let Err(error) = spawn_result {
        started.store(false, Ordering::SeqCst);
        tracing::error!(
            event = "launcher_indexer_spawn_failed",
            error = error.to_string()
        );
        return;
    }

    tracing::info!(event = "launcher_indexer_started");
}

pub fn stop_background_indexer() {
    let started = indexer_started_flag();
    let stopped = indexer_stopped_flag();
    if !started.load(Ordering::SeqCst) {
        return;
    }
    stopped.store(true, Ordering::SeqCst);
    started.store(false, Ordering::SeqCst);
}

pub fn get_search_settings(db_pool: &DbPool) -> AppResult<LauncherSearchSettingsDto> {
    let settings = load_or_init_settings(db_pool)?;
    Ok(settings.to_dto())
}

pub fn update_search_settings(
    db_pool: &DbPool,
    input: LauncherUpdateSearchSettingsInputDto,
) -> AppResult<LauncherSearchSettingsDto> {
    let current = load_or_init_settings(db_pool)?;
    let next = LauncherSearchSettingsRecord {
        roots: input.roots.unwrap_or(current.roots),
        exclude_patterns: input.exclude_patterns.unwrap_or(current.exclude_patterns),
        max_scan_depth: input.max_scan_depth.unwrap_or(current.max_scan_depth),
        max_items_per_root: input
            .max_items_per_root
            .unwrap_or(current.max_items_per_root),
        max_total_items: input.max_total_items.unwrap_or(current.max_total_items),
        refresh_interval_secs: input
            .refresh_interval_secs
            .unwrap_or(current.refresh_interval_secs),
    }
    .normalize();

    save_settings(db_pool, &next)?;
    Ok(next.to_dto())
}

pub fn get_index_status(db_pool: &DbPool) -> AppResult<LauncherIndexStatusDto> {
    let ready = read_meta(db_pool, INDEX_READY_KEY)?
        .as_deref()
        .map(is_truthy_flag)
        .unwrap_or(false);
    let indexed_items = read_meta(db_pool, INDEX_LAST_ITEM_COUNT_KEY)?
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let indexed_roots = read_meta(db_pool, INDEX_LAST_ROOT_COUNT_KEY)?
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let last_build_ms =
        read_meta(db_pool, INDEX_LAST_BUILD_MS_KEY)?.and_then(|value| value.parse().ok());
    let last_duration_ms =
        read_meta(db_pool, INDEX_LAST_DURATION_MS_KEY)?.and_then(|value| value.parse::<u64>().ok());
    let last_error = read_meta(db_pool, INDEX_LAST_ERROR_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let index_version = read_meta(db_pool, INDEX_VERSION_KEY)?
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| INDEX_VERSION_VALUE.to_string());
    let truncated = read_meta(db_pool, INDEX_LAST_TRUNCATED_KEY)?
        .as_deref()
        .map(is_truthy_flag)
        .unwrap_or(false);

    let settings = load_or_init_settings(db_pool)?;
    Ok(LauncherIndexStatusDto {
        ready,
        building: index_building_flag().load(Ordering::SeqCst),
        indexed_items,
        indexed_roots,
        last_build_ms,
        last_duration_ms,
        last_error,
        refresh_interval_secs: settings.refresh_interval_secs,
        index_version,
        truncated,
    })
}

pub fn rebuild_index_now(db_pool: &DbPool) -> AppResult<LauncherRebuildResultDto> {
    let started_at = Instant::now();
    refresh_index(db_pool, RefreshReason::Manual)?;
    let duration_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
    let status = get_index_status(db_pool)?;
    Ok(LauncherRebuildResultDto {
        success: status.ready,
        duration_ms,
        indexed_items: status.indexed_items,
        indexed_roots: status.indexed_roots,
        truncated: status.truncated,
        ready: status.ready,
    })
}

pub fn search_indexed_items(
    app: &dyn LauncherHost,
    db_pool: &DbPool,
    normalized_query: &str,
    locale: &str,
    limit: usize,
) -> AppResult<IndexedSearchResult> {
    let ready = read_index_ready(db_pool)?;
    let limit = limit.max(1);
    let candidate_limit = (limit * QUERY_OVERSCAN_FACTOR).clamp(limit, MAX_QUERY_CANDIDATE_LIMIT);
    let conn = db_pool.get()?;

    let rows = if normalized_query.is_empty() {
        query_index_rows_default(&conn, candidate_limit as i64)?
    } else {
        let fts_query = build_fts_query(normalized_query);
        if let Some(fts_query) = fts_query {
            match query_index_rows_fts(&conn, fts_query.as_str(), candidate_limit as i64) {
                Ok(rows) => rows,
                Err(error) => {
                    tracing::warn!(
                        event = "launcher_index_fts_query_failed",
                        query = normalized_query,
                        error = error.to_string()
                    );
                    query_index_rows_like(&conn, normalized_query, candidate_limit as i64)?
                }
            }
        } else {
            query_index_rows_like(&conn, normalized_query, candidate_limit as i64)?
        }
    };

    let mut items = Vec::new();
    for (path, kind_raw, name, parent) in rows {
        let Some(kind) = IndexedEntryKind::from_db(kind_raw.as_str()) else {
            tracing::warn!(
                event = "launcher_index_unknown_entry_kind",
                kind = kind_raw.as_str(),
                path = path.as_str()
            );
            continue;
        };

        let title = if name.trim().is_empty() {
            path.clone()
        } else {
            name
        };
        let subtitle = if parent.trim().is_empty() {
            path.clone()
        } else {
            parent
        };

        let item = match kind {
            IndexedEntryKind::Directory => {
                let icon = resolve_builtin_icon("i-noto:file-folder");
                LauncherItemDto {
                    id: stable_id("dir", path.as_str()),
                    title,
                    subtitle,
                    category: "directory".to_string(),
                    source: Some(t(locale, "launcher.source.directory")),
                    shortcut: None,
                    score: 0,
                    icon_kind: icon.kind,
                    icon_value: icon.value,
                    action: LauncherActionDto::OpenDirectory { path },
                }
            }
            IndexedEntryKind::File => {
                let path_buf = PathBuf::from(path.as_str());
                let icon = resolve_file_type_icon(app, path_buf.as_path());
                LauncherItemDto {
                    id: stable_id("file", path.as_str()),
                    title,
                    subtitle,
                    category: "file".to_string(),
                    source: Some(t(locale, "launcher.source.file")),
                    shortcut: None,
                    score: 0,
                    icon_kind: icon.kind,
                    icon_value: icon.value,
                    action: LauncherActionDto::OpenFile { path },
                }
            }
        };

        items.push(item);
    }

    Ok(IndexedSearchResult { items, ready })
}

fn refresh_index(db_pool: &DbPool, reason: RefreshReason) -> AppResult<()> {
    let _lock_guard = index_rebuild_lock()
        .lock()
        .map_err(|_| AppError::new("launcher_index_lock_failed", "启动器索引构建锁异常"))?;
    index_building_flag().store(true, Ordering::SeqCst);

    let started_at = Instant::now();
    let result = refresh_index_inner(db_pool, reason, started_at);
    index_building_flag().store(false, Ordering::SeqCst);
    if let Err(error) = &result {
        if matches!(reason, RefreshReason::Startup) {
            let _ = write_meta(db_pool, INDEX_READY_KEY, "0");
        }
        let _ = write_meta(db_pool, INDEX_LAST_ERROR_KEY, error.to_string().as_str());
    }
    result
}

fn refresh_index_inner(
    db_pool: &DbPool,
    reason: RefreshReason,
    started_at: Instant,
) -> AppResult<()> {
    if matches!(reason, RefreshReason::Startup) {
        write_meta(db_pool, INDEX_READY_KEY, "0")?;
    }

    let settings = load_or_init_settings(db_pool)?;
    let exclusion_rules = build_exclusion_rules(settings.exclude_patterns.as_slice());
    let scan_token = now_unix_millis().to_string();
    write_meta(db_pool, INDEX_VERSION_KEY, INDEX_VERSION_VALUE)?;

    let single_system_root_scope = has_single_system_root_scope(&settings);
    let mut indexed_items: usize = 0;
    let mut indexed_roots: u32 = 0;
    let mut truncated = false;
    let mut remaining_total = settings.max_total_items as usize;

    for root in &settings.roots {
        if remaining_total == 0 {
            truncated = true;
            break;
        }

        let root_path = PathBuf::from(root);
        if !root_path.exists() {
            continue;
        }

        let configured_max_items_per_root = settings.max_items_per_root as usize;
        let effective_max_items_per_root = resolve_effective_max_items_per_root(
            configured_max_items_per_root,
            remaining_total,
            single_system_root_scope,
        );
        let effective_max_items = effective_max_items_per_root
            .max(1)
            .min(remaining_total.max(1));

        indexed_roots += 1;
        let outcome = scan_index_root_with_rules(
            root_path.as_path(),
            settings.max_scan_depth as usize,
            effective_max_items_per_root,
            remaining_total,
            exclusion_rules.as_slice(),
            root,
            reason,
        );

        indexed_items = indexed_items.saturating_add(outcome.entries.len());
        remaining_total = remaining_total.saturating_sub(outcome.entries.len());
        truncated |= outcome.truncated;
        if outcome.truncated {
            log_scan_truncation(
                &settings,
                reason,
                root.as_str(),
                effective_max_items,
                indexed_items,
            );
        }

        upsert_entries_batched(db_pool, outcome.entries.as_slice(), scan_token.as_str())?;
        delete_stale_entries_for_root(db_pool, root.as_str(), scan_token.as_str())?;
    }

    purge_removed_roots(db_pool, settings.roots.as_slice())?;

    let duration_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
    write_meta(db_pool, INDEX_READY_KEY, "1")?;
    write_meta(
        db_pool,
        INDEX_LAST_BUILD_MS_KEY,
        now_unix_millis().to_string().as_str(),
    )?;
    write_meta(
        db_pool,
        INDEX_LAST_DURATION_MS_KEY,
        duration_ms.to_string().as_str(),
    )?;
    write_meta(
        db_pool,
        INDEX_LAST_ITEM_COUNT_KEY,
        indexed_items.to_string().as_str(),
    )?;
    write_meta(
        db_pool,
        INDEX_LAST_ROOT_COUNT_KEY,
        indexed_roots.to_string().as_str(),
    )?;
    write_meta(
        db_pool,
        INDEX_LAST_TRUNCATED_KEY,
        if truncated { "1" } else { "0" },
    )?;
    write_meta(db_pool, INDEX_LAST_ERROR_KEY, "")?;

    tracing::info!(
        event = "launcher_index_refresh_finished",
        reason = reason.as_str(),
        indexed_items,
        indexed_roots,
        truncated,
        duration_ms
    );
    Ok(())
}

fn upsert_entries_batched(
    db_pool: &DbPool,
    entries: &[LauncherIndexEntry],
    scan_token: &str,
) -> AppResult<()> {
    const UPSERT_BATCH_SIZE: usize = 2_000;
    for chunk in entries.chunks(UPSERT_BATCH_SIZE) {
        let mut conn = db_pool.get()?;
        let transaction = conn.transaction()?;
        upsert_entries(&transaction, chunk, scan_token)?;
        transaction.commit()?;
    }
    Ok(())
}

fn delete_stale_entries_for_root(db_pool: &DbPool, root: &str, scan_token: &str) -> AppResult<()> {
    let conn = db_pool.get()?;
    conn.execute(
        "DELETE FROM launcher_index_entries
         WHERE source_root = ?1
           AND COALESCE(scan_token, '') <> ?2",
        params![root, scan_token],
    )?;
    Ok(())
}

fn wait_for_next_refresh(db_pool: &DbPool, stopped: &AtomicBool) -> bool {
    let refresh_interval_secs = load_or_init_settings(db_pool)
        .map(|value| value.refresh_interval_secs)
        .unwrap_or(DEFAULT_REFRESH_INTERVAL_SECS);
    let target_ms = i64::from(refresh_interval_secs).saturating_mul(1000);
    let poll_ms = 1_000_i64;
    let mut elapsed = 0_i64;
    while elapsed < target_ms {
        if stopped.load(Ordering::SeqCst) {
            return true;
        }
        thread::sleep(Duration::from_millis(poll_ms as u64));
        elapsed += poll_ms;
    }
    stopped.load(Ordering::SeqCst)
}

fn upsert_entries(
    transaction: &rusqlite::Transaction<'_>,
    entries: &[LauncherIndexEntry],
    scan_token: &str,
) -> AppResult<()> {
    for entry in entries {
        transaction.execute(
            r#"
            INSERT INTO launcher_index_entries (
                path,
                kind,
                name,
                parent,
                ext,
                mtime,
                size,
                source_root,
                searchable_text,
                scan_token
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(path) DO UPDATE SET
                kind = excluded.kind,
                name = excluded.name,
                parent = excluded.parent,
                ext = excluded.ext,
                mtime = excluded.mtime,
                size = excluded.size,
                source_root = excluded.source_root,
                searchable_text = excluded.searchable_text,
                scan_token = excluded.scan_token
            "#,
            params![
                entry.path,
                entry.kind.as_str(),
                entry.name,
                entry.parent,
                entry.ext,
                entry.mtime,
                entry.size,
                entry.source_root,
                entry.searchable_text,
                scan_token,
            ],
        )?;
    }
    Ok(())
}

fn purge_removed_roots(db_pool: &DbPool, roots: &[String]) -> AppResult<()> {
    let conn = db_pool.get()?;
    if roots.is_empty() {
        conn.execute("DELETE FROM launcher_index_entries", [])?;
        return Ok(());
    }

    let placeholders = (1..=roots.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql =
        format!("DELETE FROM launcher_index_entries WHERE source_root NOT IN ({placeholders})");
    conn.execute(sql.as_str(), params_from_iter(roots.iter()))?;
    Ok(())
}

fn query_index_rows_default(
    conn: &rusqlite::Connection,
    limit: i64,
) -> AppResult<Vec<(String, String, String, String)>> {
    let mut statement = conn.prepare(
        r#"
        SELECT path, kind, name, parent
        FROM launcher_index_entries
        ORDER BY
            CASE kind
                WHEN 'directory' THEN 0
                WHEN 'file' THEN 1
                ELSE 2
            END ASC,
            name COLLATE NOCASE ASC,
            path COLLATE NOCASE ASC
        LIMIT ?1
        "#,
    )?;
    read_rows(statement.query(params![limit])?)
}

fn query_index_rows_fts(
    conn: &rusqlite::Connection,
    fts_query: &str,
    limit: i64,
) -> AppResult<Vec<(String, String, String, String)>> {
    let mut statement = conn.prepare(
        r#"
        SELECT e.path, e.kind, e.name, e.parent
        FROM launcher_index_entries_fts f
        JOIN launcher_index_entries e ON e.rowid = f.rowid
        WHERE launcher_index_entries_fts MATCH ?1
        ORDER BY
            CASE e.kind
                WHEN 'directory' THEN 0
                WHEN 'file' THEN 1
                ELSE 2
            END ASC,
            bm25(launcher_index_entries_fts) ASC,
            e.name COLLATE NOCASE ASC,
            e.path COLLATE NOCASE ASC
        LIMIT ?2
        "#,
    )?;
    read_rows(statement.query(params![fts_query, limit])?)
}

fn query_index_rows_like(
    conn: &rusqlite::Connection,
    normalized_query: &str,
    limit: i64,
) -> AppResult<Vec<(String, String, String, String)>> {
    let pattern = format!("%{}%", escape_like_pattern(normalized_query));
    let mut statement = conn.prepare(
        r#"
        SELECT path, kind, name, parent
        FROM launcher_index_entries
        WHERE searchable_text LIKE ?1 ESCAPE '\'
        ORDER BY
            CASE kind
                WHEN 'directory' THEN 0
                WHEN 'file' THEN 1
                ELSE 2
            END ASC,
            name COLLATE NOCASE ASC,
            path COLLATE NOCASE ASC
        LIMIT ?2
        "#,
    )?;
    read_rows(statement.query(params![pattern, limit])?)
}

fn read_rows(mut rows: rusqlite::Rows<'_>) -> AppResult<Vec<(String, String, String, String)>> {
    let mut values = Vec::new();
    while let Some(row) = rows.next()? {
        values.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
    }
    Ok(values)
}

fn build_fts_query(normalized_query: &str) -> Option<String> {
    let terms = normalized_query
        .split_whitespace()
        .map(sanitize_fts_token)
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{term}\"*"))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return None;
    }
    Some(terms.join(" AND "))
}

fn sanitize_fts_token(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .collect::<String>()
}

fn scan_index_root_with_rules(
    root: &Path,
    max_depth: usize,
    max_items: usize,
    remaining_total: usize,
    exclusion_rules: &[ExclusionRule],
    source_root: &str,
    reason: RefreshReason,
) -> ScanOutcome {
    if !root.exists() {
        return ScanOutcome {
            entries: Vec::new(),
            truncated: false,
        };
    }

    let hard_limit = max_items.max(1).min(remaining_total.max(1));
    let mut queue = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0usize));

    let mut entries = Vec::new();
    let mut truncated = false;
    let mut processed: usize = 0;
    let mut warning_aggregator = ScanWarningAggregator::default();
    let home_normalized = current_home_dir()
        .map(|value| normalize_path_for_match(value.as_path()))
        .filter(|value| !value.is_empty());

    while let Some((current_dir, depth)) = queue.pop_front() {
        if entries.len() >= hard_limit {
            truncated = true;
            break;
        }

        let dir_entries = match fs::read_dir(&current_dir) {
            Ok(dir_entries) => dir_entries,
            Err(_error) => {
                warning_aggregator.record(ScanWarningKind::ReadDir, current_dir.as_path());
                continue;
            }
        };

        let is_root_level = depth == 0 && normalize_path_for_match(root) == "/";
        let mut dir_entries = dir_entries
            .filter_map(|entry| match entry {
                Ok(entry) => Some(entry),
                Err(_error) => {
                    warning_aggregator.record(ScanWarningKind::ReadDirEntry, current_dir.as_path());
                    None
                }
            })
            .map(|entry| {
                let path = entry.path();
                let normalized = normalize_path_for_match(path.as_path());
                let priority = scan_priority_for_path(
                    normalized.as_str(),
                    is_root_level,
                    home_normalized.as_deref(),
                );
                (entry, normalized, priority)
            })
            .collect::<Vec<_>>();
        dir_entries.sort_by(|left, right| left.2.cmp(&right.2).then_with(|| left.1.cmp(&right.1)));

        for (dir_entry, _, _) in dir_entries {
            if entries.len() >= hard_limit {
                truncated = true;
                break;
            }

            processed = processed.saturating_add(1);
            if processed.is_multiple_of(SCAN_YIELD_EVERY) {
                thread::sleep(SCAN_YIELD_SLEEP);
            }

            let path = dir_entry.path();
            let file_type = match dir_entry.file_type() {
                Ok(file_type) => file_type,
                Err(_error) => {
                    warning_aggregator.record(ScanWarningKind::FileType, path.as_path());
                    continue;
                }
            };

            if file_type.is_symlink() {
                continue;
            }

            let normalized_path = normalize_path_for_match(path.as_path());
            if should_exclude_path(path.as_path(), normalized_path.as_str(), exclusion_rules) {
                continue;
            }

            if file_type.is_dir() {
                if let Some(entry) = build_index_entry(
                    path.as_path(),
                    IndexedEntryKind::Directory,
                    source_root,
                    Some(&mut warning_aggregator),
                ) {
                    entries.push(entry);
                }

                if depth < max_depth && !should_skip_dir_traversal(path.as_path()) {
                    queue.push_back((path, depth + 1));
                }
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            if let Some(entry) = build_index_entry(
                path.as_path(),
                IndexedEntryKind::File,
                source_root,
                Some(&mut warning_aggregator),
            ) {
                entries.push(entry);
            }
        }
    }

    warning_aggregator.log_summary(
        "launcher_index_scan_warning_summary",
        source_root,
        reason.as_str(),
    );

    ScanOutcome { entries, truncated }
}

#[cfg(test)]
fn scan_index_root(
    root: &Path,
    max_depth: usize,
    max_items: usize,
    source_root: &str,
) -> Vec<LauncherIndexEntry> {
    let rules = build_exclusion_rules(default_exclude_patterns().as_slice());
    scan_index_root_with_rules(
        root,
        max_depth,
        max_items,
        max_items,
        rules.as_slice(),
        source_root,
        RefreshReason::Manual,
    )
    .entries
}

fn build_index_entry(
    path: &Path,
    kind: IndexedEntryKind,
    source_root: &str,
    warning_aggregator: Option<&mut ScanWarningAggregator>,
) -> Option<LauncherIndexEntry> {
    let path_value = path.to_string_lossy().to_string();
    if path_value.trim().is_empty() {
        return None;
    }

    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| path_value.clone());
    let parent = path
        .parent()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path_value.clone());
    let ext = if matches!(kind, IndexedEntryKind::File) {
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
    } else {
        None
    };

    let metadata = match fs::metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(_error) => {
            if let Some(aggregator) = warning_aggregator {
                aggregator.record(ScanWarningKind::Metadata, path);
            }
            None
        }
    };
    let mtime = metadata
        .as_ref()
        .and_then(|value| value.modified().ok())
        .and_then(system_time_to_unix_millis);
    let size = metadata
        .as_ref()
        .map(|value| value.len())
        .and_then(|value| i64::try_from(value).ok());

    let searchable_text = normalize_query(
        format!(
            "{} {} {} {} {}",
            name,
            parent,
            path_value,
            ext.clone().unwrap_or_default(),
            kind.as_str()
        )
        .as_str(),
    );

    Some(LauncherIndexEntry {
        path: path_value,
        kind,
        name,
        parent,
        ext,
        mtime,
        size,
        source_root: source_root.to_string(),
        searchable_text,
    })
}

fn build_exclusion_rules(patterns: &[String]) -> Vec<ExclusionRule> {
    patterns
        .iter()
        .map(|pattern| normalize_path_pattern(pattern))
        .filter(|pattern| !pattern.is_empty())
        .filter_map(|pattern| {
            if pattern.contains('*') || pattern.contains('?') {
                return wildcard_to_regex(pattern.as_str()).map(ExclusionRule::Wildcard);
            }
            if pattern.contains('/') || pattern.contains(':') {
                if !is_absolute_pattern(pattern.as_str()) {
                    return Some(ExclusionRule::Subpath(pattern));
                }
                return Some(ExclusionRule::Prefix(pattern));
            }
            Some(ExclusionRule::Segment(pattern))
        })
        .collect()
}

fn should_exclude_path(path: &Path, normalized_path: &str, rules: &[ExclusionRule]) -> bool {
    if is_hidden(path) {
        return true;
    }

    for rule in rules {
        match rule {
            ExclusionRule::Segment(value) => {
                if path_has_component(path, value.as_str()) {
                    return true;
                }
            }
            ExclusionRule::Prefix(value) => {
                if normalized_path == value
                    || normalized_path
                        .strip_prefix(value.as_str())
                        .is_some_and(|tail| tail.starts_with('/'))
                {
                    return true;
                }
            }
            ExclusionRule::Subpath(value) => {
                if normalized_path == value
                    || normalized_path.ends_with(format!("/{value}").as_str())
                    || normalized_path.contains(format!("/{value}/").as_str())
                {
                    return true;
                }
            }
            ExclusionRule::Wildcard(regex) => {
                if regex.is_match(normalized_path) {
                    return true;
                }
            }
        }
    }
    false
}

fn wildcard_to_regex(pattern: &str) -> Option<Regex> {
    let mut regex = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '\\' | '.' | '+' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '$' | '|' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }
    regex.push('$');
    Regex::new(regex.as_str()).ok()
}

fn is_absolute_pattern(pattern: &str) -> bool {
    if pattern.starts_with('/') {
        return true;
    }
    if pattern.len() >= 2 {
        let bytes = pattern.as_bytes();
        if bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
            return true;
        }
    }
    false
}

fn path_has_component(path: &Path, target: &str) -> bool {
    path.components().any(|component| match component {
        Component::Normal(value) => value
            .to_string_lossy()
            .to_ascii_lowercase()
            .eq_ignore_ascii_case(target),
        _ => false,
    })
}

fn scan_priority_for_path(
    normalized_path: &str,
    is_root_level: bool,
    home_normalized: Option<&str>,
) -> u8 {
    if !is_root_level {
        return 3;
    }

    if home_normalized.is_some_and(|home| path_is_same_or_ancestor(normalized_path, home)) {
        return 0;
    }
    if normalized_path == "/applications" {
        return 1;
    }
    if path_is_same_or_ancestor(normalized_path, "/system/applications") {
        return 2;
    }
    3
}

fn path_is_same_or_ancestor(path: &str, target: &str) -> bool {
    path == target
        || target
            .strip_prefix(path)
            .is_some_and(|tail| tail.starts_with('/'))
}

fn should_skip_dir_traversal(path: &Path) -> bool {
    cfg!(target_os = "macos")
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

fn read_index_ready(db_pool: &DbPool) -> AppResult<bool> {
    let value = read_meta(db_pool, INDEX_READY_KEY)?;
    Ok(value.as_deref().map(is_truthy_flag).unwrap_or(false))
}

fn read_meta(db_pool: &DbPool, key: &str) -> AppResult<Option<String>> {
    let conn = db_pool.get()?;
    conn.query_row(
        "SELECT value FROM launcher_index_meta WHERE key = ?1 LIMIT 1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(Into::into)
}

fn write_meta(db_pool: &DbPool, key: &str, value: &str) -> AppResult<()> {
    let conn = db_pool.get()?;
    conn.execute(
        "INSERT INTO launcher_index_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn load_or_init_settings(db_pool: &DbPool) -> AppResult<LauncherSearchSettingsRecord> {
    let fallback = LauncherSearchSettingsRecord::default().normalize();
    let settings = match get_app_setting(db_pool, SEARCH_SETTINGS_KEY)? {
        Some(raw_value) => {
            match serde_json::from_str::<LauncherSearchSettingsRecord>(raw_value.as_str()) {
                Ok(parsed) => {
                    let normalized = parsed.clone().normalize();
                    if normalized != parsed {
                        save_settings(db_pool, &normalized)?;
                    }
                    normalized
                }
                Err(error) => {
                    tracing::warn!(
                        event = "launcher_settings_parse_failed",
                        error = error.to_string()
                    );
                    save_settings(db_pool, &fallback)?;
                    fallback
                }
            }
        }
        None => {
            save_settings(db_pool, &fallback)?;
            fallback
        }
    };

    enforce_scope_policy_migration(db_pool, settings)
}

fn enforce_scope_policy_migration(
    db_pool: &DbPool,
    mut settings: LauncherSearchSettingsRecord,
) -> AppResult<LauncherSearchSettingsRecord> {
    let from_state = get_app_setting(db_pool, LAUNCHER_SCOPE_POLICY_APPLIED_KEY)?
        .filter(|value| !value.trim().is_empty());
    if from_state.as_deref() == Some(LAUNCHER_SCOPE_POLICY_APPLIED_VALUE) {
        return Ok(settings);
    }

    let old_roots_count = settings.roots.len() as u32;
    settings.roots = default_search_roots();
    save_settings(db_pool, &settings)?;
    set_app_setting(
        db_pool,
        LAUNCHER_SCOPE_POLICY_APPLIED_KEY,
        LAUNCHER_SCOPE_POLICY_APPLIED_VALUE,
    )?;

    tracing::info!(
        event = "launcher_scope_policy_migrated",
        from_state = from_state.as_deref().unwrap_or("none"),
        to_state = LAUNCHER_SCOPE_POLICY_APPLIED_VALUE,
        platform = scope_platform_name(current_scope_platform()),
        old_roots_count,
        new_roots_count = settings.roots.len() as u32
    );

    Ok(settings)
}

fn save_settings(db_pool: &DbPool, settings: &LauncherSearchSettingsRecord) -> AppResult<()> {
    let serialized = serde_json::to_string(settings).map_err(|error| {
        AppError::new("launcher_settings_serialize_failed", "启动器设置序列化失败")
            .with_source(error)
    })?;
    set_app_setting(db_pool, SEARCH_SETTINGS_KEY, serialized.as_str())
}

fn sanitize_roots(roots: Vec<String>) -> Vec<String> {
    let mut values = Vec::new();
    let mut dedup = std::collections::HashSet::new();
    for raw in roots {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = normalize_path_for_match(Path::new(trimmed));
        if normalized.is_empty() || !dedup.insert(normalized) {
            continue;
        }
        values.push(trimmed.to_string());
    }
    values
}

fn sanitize_patterns(patterns: Vec<String>) -> Vec<String> {
    let mut values = Vec::new();
    let mut dedup = std::collections::HashSet::new();
    for pattern in patterns {
        let normalized = normalize_path_pattern(pattern.as_str());
        if normalized.is_empty() || !dedup.insert(normalized.clone()) {
            continue;
        }
        values.push(normalized);
    }
    values
}

fn normalize_path_pattern(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = trimmed.replace('\\', "/").to_ascii_lowercase();
    if normalized == "/" {
        return normalized;
    }
    normalized.trim_end_matches('/').to_string()
}

fn default_search_roots() -> Vec<String> {
    let home_dir = current_home_dir();
    let app_data_dir = std::env::var_os("APPDATA").map(PathBuf::from);
    let program_data_dir = std::env::var_os("ProgramData").map(PathBuf::from);
    let candidates = build_default_search_root_candidates(
        current_scope_platform(),
        home_dir.as_deref(),
        app_data_dir.as_deref(),
        program_data_dir.as_deref(),
    );
    let mut roots = collect_existing_roots(candidates);

    if roots.is_empty() {
        if let Some(home) = home_dir {
            roots.push(home.to_string_lossy().to_string());
        } else {
            roots.push("/".to_string());
        }
    }

    roots
}

fn build_default_search_root_candidates(
    platform: ScopePlatform,
    home_dir: Option<&Path>,
    app_data_dir: Option<&Path>,
    program_data_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if matches!(platform, ScopePlatform::Macos | ScopePlatform::Windows) {
        roots.extend(collect_user_common_roots(home_dir));
        roots.extend(collect_app_roots_for_platform(
            platform,
            home_dir,
            app_data_dir,
            program_data_dir,
        ));
    }
    roots.extend(collect_system_roots_for_platform(platform));
    roots
}

fn collect_user_common_roots(home_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let Some(home) = home_dir else {
        return roots;
    };

    roots.push(home.to_path_buf());
    roots.push(home.join("Desktop"));
    roots.push(home.join("Documents"));
    roots.push(home.join("Downloads"));
    roots
}

fn collect_app_roots_for_platform(
    platform: ScopePlatform,
    home_dir: Option<&Path>,
    app_data_dir: Option<&Path>,
    program_data_dir: Option<&Path>,
) -> Vec<PathBuf> {
    match platform {
        ScopePlatform::Macos => {
            let mut roots = vec![PathBuf::from("/Applications")];
            if let Some(home) = home_dir {
                roots.push(home.join("Applications"));
            }
            roots
        }
        ScopePlatform::Windows => {
            let mut roots = Vec::new();
            if let Some(app_data) = app_data_dir {
                roots.push(app_data.join("Microsoft/Windows/Start Menu/Programs"));
            }
            if let Some(program_data) = program_data_dir {
                roots.push(program_data.join("Microsoft/Windows/Start Menu/Programs"));
            }
            roots
        }
        ScopePlatform::Linux => Vec::new(),
    }
}

fn collect_system_roots_for_platform(platform: ScopePlatform) -> Vec<PathBuf> {
    match platform {
        ScopePlatform::Macos => vec![PathBuf::from("/")],
        ScopePlatform::Windows => (b'A'..=b'Z')
            .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
            .collect(),
        ScopePlatform::Linux => vec![PathBuf::from("/")],
    }
}

fn collect_existing_roots(candidates: Vec<PathBuf>) -> Vec<String> {
    let mut roots = Vec::new();
    let mut dedup = HashSet::new();
    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        let normalized = normalize_path_for_match(candidate.as_path());
        if normalized.is_empty() || !dedup.insert(normalized) {
            continue;
        }
        roots.push(candidate.to_string_lossy().to_string());
    }
    roots
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        ".git".to_string(),
        ".svn".to_string(),
        ".hg".to_string(),
        ".trash".to_string(),
        ".cache".to_string(),
        ".pnpm-store".to_string(),
        ".npm".to_string(),
        ".yarn".to_string(),
        "__pycache__".to_string(),
        "node_modules".to_string(),
        "target".to_string(),
        "dist".to_string(),
        "build".to_string(),
        ".next".to_string(),
        ".nuxt".to_string(),
        "venv".to_string(),
        ".venv".to_string(),
        "library/caches".to_string(),
        "library/containers".to_string(),
        "library/logs".to_string(),
        "/system".to_string(),
        "/private".to_string(),
        "/tmp".to_string(),
        "/var/tmp".to_string(),
        "windows".to_string(),
        "programdata".to_string(),
        "$recycle.bin".to_string(),
    ]
}

fn current_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn is_truthy_flag(value: &str) -> bool {
    matches!(value, "1" | "true" | "TRUE")
}

fn stable_id(prefix: &str, input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{prefix}.{:x}", hasher.finish())
}

fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}

fn system_time_to_unix_millis(value: SystemTime) -> Option<i64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

fn normalize_path_for_match(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    let lower = raw.to_ascii_lowercase();
    if lower == "/" {
        return lower;
    }
    lower.trim_end_matches('/').to_string()
}

fn is_windows_drive_root(normalized: &str) -> bool {
    let bytes = normalized.as_bytes();
    bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn normalize_query(value: &str) -> String {
    value.trim().to_lowercase()
}

fn escape_like_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '%' | '_' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
#[path = "../../tests/launcher/launcher_index_service_tests.rs"]
mod tests;

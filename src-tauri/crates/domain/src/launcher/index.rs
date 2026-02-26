use crate::host::LauncherHost;
use crate::launcher::icon::{resolve_builtin_icon, resolve_file_type_icon};
use foundation::db::{DbConn, get_app_setting, set_app_setting};
use foundation::db_error::DbResult;
use foundation::i18n::t;
use foundation::models::{
    LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto, LauncherRebuildResultDto,
    LauncherRuntimeStatusDto, LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto,
};
use foundation::{AppError, AppResult};
use libsql::params_from_iter;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::sleep;

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
const LAUNCHER_SCOPE_POLICY_VERSION_KEY: &str = "launcher.search.scope_policy_version";
const LAUNCHER_SCOPE_POLICY_VERSION_VALUE: &str = "2";

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

#[path = "persistence.rs"]
mod persistence;
#[path = "query.rs"]
mod query;
#[path = "refresh.rs"]
mod refresh;
#[path = "scan.rs"]
mod scan;
#[path = "settings.rs"]
mod settings;

use persistence::{
    delete_stale_entries_for_root, purge_removed_roots, read_index_ready, read_meta,
    upsert_entries_batched, write_meta,
};
use scan::{build_exclusion_rules, scan_index_root_with_rules};
use settings::{
    has_single_system_root_scope, is_default_scope_profile, is_truthy_flag, load_or_init_settings,
    normalize_path_for_match, normalize_path_pattern, normalize_query,
    resolve_effective_max_items_per_root,
};

#[cfg(test)]
use refresh::{TruncationLogLevel, classify_truncation_log_level};
#[cfg(test)]
use scan::{
    ScanWarningAggregator, ScanWarningKind, build_index_entry, scan_index_root,
    scan_priority_for_path,
};
use settings::escape_like_pattern;
#[cfg(test)]
use settings::{
    ScopePlatform, build_default_search_root_candidates, default_exclude_patterns, save_settings,
};

pub use query::{IndexedSearchResult, search_indexed_items_async};
pub use refresh::{
    get_index_status_async, get_indexer_runtime_status, rebuild_index_now_async,
    start_background_indexer, stop_background_indexer,
};
pub use settings::{
    get_search_settings_async, reset_search_settings_async, update_search_settings_async,
};

#[cfg(test)]
#[path = "../../tests/launcher/launcher_index_service_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../tests/launcher/launcher_slo_smoke_tests.rs"]
mod slo_smoke_tests;

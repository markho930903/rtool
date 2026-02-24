use super::*;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

const APP_SIZE_CACHE_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
pub(super) struct PathSizeWarning {
    pub(super) code: AppManagerScanWarningCode,
    pub(super) path: String,
    pub(super) detail_code: AppManagerScanWarningDetailCode,
}

#[derive(Debug, Clone)]
pub(super) struct PathSizeComputation {
    pub(super) size_bytes: u64,
    pub(super) warnings: Vec<PathSizeWarning>,
}

#[derive(Debug, Clone)]
pub(super) struct AppSizeSnapshot {
    pub(super) size_bytes: Option<u64>,
    pub(super) size_accuracy: AppManagerSizeAccuracy,
    pub(super) size_computed_at: Option<i64>,
}

#[derive(Debug, Clone)]
struct AppSizeCacheEntry {
    path_signature: String,
    snapshot: AppSizeSnapshot,
    refreshed_at: Instant,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub(super) struct MacStartupCache {
    pub(super) refreshed_at: Option<Instant>,
    pub(super) user_plist_blobs: Vec<String>,
    pub(super) system_plist_blobs: Vec<String>,
}

#[cfg(target_os = "macos")]
impl MacStartupCache {
    pub(super) fn new() -> Self {
        Self {
            refreshed_at: None,
            user_plist_blobs: Vec::new(),
            system_plist_blobs: Vec::new(),
        }
    }

    pub(super) fn is_stale(&self) -> bool {
        match self.refreshed_at {
            None => true,
            Some(at) => at.elapsed() >= MAC_STARTUP_CACHE_TTL,
        }
    }
}

impl AppIndexCache {
    pub(super) fn new() -> Self {
        Self {
            refreshed_at: None,
            indexed_at: 0,
            items: Vec::new(),
            revision: 0,
            source_fingerprint: String::new(),
            building: false,
            index_state: AppManagerIndexState::Ready,
            last_error: None,
            disk_bootstrapped: false,
        }
    }

    pub(super) fn is_stale(&self) -> bool {
        match self.refreshed_at {
            None => true,
            Some(at) => at.elapsed() >= INDEX_CACHE_TTL,
        }
    }
}

pub(super) fn app_index_runtime() -> &'static AppIndexRuntime {
    static RUNTIME: OnceLock<AppIndexRuntime> = OnceLock::new();
    RUNTIME.get_or_init(|| AppIndexRuntime {
        cache: Mutex::new(AppIndexCache::new()),
        condvar: Condvar::new(),
    })
}

pub(super) fn residue_scan_cache() -> &'static Mutex<HashMap<String, ResidueScanCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, ResidueScanCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn app_size_cache() -> &'static Mutex<HashMap<String, AppSizeCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, AppSizeCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "macos")]
pub(super) fn mac_startup_cache() -> &'static Mutex<MacStartupCache> {
    static CACHE: OnceLock<Mutex<MacStartupCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(MacStartupCache::new()))
}

fn path_signature(path: &Path) -> String {
    if !path.exists() {
        return "missing".to_string();
    }
    match fs::metadata(path) {
        Ok(metadata) => {
            let modified = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|value| value.as_secs())
                .unwrap_or(0);
            let type_key = if metadata.is_file() {
                "f"
            } else if metadata.is_dir() {
                "d"
            } else {
                "o"
            };
            format!("{type_key}|{}|{modified}", metadata.len())
        }
        Err(_) => "metadata-error".to_string(),
    }
}

pub(super) fn resolve_app_size_snapshot(path: &Path) -> AppSizeSnapshot {
    let path_key = normalize_path_key(path.to_string_lossy().as_ref());
    let signature = path_signature(path);
    {
        let cache = app_size_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = cache.get(path_key.as_str())
            && entry.path_signature == signature
            && entry.refreshed_at.elapsed() <= APP_SIZE_CACHE_TTL
        {
            return entry.snapshot.clone();
        }
    }

    let computed_at = now_unix_seconds();
    let snapshot = if let Some(size_bytes) = exact_path_size_bytes(path) {
        AppSizeSnapshot {
            size_bytes: Some(size_bytes),
            size_accuracy: AppManagerSizeAccuracy::Exact,
            size_computed_at: Some(computed_at),
        }
    } else if let Some(size_bytes) = try_get_path_size_bytes(path) {
        AppSizeSnapshot {
            size_bytes: Some(size_bytes),
            size_accuracy: AppManagerSizeAccuracy::Estimated,
            size_computed_at: Some(computed_at),
        }
    } else {
        AppSizeSnapshot {
            size_bytes: None,
            size_accuracy: AppManagerSizeAccuracy::Estimated,
            size_computed_at: None,
        }
    };

    let mut cache = app_size_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.insert(
        path_key,
        AppSizeCacheEntry {
            path_signature: signature,
            snapshot: snapshot.clone(),
            refreshed_at: Instant::now(),
        },
    );
    snapshot
}

pub(super) fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

pub(super) fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub(super) fn sanitize_file_stem(value: &str) -> String {
    let mut out = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if out.trim_matches('_').is_empty() {
        out = "app".to_string();
    }
    out.trim_matches('_').to_string()
}

pub(super) fn export_root_dir() -> PathBuf {
    if let Some(home) = home_dir() {
        let download = home.join("Downloads");
        return download.join(EXPORT_DIR_NAME);
    }
    std::env::temp_dir().join(EXPORT_DIR_NAME)
}

pub(super) fn stable_hash(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub(super) fn stable_app_id(prefix: &str, path: &str) -> String {
    format!("{prefix}.{}", stable_hash(path))
}

pub(super) fn normalize_path_key(path: &str) -> String {
    let trimmed = path.trim();
    #[cfg(target_os = "windows")]
    {
        trimmed.replace('/', "\\").to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        trimmed.to_string()
    }
}

pub(super) fn startup_label(app_id: &str) -> String {
    let short = stable_hash(app_id).chars().take(12).collect::<String>();
    format!("{STARTUP_LABEL_PREFIX}.{short}")
}

pub(super) fn fingerprint_for_app(item: &ManagedAppDto) -> String {
    let content = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        item.id,
        item.name,
        item.path,
        item.bundle_or_app_id.clone().unwrap_or_default(),
        item.version.clone().unwrap_or_default(),
        item.publisher.clone().unwrap_or_default(),
        item.size_bytes.unwrap_or(0),
        item.size_accuracy.as_str(),
        item.size_computed_at.unwrap_or(0)
    );
    stable_hash(content.as_str())
}

pub(super) fn make_action_result(
    ok: bool,
    code: AppManagerActionCode,
    message: impl Into<String>,
    detail: Option<String>,
) -> AppManagerActionResultDto {
    AppManagerActionResultDto {
        ok,
        code,
        message: message.into(),
        detail,
    }
}

pub(super) fn startup_readonly_reason_code(
    startup_scope: AppManagerStartupScope,
    startup_editable: bool,
) -> Option<AppReadonlyReasonCode> {
    if startup_editable {
        return None;
    }
    if matches!(startup_scope, AppManagerStartupScope::System) {
        return Some(AppReadonlyReasonCode::ManagedByPolicy);
    }
    Some(AppReadonlyReasonCode::PermissionDenied)
}

pub(super) fn resolve_app_size_path(path: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        for ancestor in path.ancestors() {
            let is_app_bundle = ancestor
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("app"));
            if is_app_bundle {
                return ancestor.to_path_buf();
            }
        }
    }

    if path.is_file() {
        return path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf());
    }
    path.to_path_buf()
}

pub(super) fn append_path_size_warning(
    warnings: &mut Vec<PathSizeWarning>,
    code: AppManagerScanWarningCode,
    path: &Path,
    detail_code: AppManagerScanWarningDetailCode,
) {
    if warnings.len() >= SIZE_WARNING_LIMIT {
        return;
    }
    let path_value = path.to_string_lossy().to_string();
    if warnings
        .iter()
        .any(|warning| warning.code == code && warning.path == path_value)
    {
        return;
    }
    warnings.push(PathSizeWarning {
        code,
        path: path_value,
        detail_code,
    });
}

pub(super) fn walk_path_size_bytes(
    path: &Path,
    max_depth: Option<usize>,
    max_dirs: Option<usize>,
    collect_warnings: bool,
) -> Option<PathSizeComputation> {
    if !path.exists() {
        return None;
    }

    let mut warnings = Vec::new();
    if path.is_file() {
        return match fs::metadata(path) {
            Ok(meta) => Some(PathSizeComputation {
                size_bytes: meta.len(),
                warnings,
            }),
            Err(error) => {
                if collect_warnings {
                    append_path_size_warning(
                        &mut warnings,
                        AppManagerScanWarningCode::AppManagerSizeMetadataReadFailed,
                        path,
                        AppManagerScanWarningDetailCode::from_io_error_kind(error.kind()),
                    );
                    return Some(PathSizeComputation {
                        size_bytes: 0,
                        warnings,
                    });
                }
                None
            }
        };
    }

    let mut total = 0u64;
    let mut queue = VecDeque::new();
    queue.push_back((path.to_path_buf(), 0usize));
    let mut visited_dirs = 0usize;
    while let Some((dir, depth)) = queue.pop_front() {
        if max_dirs.is_some_and(|limit| visited_dirs >= limit) {
            if collect_warnings {
                append_path_size_warning(
                    &mut warnings,
                    AppManagerScanWarningCode::AppManagerSizeEstimateTruncated,
                    path,
                    AppManagerScanWarningDetailCode::LimitReached,
                );
            }
            break;
        }
        visited_dirs += 1;

        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                if collect_warnings {
                    append_path_size_warning(
                        &mut warnings,
                        AppManagerScanWarningCode::AppManagerSizeReadDirFailed,
                        dir.as_path(),
                        AppManagerScanWarningDetailCode::from_io_error_kind(error.kind()),
                    );
                }
                continue;
            }
        };

        for entry_result in entries {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(error) => {
                    if collect_warnings {
                        append_path_size_warning(
                            &mut warnings,
                            AppManagerScanWarningCode::AppManagerSizeReadDirEntryFailed,
                            dir.as_path(),
                            AppManagerScanWarningDetailCode::from_io_error_kind(error.kind()),
                        );
                    }
                    continue;
                }
            };
            let entry_path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    if collect_warnings {
                        append_path_size_warning(
                            &mut warnings,
                            AppManagerScanWarningCode::AppManagerSizeReadFileTypeFailed,
                            entry_path.as_path(),
                            AppManagerScanWarningDetailCode::from_io_error_kind(error.kind()),
                        );
                    }
                    continue;
                }
            };

            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if max_depth.map(|limit| depth < limit).unwrap_or(true) {
                    queue.push_back((entry_path, depth + 1));
                }
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            match entry.metadata() {
                Ok(meta) => {
                    total = total.saturating_add(meta.len());
                }
                Err(error) => {
                    if collect_warnings {
                        append_path_size_warning(
                            &mut warnings,
                            AppManagerScanWarningCode::AppManagerSizeReadMetadataFailed,
                            entry_path.as_path(),
                            AppManagerScanWarningDetailCode::from_io_error_kind(error.kind()),
                        );
                    }
                }
            }
        }
    }

    Some(PathSizeComputation {
        size_bytes: total,
        warnings,
    })
}

pub(super) fn try_get_path_size_bytes(path: &Path) -> Option<u64> {
    // Keep list rendering lightweight to avoid blocking large I/O.
    walk_path_size_bytes(
        path,
        Some(SIZE_ESTIMATE_MAX_DEPTH),
        Some(SIZE_ESTIMATE_MAX_DIRS),
        false,
    )
    .map(|value| value.size_bytes)
}

pub(super) fn exact_path_size_bytes(path: &Path) -> Option<u64> {
    walk_path_size_bytes(path, None, None, false).map(|value| value.size_bytes)
}

pub(super) fn exact_path_size_bytes_with_warnings(path: &Path) -> Option<PathSizeComputation> {
    walk_path_size_bytes(path, None, None, true)
}

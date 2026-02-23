use super::*;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

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
        }
    }

    pub(super) fn is_stale(&self) -> bool {
        match self.refreshed_at {
            None => true,
            Some(at) => at.elapsed() >= INDEX_CACHE_TTL,
        }
    }
}

pub(super) fn app_index_cache() -> &'static Mutex<AppIndexCache> {
    static CACHE: OnceLock<Mutex<AppIndexCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(AppIndexCache::new()))
}

pub(super) fn residue_scan_cache() -> &'static Mutex<HashMap<String, ResidueScanCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, ResidueScanCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "macos")]
pub(super) fn mac_startup_cache() -> &'static Mutex<MacStartupCache> {
    static CACHE: OnceLock<Mutex<MacStartupCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(MacStartupCache::new()))
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
        "{}|{}|{}|{}|{}|{}",
        item.id,
        item.name,
        item.path,
        item.bundle_or_app_id.clone().unwrap_or_default(),
        item.version.clone().unwrap_or_default(),
        item.publisher.clone().unwrap_or_default()
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

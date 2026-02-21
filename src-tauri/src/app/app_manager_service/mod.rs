use crate::app::icon_service::{resolve_application_icon, resolve_builtin_icon};
use crate::core::models::{
    AppManagerActionCode, AppManagerActionResultDto, AppManagerCapabilitiesDto,
    AppManagerCleanupDeleteMode, AppManagerCleanupInputDto, AppManagerCleanupItemResultDto,
    AppManagerCleanupReasonCode, AppManagerCleanupResultDto, AppManagerCleanupStatus,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerIconKind, AppManagerIdentityDto, AppManagerIdentitySource, AppManagerPageDto,
    AppManagerPathType, AppManagerPlatform, AppManagerQueryDto, AppManagerResidueConfidence,
    AppManagerResidueGroupDto, AppManagerResidueItemDto, AppManagerResidueKind,
    AppManagerResidueMatchReason, AppManagerResidueScanInputDto, AppManagerResidueScanResultDto,
    AppManagerRiskLevel, AppManagerScanWarningCode, AppManagerScanWarningDetailCode,
    AppManagerScanWarningDto, AppManagerScope, AppManagerSource, AppManagerStartupScope,
    AppManagerStartupUpdateInputDto, AppManagerUninstallInputDto, AppManagerUninstallKind,
    AppReadonlyReasonCode, AppRelatedRootDto, AppSizeSummaryDto, ManagedAppDetailDto,
    ManagedAppDto,
};
use crate::core::{AppError, AppResult, ResultExt};
use anyhow::Context;
#[cfg(target_os = "macos")]
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

mod api;
mod index;
mod residue_cleanup;
mod residue_scan;
mod startup;
mod uninstall;

pub use api::*;
use index::*;
use residue_cleanup::*;
use residue_scan::*;
use startup::*;
use uninstall::*;

const INDEX_CACHE_TTL: Duration = Duration::from_secs(30);
const RESIDUE_SCAN_CACHE_TTL: Duration = Duration::from_secs(120);
const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 300;
#[cfg(target_os = "macos")]
const MAC_SCAN_MAX_ITEMS: usize = 500;
#[cfg(target_os = "macos")]
const MAC_STARTUP_CACHE_TTL: Duration = Duration::from_secs(20);
#[cfg(target_os = "windows")]
const WIN_SCAN_MAX_ITEMS: usize = 700;
const STARTUP_LABEL_PREFIX: &str = "com.rtool.startup";
const EXPORT_DIR_NAME: &str = "rtool-app-scan-exports";
const SIZE_ESTIMATE_MAX_DEPTH: usize = 3;
const SIZE_ESTIMATE_MAX_DIRS: usize = 2_000;
const SIZE_WARNING_LIMIT: usize = 24;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AppManagerErrorCode {
    NotFound,
    StartupReadOnly,
    ExportDirFailed,
    ExportSerializeFailed,
    ExportWriteFailed,
    FingerprintMismatch,
    UninstallUnsupported,
    UninstallSelfForbidden,
    UninstallNotSupported,
    OpenHelpInvalid,
    OpenHelpFailed,
    OpenHelpNotSupported,
    UninstallInvalidPath,
    UninstallNotFound,
    UninstallFailed,
    StartupNotSupported,
    StartupPathMissing,
    StartupPathInvalid,
    StartupDirCreateFailed,
    StartupWriteFailed,
    StartupDeleteFailed,
    StartupUpdateFailed,
    CleanupDeleteFailed,
    CleanupModeInvalid,
    CleanupNotFound,
    CleanupPathInvalid,
    CleanupNotSupported,
    FingerprintMissing,
    CleanupFailed,
}

impl AppManagerErrorCode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "app_manager_not_found",
            Self::StartupReadOnly => "app_manager_startup_read_only",
            Self::ExportDirFailed => "app_manager_export_dir_failed",
            Self::ExportSerializeFailed => "app_manager_export_serialize_failed",
            Self::ExportWriteFailed => "app_manager_export_write_failed",
            Self::FingerprintMismatch => "app_manager_fingerprint_mismatch",
            Self::UninstallUnsupported => "app_manager_uninstall_unsupported",
            Self::UninstallSelfForbidden => "app_manager_uninstall_self_forbidden",
            Self::UninstallNotSupported => "app_manager_uninstall_not_supported",
            Self::OpenHelpInvalid => "app_manager_open_help_invalid",
            Self::OpenHelpFailed => "app_manager_open_help_failed",
            Self::OpenHelpNotSupported => "app_manager_open_help_not_supported",
            Self::UninstallInvalidPath => "app_manager_uninstall_invalid_path",
            Self::UninstallNotFound => "app_manager_uninstall_not_found",
            Self::UninstallFailed => "app_manager_uninstall_failed",
            Self::StartupNotSupported => "app_manager_startup_not_supported",
            Self::StartupPathMissing => "app_manager_startup_path_missing",
            Self::StartupPathInvalid => "app_manager_startup_path_invalid",
            Self::StartupDirCreateFailed => "app_manager_startup_dir_create_failed",
            Self::StartupWriteFailed => "app_manager_startup_write_failed",
            Self::StartupDeleteFailed => "app_manager_startup_delete_failed",
            Self::StartupUpdateFailed => "app_manager_startup_update_failed",
            Self::CleanupDeleteFailed => "app_manager_cleanup_delete_failed",
            Self::CleanupModeInvalid => "app_manager_cleanup_mode_invalid",
            Self::CleanupNotFound => "app_manager_cleanup_not_found",
            Self::CleanupPathInvalid => "app_manager_cleanup_path_invalid",
            Self::CleanupNotSupported => "app_manager_cleanup_not_supported",
            Self::FingerprintMissing => "app_manager_fingerprint_missing",
            Self::CleanupFailed => "app_manager_cleanup_failed",
        }
    }
}

fn app_error(code: AppManagerErrorCode, message: impl Into<String>) -> AppError {
    AppError::new(code.as_str(), message.into())
}

#[derive(Debug, Clone)]
struct AppIndexCache {
    refreshed_at: Option<Instant>,
    indexed_at: i64,
    items: Vec<ManagedAppDto>,
}

#[derive(Debug, Clone)]
struct ResidueScanCacheEntry {
    refreshed_at: Instant,
    result: AppManagerResidueScanResultDto,
}

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
struct MacStartupCache {
    refreshed_at: Option<Instant>,
    user_plist_blobs: Vec<String>,
    system_plist_blobs: Vec<String>,
}

#[cfg(target_os = "macos")]
impl MacStartupCache {
    fn new() -> Self {
        Self {
            refreshed_at: None,
            user_plist_blobs: Vec::new(),
            system_plist_blobs: Vec::new(),
        }
    }

    fn is_stale(&self) -> bool {
        match self.refreshed_at {
            None => true,
            Some(at) => at.elapsed() >= MAC_STARTUP_CACHE_TTL,
        }
    }
}

impl AppIndexCache {
    fn new() -> Self {
        Self {
            refreshed_at: None,
            indexed_at: 0,
            items: Vec::new(),
        }
    }

    fn is_stale(&self) -> bool {
        match self.refreshed_at {
            None => true,
            Some(at) => at.elapsed() >= INDEX_CACHE_TTL,
        }
    }
}

fn app_index_cache() -> &'static Mutex<AppIndexCache> {
    static CACHE: OnceLock<Mutex<AppIndexCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(AppIndexCache::new()))
}

fn residue_scan_cache() -> &'static Mutex<HashMap<String, ResidueScanCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, ResidueScanCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "macos")]
fn mac_startup_cache() -> &'static Mutex<MacStartupCache> {
    static CACHE: OnceLock<Mutex<MacStartupCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(MacStartupCache::new()))
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn sanitize_file_stem(value: &str) -> String {
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

fn export_root_dir() -> PathBuf {
    if let Some(home) = home_dir() {
        let download = home.join("Downloads");
        return download.join(EXPORT_DIR_NAME);
    }
    std::env::temp_dir().join(EXPORT_DIR_NAME)
}

fn stable_hash(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn stable_app_id(prefix: &str, path: &str) -> String {
    format!("{prefix}.{}", stable_hash(path))
}

fn normalize_path_key(path: &str) -> String {
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

#[derive(Debug, Clone)]
struct DisplayNameCandidate {
    value: String,
    confidence: u8,
}

fn normalize_display_name(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return None;
    }
    Some(normalized)
}

fn path_stem_string(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .and_then(normalize_display_name)
}

fn normalize_name_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_name_tokens(value: &str) -> Vec<String> {
    normalize_name_key(value)
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn push_display_name_candidate(
    candidates: &mut Vec<DisplayNameCandidate>,
    value: Option<String>,
    confidence: u8,
) {
    let Some(value) = value.as_deref().and_then(normalize_display_name) else {
        return;
    };
    candidates.push(DisplayNameCandidate { value, confidence });
}

fn score_display_name_candidate(
    candidate: &DisplayNameCandidate,
    stem_key: &str,
    stem_tokens: &[String],
) -> i32 {
    let mut score = i32::from(candidate.confidence) * 10;
    let candidate_key = normalize_name_key(candidate.value.as_str());
    let candidate_tokens = candidate_key
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if candidate_tokens.len() >= 2 {
        score += 30;
    }
    if candidate.value.chars().count() >= 8 {
        score += 15;
    }

    if !stem_key.is_empty() && !candidate_key.is_empty() {
        if candidate_key == stem_key {
            score += 80;
        } else {
            if stem_key.contains(candidate_key.as_str()) {
                score += 35;
            }
            if candidate_key.contains(stem_key) {
                score += 20;
            }
            let shared = candidate_tokens
                .iter()
                .filter(|token| stem_tokens.iter().any(|stem| stem == *token))
                .count();
            score += (shared as i32) * 18;
        }
    }

    if candidate_tokens.len() == 1 {
        let len = candidate.value.chars().count();
        let stem_word_count = stem_tokens.len();
        if len <= 4 && stem_word_count >= 2 {
            score -= 90;
        } else if len <= 5 && stem_word_count >= 2 {
            score -= 40;
        }
    }

    if matches!(candidate_key.as_str(), "app" | "application" | "program") {
        score -= 60;
    }

    score
}

fn resolve_application_display_name(
    path: &Path,
    path_fallback: &str,
    candidates: Vec<DisplayNameCandidate>,
) -> String {
    let mut dedup = HashMap::<String, DisplayNameCandidate>::new();
    for candidate in candidates {
        let key = candidate.value.to_ascii_lowercase();
        match dedup.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if candidate.confidence > entry.get().confidence {
                    entry.insert(candidate);
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(candidate);
            }
        }
    }

    let mut dedup_candidates = dedup.into_values().collect::<Vec<_>>();
    if dedup_candidates.is_empty() {
        return path_stem_string(path).unwrap_or_else(|| path_fallback.to_string());
    }

    let stem = path_stem_string(path).unwrap_or_else(|| path_fallback.to_string());
    let stem_key = normalize_name_key(stem.as_str());
    let stem_tokens = split_name_tokens(stem.as_str());

    dedup_candidates.sort_by(|left, right| {
        let left_score =
            score_display_name_candidate(left, stem_key.as_str(), stem_tokens.as_slice());
        let right_score =
            score_display_name_candidate(right, stem_key.as_str(), stem_tokens.as_slice());
        right_score
            .cmp(&left_score)
            .then_with(|| right.value.chars().count().cmp(&left.value.chars().count()))
            .then_with(|| left.value.cmp(&right.value))
    });

    dedup_candidates
        .into_iter()
        .next()
        .map(|candidate| candidate.value)
        .unwrap_or_else(|| stem)
}

fn startup_label(app_id: &str) -> String {
    let short = stable_hash(app_id).chars().take(12).collect::<String>();
    format!("{STARTUP_LABEL_PREFIX}.{short}")
}

fn fingerprint_for_app(item: &ManagedAppDto) -> String {
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

fn make_action_result(
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

fn startup_readonly_reason_code(
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

fn append_path_size_warning(
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

fn walk_path_size_bytes(
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

fn try_get_path_size_bytes(path: &Path) -> Option<u64> {
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

fn path_is_readonly(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.permissions().readonly())
        .unwrap_or(false)
}

fn cleanup_stale_scan_cache() {
    let mut cache = residue_scan_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.retain(|_, entry| entry.refreshed_at.elapsed() <= RESIDUE_SCAN_CACHE_TTL);
}

fn load_or_refresh_index(app: &AppHandle, force_refresh: bool) -> AppResult<AppIndexCache> {
    let stale = {
        let cache = app_index_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        force_refresh || cache.is_stale()
    };

    if stale {
        let items = build_app_index(app)?;
        let indexed_at = now_unix_seconds();
        let mut cache = app_index_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.items = items;
        cache.refreshed_at = Some(Instant::now());
        cache.indexed_at = indexed_at;
        return Ok(cache.clone());
    }

    let cache = app_index_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Ok(cache.clone())
}

fn app_install_root(item: &ManagedAppDto) -> PathBuf {
    let path = PathBuf::from(item.path.as_str());
    if path.is_file() {
        return path.parent().map(Path::to_path_buf).unwrap_or(path);
    }
    path
}

fn build_app_capabilities(
    startup: bool,
    uninstall: bool,
    residue_scan: bool,
) -> AppManagerCapabilitiesDto {
    AppManagerCapabilitiesDto {
        startup,
        uninstall,
        residue_scan,
    }
}

fn build_app_identity(
    primary_id: impl Into<String>,
    aliases: Vec<String>,
    identity_source: AppManagerIdentitySource,
) -> AppManagerIdentityDto {
    AppManagerIdentityDto {
        primary_id: primary_id.into(),
        aliases,
        identity_source,
    }
}

fn collect_app_path_aliases_from_parts(
    name: &str,
    path: &str,
    bundle_or_app_id: Option<&str>,
) -> Vec<String> {
    let mut aliases = Vec::new();
    let mut seen = HashSet::new();
    let mut push_alias = |value: &str| {
        let normalized = value.trim().trim_matches(|ch| matches!(ch, '"' | '\''));
        if normalized.len() < 2
            || matches!(normalized, "." | "..")
            || normalized.contains('/')
            || normalized.contains('\\')
        {
            return;
        }
        let key = normalize_path_key(normalized);
        if key.is_empty() || !seen.insert(key) {
            return;
        }
        aliases.push(normalized.to_string());
    };

    push_alias(name);
    if let Some(file_name) = Path::new(path).file_stem().and_then(|value| value.to_str()) {
        push_alias(file_name);
    }
    if let Some(bundle) = bundle_or_app_id {
        push_alias(bundle);
        if let Some(last_part) = bundle.rsplit('.').next() {
            push_alias(last_part);
        }
    }
    aliases
}

fn collect_app_path_aliases(item: &ManagedAppDto) -> Vec<String> {
    collect_app_path_aliases_from_parts(
        item.name.as_str(),
        item.path.as_str(),
        item.bundle_or_app_id.as_deref(),
    )
}

#[cfg(target_os = "windows")]
fn windows_powershell_escape(value: &str) -> String {
    value.replace('\'', "''")
}

fn open_with_command(
    command: &str,
    args: &[&str],
    error_code: AppManagerErrorCode,
) -> AppResult<()> {
    let status = Command::new(command)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute command: {} {:?}", command, args))
        .with_code(error_code.as_str(), "系统操作失败")?;
    if status.success() {
        return Ok(());
    }
    Err(app_error(error_code, "系统操作失败")
        .with_context("status", status.to_string())
        .with_context("command", command)
        .with_context("args", args.join(" ")))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

#[cfg(test)]
#[path = "../../../tests/app/app_manager_service/residue_tests.rs"]
mod residue_tests;

#[cfg(test)]
#[path = "../../../tests/app/app_manager_service/path_size_tests.rs"]
mod path_size_tests;

#[cfg(test)]
#[path = "../../../tests/app/app_manager_service/display_name_tests.rs"]
mod display_name_tests;

#[cfg(test)]
#[path = "../../../tests/app/app_manager_service/query_contract_tests.rs"]
mod query_contract_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../tests/app/app_manager_service/macos_tests.rs"]
mod tests;

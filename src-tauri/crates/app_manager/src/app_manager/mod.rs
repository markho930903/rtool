use crate::host::LauncherHost;
use crate::launcher::icon::{resolve_application_icon, resolve_builtin_icon};
use anyhow::Context;
use protocol::models::{
    AppManagerActionCode, AppManagerActionResultDto, AppManagerCapabilitiesDto,
    AppManagerCleanupDeleteMode, AppManagerCleanupInputDto, AppManagerCleanupItemResultDto,
    AppManagerCleanupReasonCode, AppManagerCleanupResultDto, AppManagerCleanupStatus,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerIconKind, AppManagerIdentityDto, AppManagerIdentitySource, AppManagerIndexState,
    AppManagerIndexUpdateReason, AppManagerIndexUpdatedPayloadDto, AppManagerPageDto,
    AppManagerPathType, AppManagerPlatform, AppManagerQueryDto, AppManagerResidueConfidence,
    AppManagerResidueGroupDto, AppManagerResidueItemDto, AppManagerResidueKind,
    AppManagerResidueMatchReason, AppManagerResidueScanInputDto, AppManagerResidueScanMode,
    AppManagerResidueScanResultDto, AppManagerResolveSizesInputDto,
    AppManagerResolveSizesResultDto, AppManagerResolvedSizeDto, AppManagerRiskLevel,
    AppManagerScanWarningCode, AppManagerScanWarningDetailCode, AppManagerScanWarningDto,
    AppManagerScope, AppManagerSizeAccuracy, AppManagerSnapshotMetaDto, AppManagerSource,
    AppManagerStartupScope, AppManagerStartupUpdateInputDto, AppManagerUninstallInputDto,
    AppManagerUninstallKind, AppReadonlyReasonCode, AppRelatedRootDto, AppSizeSummaryDto,
    ManagedAppDetailDto, ManagedAppDto,
};
use protocol::{AppError, AppResult, ResultExt};
#[cfg(target_os = "macos")]
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod api;
mod cleanup;
mod discovery;
mod identity;
mod index;
mod naming;
mod residue;
mod size;
mod startup;
mod uninstall;

pub use api::*;
use cleanup::*;
use discovery::*;
use identity::*;
use index::*;
use naming::*;
use residue::*;
use size::*;
use startup::*;
use uninstall::*;

const INDEX_CACHE_TTL: Duration = Duration::from_secs(30);
const INDEX_DISK_CACHE_FILE: &str = "app_manager_index_cache.json";
const INDEX_DISK_CACHE_PREFIX: &str = "app_manager_index_cache";
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
    revision: u64,
    source_fingerprint: String,
    building: bool,
    index_state: AppManagerIndexState,
    last_error: Option<String>,
    disk_bootstrapped: bool,
}

#[derive(Debug, Clone)]
struct AppIndexRefreshMeta {
    cache: AppIndexCache,
    changed_count: u32,
    rebuilt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedAppIndexCache {
    indexed_at: i64,
    revision: u64,
    source_fingerprint: String,
    items: Vec<ManagedAppDto>,
}

#[derive(Debug, Clone)]
struct ResidueScanCacheEntry {
    refreshed_at: Instant,
    result: AppManagerResidueScanResultDto,
}

struct AppIndexRuntime {
    cache: Mutex<AppIndexCache>,
    condvar: Condvar,
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

fn count_item_changes(previous: &[ManagedAppDto], next: &[ManagedAppDto]) -> u32 {
    let mut previous_map = HashMap::new();
    for item in previous {
        previous_map.insert(item.id.as_str(), item.fingerprint.as_str());
    }
    let mut changed = 0u32;
    for item in next {
        let key = item.id.as_str();
        let changed_item = previous_map
            .remove(key)
            .is_none_or(|old_fingerprint| old_fingerprint != item.fingerprint.as_str());
        if changed_item {
            changed = changed.saturating_add(1);
        }
    }
    changed.saturating_add(previous_map.len() as u32)
}

fn try_bootstrap_index_from_disk(app: &dyn LauncherHost, cache: &mut AppIndexCache) {
    if cache.disk_bootstrapped {
        return;
    }
    cache.disk_bootstrapped = true;
    let Ok(app_data_dir) = app.app_data_dir() else {
        return;
    };
    cleanup_stale_index_cache_files(app_data_dir.as_path());
    let path = app_data_dir.join(INDEX_DISK_CACHE_FILE);
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    let Ok(snapshot) = serde_json::from_str::<PersistedAppIndexCache>(&content) else {
        return;
    };
    cache.items = snapshot.items;
    cache.indexed_at = snapshot.indexed_at;
    cache.revision = snapshot.revision;
    cache.source_fingerprint = snapshot.source_fingerprint;
    cache.index_state = AppManagerIndexState::Ready;
    cache.last_error = None;
    cache.refreshed_at = Some(Instant::now());
}

fn cleanup_stale_index_cache_files(app_data_dir: &Path) {
    let Ok(entries) = fs::read_dir(app_data_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name == INDEX_DISK_CACHE_FILE {
            continue;
        }
        if file_name.starts_with(INDEX_DISK_CACHE_PREFIX) {
            let _ = fs::remove_file(path);
        }
    }
}

fn persist_index_to_disk(app: &dyn LauncherHost, cache: &AppIndexCache) {
    let Ok(app_data_dir) = app.app_data_dir() else {
        return;
    };
    if fs::create_dir_all(&app_data_dir).is_err() {
        return;
    }
    let path = app_data_dir.join(INDEX_DISK_CACHE_FILE);
    let temp_path = app_data_dir.join(format!("{INDEX_DISK_CACHE_FILE}.tmp"));
    let snapshot = PersistedAppIndexCache {
        indexed_at: cache.indexed_at,
        revision: cache.revision,
        source_fingerprint: cache.source_fingerprint.clone(),
        items: cache.items.clone(),
    };
    let Ok(content) = serde_json::to_vec(&snapshot) else {
        return;
    };
    if fs::write(&temp_path, content).is_err() {
        return;
    }
    let _ = fs::rename(temp_path, path);
}

fn refresh_index_with_meta(
    app: &dyn LauncherHost,
    force_refresh: bool,
) -> AppResult<AppIndexRefreshMeta> {
    let runtime = app_index_runtime();
    loop {
        let mut guard = runtime
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        try_bootstrap_index_from_disk(app, &mut guard);
        let stale = force_refresh || guard.is_stale();
        if !stale {
            return Ok(AppIndexRefreshMeta {
                cache: guard.clone(),
                changed_count: 0,
                rebuilt: false,
            });
        }

        let source_fingerprint = collect_index_source_fingerprint();
        let fingerprint_unchanged = !force_refresh
            && !source_fingerprint.is_empty()
            && guard.source_fingerprint == source_fingerprint
            && !guard.items.is_empty();
        if fingerprint_unchanged {
            guard.refreshed_at = Some(Instant::now());
            return Ok(AppIndexRefreshMeta {
                cache: guard.clone(),
                changed_count: 0,
                rebuilt: false,
            });
        }

        if guard.building {
            guard = runtime
                .condvar
                .wait(guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            continue;
        }

        guard.building = true;
        guard.index_state = AppManagerIndexState::Building;
        let previous_items = guard.items.clone();
        drop(guard);

        let rebuild_result = build_app_index(app);
        let indexed_at = now_unix_seconds();
        let mut guard = runtime
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.building = false;
        match rebuild_result {
            Ok(items) => {
                let changed_count = count_item_changes(previous_items.as_slice(), items.as_slice());
                let changed = changed_count > 0;
                guard.items = items;
                guard.indexed_at = indexed_at;
                guard.refreshed_at = Some(Instant::now());
                guard.source_fingerprint = source_fingerprint;
                guard.index_state = AppManagerIndexState::Ready;
                guard.last_error = None;
                if changed || guard.revision == 0 {
                    guard.revision = guard.revision.saturating_add(1);
                }
                let cache_snapshot = guard.clone();
                runtime.condvar.notify_all();
                persist_index_to_disk(app, &cache_snapshot);
                return Ok(AppIndexRefreshMeta {
                    cache: cache_snapshot,
                    changed_count,
                    rebuilt: true,
                });
            }
            Err(error) => {
                guard.refreshed_at = Some(Instant::now());
                guard.index_state = AppManagerIndexState::Degraded;
                guard.last_error = Some(error.to_string());
                let cache_snapshot = guard.clone();
                runtime.condvar.notify_all();
                if cache_snapshot.items.is_empty() {
                    return Err(error);
                }
                return Ok(AppIndexRefreshMeta {
                    cache: cache_snapshot,
                    changed_count: 0,
                    rebuilt: false,
                });
            }
        }
    }
}

fn load_or_refresh_index(app: &dyn LauncherHost, force_refresh: bool) -> AppResult<AppIndexCache> {
    refresh_index_with_meta(app, force_refresh).map(|value| value.cache)
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
#[path = "../../tests/residue_tests.inc"]
mod residue_tests;

#[cfg(test)]
#[path = "../../tests/path_size_tests.inc"]
mod path_size_tests;

#[cfg(test)]
#[path = "../../tests/display_name_tests.inc"]
mod display_name_tests;

#[cfg(test)]
#[path = "../../tests/query_contract_tests.inc"]
mod query_contract_tests;

#[cfg(test)]
#[path = "../../tests/discovery_tests.inc"]
mod discovery_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../tests/macos_tests.inc"]
mod macos_tests;

use app_core::models::{
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
use app_core::{AppError, AppResult, ResultExt};
use crate::host::LauncherHost;
use crate::launcher::icon::{resolve_application_icon, resolve_builtin_icon};
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

mod api;
mod index;
mod cleanup;
mod residue;
mod startup;
mod uninstall;
mod naming;
mod size;

pub use api::*;
use index::*;
use cleanup::*;
use residue::*;
use startup::*;
use uninstall::*;
use naming::*;
use size::*;

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

fn load_or_refresh_index(app: &dyn LauncherHost, force_refresh: bool) -> AppResult<AppIndexCache> {
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
#[path = "../../../../tests/app/app_manager/residue_tests.rs"]
mod residue_tests;

#[cfg(test)]
#[path = "../../../../tests/app/app_manager/path_size_tests.rs"]
mod path_size_tests;

#[cfg(test)]
#[path = "../../../../tests/app/app_manager/display_name_tests.rs"]
mod display_name_tests;

#[cfg(test)]
#[path = "../../../../tests/app/app_manager/query_contract_tests.rs"]
mod query_contract_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../tests/app/app_manager/macos_tests.rs"]
mod tests;

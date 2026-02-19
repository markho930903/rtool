use crate::app::icon_service::{resolve_application_icon, resolve_builtin_icon};
use crate::core::models::{
    AppManagerActionResultDto, AppManagerCapabilitiesDto, AppManagerCleanupInputDto,
    AppManagerCleanupItemResultDto, AppManagerCleanupResultDto, AppManagerDetailQueryDto,
    AppManagerExportScanInputDto, AppManagerExportScanResultDto, AppManagerIdentityDto,
    AppManagerPageDto, AppManagerQueryDto, AppManagerResidueGroupDto, AppManagerResidueItemDto,
    AppManagerResidueScanInputDto, AppManagerResidueScanResultDto, AppManagerStartupUpdateInputDto,
    AppManagerUninstallInputDto, AppRelatedRootDto, AppSizeSummaryDto, ManagedAppDetailDto,
    ManagedAppDto,
};
use crate::core::{AppError, AppResult};
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
    code: impl Into<String>,
    message: impl Into<String>,
    detail: Option<String>,
) -> AppManagerActionResultDto {
    AppManagerActionResultDto {
        ok,
        code: code.into(),
        message: message.into(),
        detail,
    }
}

fn startup_readonly_reason_code(startup_scope: &str, startup_editable: bool) -> Option<String> {
    if startup_editable {
        return None;
    }
    if startup_scope.eq_ignore_ascii_case("system") {
        return Some("managed_by_policy".to_string());
    }
    Some("permission_denied".to_string())
}

fn try_get_path_size_bytes(path: &Path) -> Option<u64> {
    if !path.exists() {
        return None;
    }
    if path.is_file() {
        return fs::metadata(path).ok().map(|meta| meta.len());
    }

    // Use a lightweight walk for list rendering to avoid blocking large I/O.
    let mut total = 0u64;
    let mut queue = VecDeque::new();
    queue.push_back((path.to_path_buf(), 0usize));
    let mut visited = 0usize;
    while let Some((dir, depth)) = queue.pop_front() {
        if visited >= 2_000 {
            break;
        }
        visited += 1;
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                if depth < 3 {
                    queue.push_back((entry_path, depth + 1));
                }
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    Some(total)
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
    identity_source: &str,
) -> AppManagerIdentityDto {
    AppManagerIdentityDto {
        primary_id: primary_id.into(),
        aliases,
        identity_source: identity_source.to_string(),
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

fn open_with_command(command: &str, args: &[&str], error_code: &str) -> AppResult<()> {
    let status = Command::new(command).args(args).status().map_err(|error| {
        AppError::new(error_code, "系统操作失败").with_detail(error.to_string())
    })?;
    if status.success() {
        return Ok(());
    }
    Err(AppError::new(error_code, "系统操作失败").with_detail(format!("status={status}")))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

#[cfg(test)]
mod residue_tests {
    use super::*;

    fn candidate_with_confidence(confidence: &str, evidence_len: usize) -> ResidueCandidate {
        ResidueCandidate {
            path: PathBuf::from("/tmp/a"),
            scope: "user".to_string(),
            kind: "cache".to_string(),
            exists: true,
            filesystem: true,
            match_reason: "test".to_string(),
            confidence: confidence.to_string(),
            evidence: (0..evidence_len).map(|idx| format!("e{idx}")).collect(),
            risk_level: "low".to_string(),
            recommended: true,
            readonly_reason_code: None,
        }
    }

    #[test]
    fn residue_candidate_replace_prefers_higher_confidence() {
        let current = candidate_with_confidence("high", 1);
        let next = candidate_with_confidence("exact", 1);
        assert!(should_replace_residue_candidate(&current, &next));
        assert!(!should_replace_residue_candidate(&next, &current));
    }

    #[test]
    fn residue_candidate_replace_prefers_more_evidence_when_confidence_equal() {
        let current = candidate_with_confidence("high", 1);
        let next = candidate_with_confidence("high", 2);
        assert!(should_replace_residue_candidate(&current, &next));
    }
}

#[cfg(test)]
mod display_name_tests {
    use super::*;

    #[test]
    fn resolve_display_name_prefers_readable_stem_over_short_alias() {
        let path = Path::new("/Applications/Visual Studio Code.app");
        let mut candidates = Vec::new();
        push_display_name_candidate(&mut candidates, Some("Code".to_string()), 90);
        push_display_name_candidate(&mut candidates, Some("Visual Studio Code".to_string()), 85);

        let name = resolve_application_display_name(
            path,
            "/Applications/Visual Studio Code.app",
            candidates,
        );
        assert_eq!(name, "Visual Studio Code");
    }

    #[test]
    fn resolve_display_name_keeps_short_name_when_stem_is_also_short() {
        let path = Path::new("/Applications/Code.app");
        let mut candidates = Vec::new();
        push_display_name_candidate(&mut candidates, Some("Code".to_string()), 90);
        let name = resolve_application_display_name(path, "/Applications/Code.app", candidates);
        assert_eq!(name, "Code");
    }

    #[test]
    fn resolve_display_name_prefers_windows_registry_display_name() {
        let path = Path::new("/Program Files/Foo/foo.exe");
        let mut candidates = Vec::new();
        push_display_name_candidate(&mut candidates, Some("Foo Enterprise".to_string()), 90);
        push_display_name_candidate(&mut candidates, Some("foo".to_string()), 80);
        let name = resolve_application_display_name(path, "/Program Files/Foo/foo.exe", candidates);
        assert_eq!(name, "Foo Enterprise");
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    fn test_app(bundle_or_app_id: Option<&str>) -> ManagedAppDto {
        ManagedAppDto {
            id: "mac.test.app".to_string(),
            name: "AppCleaner.app".to_string(),
            path: "/Applications/AppCleaner.app".to_string(),
            bundle_or_app_id: bundle_or_app_id.map(ToString::to_string),
            version: None,
            publisher: None,
            platform: "macos".to_string(),
            source: "application".to_string(),
            icon_kind: "iconify".to_string(),
            icon_value: "i-noto:desktop-computer".to_string(),
            estimated_size_bytes: None,
            startup_enabled: false,
            startup_scope: "none".to_string(),
            startup_editable: true,
            readonly_reason_code: None,
            uninstall_supported: true,
            uninstall_kind: None,
            capabilities: build_app_capabilities(true, true, true),
            identity: build_app_identity(
                bundle_or_app_id.unwrap_or("net.freemacsoft.AppCleaner"),
                vec![
                    "AppCleaner".to_string(),
                    "net.freemacsoft.AppCleaner".to_string(),
                ],
                "bundle_id",
            ),
            risk_level: "low".to_string(),
            fingerprint: "fp".to_string(),
        }
    }

    fn has_root_path(roots: &[RelatedRootSpec], expected: &Path) -> bool {
        let expected_key = normalize_path_key(expected.to_string_lossy().as_ref());
        roots
            .iter()
            .any(|root| normalize_path_key(root.path.to_string_lossy().as_ref()) == expected_key)
    }

    #[test]
    fn collect_related_root_specs_includes_http_storages_bundle_path() {
        let app = test_app(Some("net.freemacsoft.AppCleaner"));
        let roots = collect_related_root_specs(&app);
        let home = home_dir().expect("home dir should exist for mac tests");
        let expected = home
            .join("Library")
            .join("HTTPStorages")
            .join("net.freemacsoft.AppCleaner");
        assert!(
            has_root_path(roots.as_slice(), expected.as_path()),
            "expected HTTPStorages path {} to be included",
            expected.to_string_lossy()
        );
    }

    #[test]
    fn collect_related_root_specs_includes_temp_cache_alias_paths() {
        let app = test_app(Some("net.freemacsoft.AppCleaner"));
        let roots = collect_related_root_specs(&app);
        let expected_paths = mac_collect_temp_alias_roots("net.freemacsoft.AppCleaner");
        assert!(!expected_paths.is_empty(), "expected temp candidate paths");
        for expected in expected_paths {
            assert!(
                has_root_path(roots.as_slice(), expected.as_path()),
                "expected temp cache path {} to be included",
                expected.to_string_lossy()
            );
        }
    }
}

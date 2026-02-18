use crate::app::icon_service::{resolve_application_icon, resolve_builtin_icon};
use crate::core::models::{
    AppManagerActionResultDto, AppManagerCleanupInputDto, AppManagerCleanupItemResultDto,
    AppManagerCapabilitiesDto, AppManagerIdentityDto,
    AppManagerCleanupResultDto, AppManagerDetailQueryDto, AppManagerExportScanInputDto,
    AppManagerExportScanResultDto, AppManagerPageDto, AppManagerQueryDto,
    AppManagerResidueGroupDto, AppManagerResidueItemDto, AppManagerResidueScanInputDto,
    AppManagerResidueScanResultDto, AppManagerStartupUpdateInputDto,
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

pub fn list_managed_apps(
    app: &AppHandle,
    query: AppManagerQueryDto,
) -> AppResult<AppManagerPageDto> {
    let cache = load_or_refresh_index(app, false)?;
    let normalized_keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let normalized_category = query
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let startup_only = query.startup_only.unwrap_or(false);
    let limit = query
        .limit
        .map(|value| value as usize)
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);
    let offset = query
        .cursor
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    let mut filtered: Vec<ManagedAppDto> = cache
        .items
        .iter()
        .filter(|item| {
            if startup_only && !item.startup_enabled {
                return false;
            }
            if let Some(category) = normalized_category.as_deref() {
                if category == "rtool" && item.source != "rtool" {
                    return false;
                }
                if category == "application" && item.source != "application" {
                    return false;
                }
                if category == "startup" && !item.startup_enabled {
                    return false;
                }
            }
            if let Some(keyword) = normalized_keyword.as_deref() {
                let name = item.name.to_ascii_lowercase();
                let path = item.path.to_ascii_lowercase();
                let publisher = item
                    .publisher
                    .clone()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if !name.contains(keyword)
                    && !path.contains(keyword)
                    && !publisher.contains(keyword)
                {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    filtered.sort_by(|left, right| {
        right
            .startup_enabled
            .cmp(&left.startup_enabled)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.name.cmp(&right.name))
    });

    let total = filtered.len();
    if offset >= total {
        return Ok(AppManagerPageDto {
            items: Vec::new(),
            next_cursor: None,
            indexed_at: cache.indexed_at,
        });
    }

    let end = offset.saturating_add(limit).min(total);
    let next_cursor = if end < total {
        Some(end.to_string())
    } else {
        None
    };
    let items = filtered[offset..end].to_vec();

    Ok(AppManagerPageDto {
        items,
        next_cursor,
        indexed_at: cache.indexed_at,
    })
}

pub fn refresh_managed_apps_index(app: &AppHandle) -> AppResult<AppManagerActionResultDto> {
    let cache = load_or_refresh_index(app, true)?;
    Ok(make_action_result(
        true,
        "app_manager_refreshed",
        "应用索引已刷新",
        Some(format!("count={}", cache.items.len())),
    ))
}

pub fn set_managed_app_startup(
    app: &AppHandle,
    input: AppManagerStartupUpdateInputDto,
) -> AppResult<AppManagerActionResultDto> {
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    if !item.startup_editable {
        return Err(AppError::new(
            "app_manager_startup_read_only",
            "当前应用启动项为只读，无法修改",
        ));
    }

    platform_set_startup(
        item.id.as_str(),
        Path::new(item.path.as_str()),
        input.enabled,
    )?;
    let _ = load_or_refresh_index(app, true)?;

    let message = if input.enabled {
        "已启用开机启动"
    } else {
        "已关闭开机启动"
    };
    Ok(make_action_result(
        true,
        "app_manager_startup_updated",
        message,
        Some(item.name),
    ))
}

pub fn get_managed_app_detail(
    app: &AppHandle,
    query: AppManagerDetailQueryDto,
) -> AppResult<ManagedAppDetailDto> {
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == query.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    Ok(build_app_detail(item))
}

pub fn scan_managed_app_residue(
    app: &AppHandle,
    input: AppManagerResidueScanInputDto,
) -> AppResult<AppManagerResidueScanResultDto> {
    cleanup_stale_scan_cache();
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    let result = build_residue_scan_result(&item);
    {
        let mut scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache.insert(
            item.id.clone(),
            ResidueScanCacheEntry {
                refreshed_at: Instant::now(),
                result: result.clone(),
            },
        );
    }
    Ok(result)
}

pub fn cleanup_managed_app_residue(
    app: &AppHandle,
    input: AppManagerCleanupInputDto,
) -> AppResult<AppManagerCleanupResultDto> {
    cleanup_stale_scan_cache();
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    let scan_result = {
        let scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache
            .get(item.id.as_str())
            .map(|entry| entry.result.clone())
            .unwrap_or_else(|| build_residue_scan_result(&item))
    };

    let result = execute_cleanup_plan(app, &item, &scan_result, input)?;
    let _ = load_or_refresh_index(app, true)?;
    Ok(result)
}

pub fn export_managed_app_scan_result(
    app: &AppHandle,
    input: AppManagerExportScanInputDto,
) -> AppResult<AppManagerExportScanResultDto> {
    cleanup_stale_scan_cache();
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    let scan_result = {
        let scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache
            .get(item.id.as_str())
            .map(|entry| entry.result.clone())
            .unwrap_or_else(|| build_residue_scan_result(&item))
    };
    let detail = build_app_detail(item.clone());

    let export_dir = export_root_dir();
    fs::create_dir_all(&export_dir).map_err(|error| {
        AppError::new("app_manager_export_dir_failed", "创建导出目录失败")
            .with_detail(error.to_string())
    })?;

    let stem = sanitize_file_stem(item.name.as_str());
    let file_name = format!("{}-{}-scan.json", stem, now_unix_millis());
    let file_path = export_dir.join(file_name);
    let payload = serde_json::json!({
        "exportedAt": now_unix_seconds(),
        "app": item,
        "detail": detail,
        "scanResult": scan_result
    });
    let content = serde_json::to_string_pretty(&payload).map_err(|error| {
        AppError::new("app_manager_export_serialize_failed", "序列化导出内容失败")
            .with_detail(error.to_string())
    })?;
    fs::write(&file_path, content).map_err(|error| {
        AppError::new("app_manager_export_write_failed", "写入导出文件失败")
            .with_detail(error.to_string())
    })?;

    Ok(AppManagerExportScanResultDto {
        app_id: input.app_id,
        file_path: file_path.to_string_lossy().to_string(),
        directory_path: export_dir.to_string_lossy().to_string(),
    })
}

pub fn uninstall_managed_app(
    app: &AppHandle,
    input: AppManagerUninstallInputDto,
) -> AppResult<AppManagerActionResultDto> {
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    if item.fingerprint != input.confirmed_fingerprint {
        return Err(AppError::new(
            "app_manager_fingerprint_mismatch",
            "应用信息已变化，请刷新后重试",
        ));
    }

    if !item.uninstall_supported {
        return Err(AppError::new(
            "app_manager_uninstall_unsupported",
            "该应用不支持在当前平台直接卸载",
        ));
    }

    if item.source == "rtool" {
        return Err(AppError::new(
            "app_manager_uninstall_self_forbidden",
            "不支持卸载当前运行中的应用",
        ));
    }

    platform_uninstall(&item)?;
    let _ = load_or_refresh_index(app, true)?;

    Ok(make_action_result(
        true,
        "app_manager_uninstall_started",
        "已触发系统卸载流程",
        Some(item.name),
    ))
}

pub fn open_uninstall_help(
    app: &AppHandle,
    app_id: String,
) -> AppResult<AppManagerActionResultDto> {
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    platform_open_uninstall_help(&item)?;
    Ok(make_action_result(
        true,
        "app_manager_uninstall_help_opened",
        "已打开系统卸载入口",
        Some(item.name),
    ))
}

#[derive(Debug, Clone)]
struct RelatedRootSpec {
    label: String,
    path: PathBuf,
    scope: String,
    kind: String,
}

#[derive(Debug, Clone)]
struct ResidueCandidate {
    path: PathBuf,
    scope: String,
    kind: String,
    exists: bool,
    filesystem: bool,
    match_reason: String,
    confidence: String,
    evidence: Vec<String>,
    risk_level: String,
    recommended: bool,
    readonly_reason_code: Option<String>,
}

fn push_related_root(
    roots: &mut Vec<RelatedRootSpec>,
    label: impl Into<String>,
    path: PathBuf,
    scope: &str,
    kind: &str,
) {
    roots.push(RelatedRootSpec {
        label: label.into(),
        path,
        scope: scope.to_string(),
        kind: kind.to_string(),
    });
}

fn app_install_root(item: &ManagedAppDto) -> PathBuf {
    let path = PathBuf::from(item.path.as_str());
    if path.is_file() {
        return path.parent().map(Path::to_path_buf).unwrap_or(path);
    }
    path
}

fn build_app_capabilities(startup: bool, uninstall: bool, residue_scan: bool) -> AppManagerCapabilitiesDto {
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

fn collect_app_path_aliases_from_parts(name: &str, path: &str, bundle_or_app_id: Option<&str>) -> Vec<String> {
    let mut aliases = Vec::new();
    let mut seen = HashSet::new();
    let mut push_alias = |value: &str| {
        let normalized = value
            .trim()
            .trim_matches(|ch| matches!(ch, '"' | '\''));
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

#[cfg(target_os = "macos")]
fn mac_is_var_folders_temp_root(path: &Path) -> bool {
    let key = normalize_path_key(path.to_string_lossy().as_ref());
    key.contains("/var/folders/")
}

#[cfg(target_os = "macos")]
fn mac_collect_temp_alias_roots(alias: &str) -> Vec<PathBuf> {
    if alias.trim().is_empty() {
        return Vec::new();
    }
    let temp_root = std::env::temp_dir();
    let mut roots = vec![temp_root.join(alias)];
    if !mac_is_var_folders_temp_root(temp_root.as_path()) {
        return roots;
    }

    let Some(parent) = temp_root.parent() else {
        return roots;
    };
    let Some(leaf) = temp_root.file_name().and_then(|value| value.to_str()) else {
        return roots;
    };
    if leaf.eq_ignore_ascii_case("t") {
        roots.push(parent.join("C").join(alias));
    } else if leaf.eq_ignore_ascii_case("c") {
        roots.push(parent.join("T").join(alias));
    }
    roots
}

fn collect_related_root_specs(item: &ManagedAppDto) -> Vec<RelatedRootSpec> {
    let mut roots = Vec::new();
    let install_root = app_install_root(item);
    let install_scope = home_dir()
        .as_ref()
        .filter(|home| install_root.starts_with(home))
        .map(|_| "user")
        .unwrap_or("system");
    push_related_root(
        &mut roots,
        "安装目录",
        install_root,
        install_scope,
        "install",
    );
    let aliases = collect_app_path_aliases(item);

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            for alias in &aliases {
                push_related_root(
                    &mut roots,
                    "用户应用支持目录",
                    home.join("Library/Application Support").join(alias),
                    "user",
                    "app_support",
                );
                push_related_root(
                    &mut roots,
                    "用户缓存目录",
                    home.join("Library/Caches").join(alias),
                    "user",
                    "cache",
                );
                push_related_root(
                    &mut roots,
                    "用户 HTTP 存储目录",
                    home.join("Library/HTTPStorages").join(alias),
                    "user",
                    "cache",
                );
                for temp_root in mac_collect_temp_alias_roots(alias.as_str()) {
                    push_related_root(&mut roots, "用户临时缓存目录", temp_root, "user", "cache");
                }
                push_related_root(
                    &mut roots,
                    "用户偏好设置",
                    home.join("Library/Preferences").join(format!("{alias}.plist")),
                    "user",
                    "preferences",
                );
                push_related_root(
                    &mut roots,
                    "用户日志目录",
                    home.join("Library/Logs").join(alias),
                    "user",
                    "logs",
                );
            }
            if let Some(startup_path) = mac_startup_file_path(item.id.as_str()) {
                push_related_root(&mut roots, "用户启动项", startup_path, "user", "startup");
            }
        }
        for alias in &aliases {
            push_related_root(
                &mut roots,
                "系统应用支持目录",
                PathBuf::from("/Library/Application Support").join(alias),
                "system",
                "app_support",
            );
            push_related_root(
                &mut roots,
                "系统缓存目录",
                PathBuf::from("/Library/Caches").join(alias),
                "system",
                "cache",
            );
            push_related_root(
                &mut roots,
                "系统偏好设置",
                PathBuf::from("/Library/Preferences").join(format!("{alias}.plist")),
                "system",
                "preferences",
            );
            push_related_root(
                &mut roots,
                "系统日志目录",
                PathBuf::from("/Library/Logs").join(alias),
                "system",
                "logs",
            );
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            for alias in &aliases {
                push_related_root(
                    &mut roots,
                    "Roaming 配置目录",
                    PathBuf::from(&app_data).join(alias),
                    "user",
                    "app_data",
                );
            }
            let startup_name = format!("{}.lnk", item.name);
            push_related_root(
                &mut roots,
                "用户启动项目录",
                PathBuf::from(app_data)
                    .join("Microsoft/Windows/Start Menu/Programs/Startup")
                    .join(startup_name),
                "user",
                "startup",
            );
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            for alias in &aliases {
                push_related_root(
                    &mut roots,
                    "Local 数据目录",
                    PathBuf::from(&local_app_data).join(alias),
                    "user",
                    "app_data",
                );
            }
        }
        if let Some(program_data) = std::env::var_os("ProgramData") {
            for alias in &aliases {
                push_related_root(
                    &mut roots,
                    "ProgramData 目录",
                    PathBuf::from(&program_data).join(alias),
                    "system",
                    "app_data",
                );
            }
            let startup_name = format!("{}.lnk", item.name);
            push_related_root(
                &mut roots,
                "系统启动项目录",
                PathBuf::from(program_data)
                    .join("Microsoft/Windows/Start Menu/Programs/Startup")
                    .join(startup_name),
                "system",
                "startup",
            );
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = aliases;
    }

    let mut dedup = HashSet::new();
    roots
        .into_iter()
        .filter(|root| dedup.insert(normalize_path_key(root.path.to_string_lossy().as_ref())))
        .collect()
}

fn build_app_detail(app: ManagedAppDto) -> ManagedAppDetailDto {
    let related_roots = collect_related_root_specs(&app)
        .into_iter()
        .map(|root| {
            let exists = root.path.exists();
            let mut readonly_reason_code = None;
            let readonly = if exists {
                let is_policy_managed = root.scope == "system" && root.kind == "startup";
                let ro = is_policy_managed || path_is_readonly(root.path.as_path());
                if is_policy_managed {
                    readonly_reason_code = Some("managed_by_policy".to_string());
                } else if ro {
                    readonly_reason_code = Some("permission_denied".to_string());
                }
                ro
            } else {
                false
            };
            AppRelatedRootDto {
                id: stable_hash(
                    format!("{}|{}|{}", app.id, root.kind, root.path.to_string_lossy()).as_str(),
                ),
                label: root.label,
                path: root.path.to_string_lossy().to_string(),
                scope: root.scope,
                kind: root.kind,
                exists,
                readonly,
                readonly_reason_code,
            }
        })
        .collect::<Vec<_>>();

    ManagedAppDetailDto {
        install_path: app.path.clone(),
        size_summary: AppSizeSummaryDto {
            app_bytes: app.estimated_size_bytes,
            residue_bytes: None,
            total_bytes: app.estimated_size_bytes,
        },
        related_roots,
        app,
    }
}

fn path_size_bytes_for_scan(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    if path.is_file() {
        return fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    }
    let mut total = 0u64;
    let mut queue = VecDeque::new();
    queue.push_back((path.to_path_buf(), 0usize));
    let mut visited = 0usize;
    while let Some((dir, depth)) = queue.pop_front() {
        if visited >= 8_000 {
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
                if depth < 8 {
                    queue.push_back((entry_path, depth + 1));
                }
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

fn collect_known_residue_candidates(item: &ManagedAppDto) -> Vec<ResidueCandidate> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if let Some(bundle) = item.bundle_or_app_id.as_deref() {
            if let Some(home) = home_dir() {
                let preference_file = home
                    .join("Library/Preferences")
                    .join(format!("{bundle}.plist"));
                candidates.push(ResidueCandidate {
                    path: preference_file,
                    scope: "user".to_string(),
                    kind: "preferences".to_string(),
                    exists: false,
                    filesystem: true,
                    match_reason: "bundle_id".to_string(),
                    confidence: "exact".to_string(),
                    evidence: vec!["bundle_id_exact".to_string()],
                    risk_level: "medium".to_string(),
                    recommended: true,
                    readonly_reason_code: None,
                });
            }
        }
        if let Some(startup_path) = mac_startup_file_path(item.id.as_str()) {
            candidates.push(ResidueCandidate {
                path: startup_path,
                scope: "user".to_string(),
                kind: "startup".to_string(),
                exists: false,
                filesystem: true,
                match_reason: "startup_label".to_string(),
                confidence: "exact".to_string(),
                evidence: vec!["startup_label_exact".to_string()],
                risk_level: "medium".to_string(),
                recommended: true,
                readonly_reason_code: None,
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let startup_name = format!("{}.lnk", item.name);
            candidates.push(ResidueCandidate {
                path: PathBuf::from(app_data)
                    .join("Microsoft/Windows/Start Menu/Programs/Startup")
                    .join(startup_name),
                scope: "user".to_string(),
                kind: "startup".to_string(),
                exists: false,
                filesystem: true,
                match_reason: "startup_shortcut".to_string(),
                confidence: "high".to_string(),
                evidence: vec!["startup_shortcut_path".to_string()],
                risk_level: "medium".to_string(),
                recommended: true,
                readonly_reason_code: None,
            });
        }

        candidates.extend(windows_collect_registry_residue_candidates(item));
    }

    candidates
}

#[cfg(target_os = "windows")]
fn windows_registry_scope(reg_path: &str) -> &'static str {
    if reg_path.to_ascii_uppercase().starts_with("HKCU\\")
        || reg_path
            .to_ascii_uppercase()
            .starts_with("HKEY_CURRENT_USER\\")
    {
        return "user";
    }
    "system"
}

#[cfg(target_os = "windows")]
fn windows_collect_registry_residue_candidates(item: &ManagedAppDto) -> Vec<ResidueCandidate> {
    let mut candidates = Vec::new();
    let uninstall_entries = windows_list_uninstall_entries();

    if let Some(entry) = windows_find_best_uninstall_entry(
        item.name.as_str(),
        Path::new(item.path.as_str()),
        uninstall_entries.as_slice(),
    ) {
        let scope = windows_registry_scope(entry.registry_key.as_str()).to_string();
        let kind = "registry_key".to_string();
        candidates.push(ResidueCandidate {
            path: PathBuf::from(entry.registry_key),
            scope: scope.clone(),
            kind: kind.clone(),
            exists: true,
            filesystem: false,
            match_reason: "uninstall_registry".to_string(),
            confidence: "exact".to_string(),
            evidence: vec!["uninstall_registry_match".to_string()],
            risk_level: if scope == "system" {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            recommended: scope == "user",
            readonly_reason_code: if scope == "system" {
                Some("managed_by_policy".to_string())
            } else {
                None
            },
        });
    }

    let startup_value_name = windows_startup_value_name(item.id.as_str());
    let startup_key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    if windows_registry_value_exists(startup_key, startup_value_name.as_str()) {
        candidates.push(ResidueCandidate {
            path: PathBuf::from(format!("{startup_key}::{startup_value_name}")),
            scope: "user".to_string(),
            kind: "registry_value".to_string(),
            exists: true,
            filesystem: false,
            match_reason: "startup_registry".to_string(),
            confidence: "exact".to_string(),
            evidence: vec!["startup_registry_value".to_string()],
            risk_level: "medium".to_string(),
            recommended: true,
            readonly_reason_code: None,
        });
    }

    let app_path_key = normalize_path_key(item.path.as_str());
    for root in [
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run",
    ] {
        for (value_name, value_data) in windows_query_registry_values(root) {
            let value_key = normalize_path_key(value_data.as_str());
            let path_match = !app_path_key.is_empty() && value_key.contains(app_path_key.as_str());
            if !path_match {
                continue;
            }
            let scope = windows_registry_scope(root).to_string();
            candidates.push(ResidueCandidate {
                path: PathBuf::from(format!("{root}::{value_name}")),
                scope: scope.clone(),
                kind: "registry_value".to_string(),
                exists: true,
                filesystem: false,
                match_reason: "run_registry".to_string(),
                confidence: "high".to_string(),
                evidence: vec!["run_registry_path_match".to_string()],
                risk_level: if scope == "system" {
                    "high".to_string()
                } else {
                    "medium".to_string()
                },
                recommended: scope == "user",
                readonly_reason_code: if scope == "system" {
                    Some("managed_by_policy".to_string())
                } else {
                    None
                },
            });
        }
    }

    candidates
}

fn group_label(kind: &str, scope: &str) -> String {
    let kind_label = match kind {
        "install" => "安装目录",
        "app_support" => "应用支持目录",
        "cache" => "缓存目录",
        "preferences" => "偏好设置",
        "logs" => "日志目录",
        "startup" => "启动项",
        "app_data" => "应用数据目录",
        "registry_key" => "注册表键",
        "registry_value" => "注册表值",
        _ => "关联目录",
    };
    let scope_label = if scope == "system" {
        "系统级"
    } else {
        "用户级"
    };
    format!("{kind_label} · {scope_label}")
}

fn residue_confidence_rank(value: &str) -> u8 {
    match value {
        "exact" => 3,
        "high" => 2,
        "medium" => 1,
        _ => 0,
    }
}

fn should_replace_residue_candidate(current: &ResidueCandidate, next: &ResidueCandidate) -> bool {
    let current_rank = residue_confidence_rank(current.confidence.as_str());
    let next_rank = residue_confidence_rank(next.confidence.as_str());
    if next_rank != current_rank {
        return next_rank > current_rank;
    }
    if next.evidence.len() != current.evidence.len() {
        return next.evidence.len() > current.evidence.len();
    }
    if next.recommended != current.recommended {
        return next.recommended;
    }
    false
}

fn candidate_from_related_root(root: &RelatedRootSpec) -> Option<ResidueCandidate> {
    if root.kind == "install" {
        return None;
    }
    let risk_level = if root.scope == "system" {
        if root.kind == "startup" {
            "high"
        } else {
            "medium"
        }
    } else if root.kind == "preferences" || root.kind == "startup" {
        "medium"
    } else {
        "low"
    };
    let readonly_reason_code = if root.scope == "system" && root.kind == "startup" {
        Some("managed_by_policy".to_string())
    } else {
        None
    };
    Some(ResidueCandidate {
        path: root.path.clone(),
        scope: root.scope.clone(),
        kind: root.kind.clone(),
        exists: false,
        filesystem: true,
        match_reason: "related_root".to_string(),
        confidence: "exact".to_string(),
        evidence: vec![format!("related_root:{}", root.kind)],
        risk_level: risk_level.to_string(),
        recommended: root.scope == "user",
        readonly_reason_code,
    })
}

fn build_residue_scan_result(item: &ManagedAppDto) -> AppManagerResidueScanResultDto {
    let roots = collect_related_root_specs(item);
    let warnings = Vec::new();
    let mut candidates = roots
        .iter()
        .filter_map(candidate_from_related_root)
        .collect::<Vec<_>>();
    candidates.extend(collect_known_residue_candidates(item));

    let mut dedup = HashMap::<String, ResidueCandidate>::new();
    for candidate in candidates {
        let key = normalize_path_key(candidate.path.to_string_lossy().as_ref());
        if key.is_empty() {
            continue;
        }
        match dedup.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut existing) => {
                if should_replace_residue_candidate(existing.get(), &candidate) {
                    existing.insert(candidate);
                }
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(candidate);
            }
        }
    }

    let mut grouped = HashMap::<String, AppManagerResidueGroupDto>::new();
    let mut total_size_bytes = 0u64;
    for candidate in dedup.into_values() {
        let exists = if candidate.filesystem {
            candidate.path.exists()
        } else {
            candidate.exists
        };
        if !exists {
            continue;
        }
        let path = candidate.path.to_string_lossy().to_string();
        let size_bytes = if candidate.filesystem {
            path_size_bytes_for_scan(Path::new(path.as_str()))
        } else {
            0
        };
        total_size_bytes = total_size_bytes.saturating_add(size_bytes);
        let readonly = if candidate.filesystem {
            candidate.readonly_reason_code.is_some() || path_is_readonly(Path::new(path.as_str()))
        } else {
            candidate.readonly_reason_code.is_some()
        };
        let readonly_reason_code = if candidate.readonly_reason_code.is_some() {
            candidate.readonly_reason_code.clone()
        } else if readonly {
            Some("permission_denied".to_string())
        } else {
            None
        };
        let item_id = stable_hash(format!("{}|{}|{}", item.id, candidate.kind, path).as_str());
        let group_key = format!("{}|{}", candidate.scope, candidate.kind);
        let group = grouped
            .entry(group_key.clone())
            .or_insert_with(|| AppManagerResidueGroupDto {
                group_id: stable_hash(group_key.as_str()),
                label: group_label(candidate.kind.as_str(), candidate.scope.as_str()),
                scope: candidate.scope.clone(),
                kind: candidate.kind.clone(),
                total_size_bytes: 0,
                items: Vec::new(),
            });
        group.total_size_bytes = group.total_size_bytes.saturating_add(size_bytes);
        group.items.push(AppManagerResidueItemDto {
            item_id,
            path,
            kind: candidate.kind,
            scope: candidate.scope,
            size_bytes,
            match_reason: candidate.match_reason,
            confidence: candidate.confidence,
            evidence: candidate.evidence,
            risk_level: candidate.risk_level,
            recommended: candidate.recommended && !readonly,
            readonly,
            readonly_reason_code,
        });
    }

    let mut groups = grouped.into_values().collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        left.scope
            .cmp(&right.scope)
            .then_with(|| left.kind.cmp(&right.kind))
    });
    for group in &mut groups {
        group
            .items
            .sort_by(|left, right| left.path.cmp(&right.path));
    }

    AppManagerResidueScanResultDto {
        app_id: item.id.clone(),
        total_size_bytes,
        groups,
        warnings,
    }
}

fn delete_path_with_mode(path: &Path, delete_mode: &str) -> AppResult<()> {
    match delete_mode {
        "trash" => move_path_to_trash(path),
        "permanent" => {
            if path.is_dir() {
                fs::remove_dir_all(path).map_err(|error| {
                    AppError::new("app_manager_cleanup_delete_failed", "删除目录失败")
                        .with_detail(error.to_string())
                })
            } else {
                fs::remove_file(path).map_err(|error| {
                    AppError::new("app_manager_cleanup_delete_failed", "删除文件失败")
                        .with_detail(error.to_string())
                })
            }
        }
        _ => Err(AppError::new(
            "app_manager_cleanup_mode_invalid",
            "不支持的删除模式",
        )),
    }
}

#[cfg(target_os = "macos")]
fn move_path_to_trash(path: &Path) -> AppResult<()> {
    let path_value = path.to_string_lossy().to_string();
    let script = format!(
        "tell application \"Finder\" to delete POSIX file \"{}\"",
        applescript_escape(path_value.as_str())
    );
    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|error| {
            AppError::new("app_manager_cleanup_delete_failed", "移入废纸篓失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }
    Err(
        AppError::new("app_manager_cleanup_delete_failed", "移入废纸篓失败")
            .with_detail(format!("status={status}")),
    )
}

#[cfg(target_os = "windows")]
fn move_path_to_trash(path: &Path) -> AppResult<()> {
    let escaped = windows_powershell_escape(path.to_string_lossy().as_ref());
    let script = format!(
        "Add-Type -AssemblyName Microsoft.VisualBasic; \
         $path='{}'; \
         if (Test-Path $path) {{ \
           $item = Get-Item -LiteralPath $path -ErrorAction SilentlyContinue; \
           if ($null -ne $item -and $item.PSIsContainer) {{ \
             [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory($path, 'OnlyErrorDialogs', 'SendToRecycleBin'); \
           }} else {{ \
             [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile($path, 'OnlyErrorDialogs', 'SendToRecycleBin'); \
           }} \
         }}",
        escaped
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", script.as_str()])
        .status()
        .map_err(|error| {
            AppError::new("app_manager_cleanup_delete_failed", "移入回收站失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }
    Err(
        AppError::new("app_manager_cleanup_delete_failed", "移入回收站失败")
            .with_detail(format!("status={status}")),
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn move_path_to_trash(path: &Path) -> AppResult<()> {
    let _ = path;
    Err(AppError::new(
        "app_manager_cleanup_delete_failed",
        "当前平台不支持移入废纸篓",
    ))
}

#[cfg(target_os = "windows")]
fn windows_registry_key_exists(reg_key: &str) -> bool {
    Command::new("reg")
        .args(["query", reg_key])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn windows_delete_registry_key(reg_key: &str) -> AppResult<()> {
    if !windows_registry_key_exists(reg_key) {
        return Err(AppError::new(
            "app_manager_cleanup_not_found",
            "注册表键不存在",
        ));
    }
    let status = Command::new("reg")
        .args(["delete", reg_key, "/f"])
        .status()
        .map_err(|error| {
            AppError::new("app_manager_cleanup_delete_failed", "删除注册表键失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }
    Err(
        AppError::new("app_manager_cleanup_delete_failed", "删除注册表键失败")
            .with_detail(format!("status={status}")),
    )
}

#[cfg(target_os = "windows")]
fn windows_delete_registry_value(spec: &str) -> AppResult<()> {
    let (reg_key, value_name) = spec
        .rsplit_once("::")
        .ok_or_else(|| AppError::new("app_manager_cleanup_path_invalid", "注册表值路径格式无效"))?;
    if !windows_registry_value_exists(reg_key, value_name) {
        return Err(AppError::new(
            "app_manager_cleanup_not_found",
            "注册表值不存在",
        ));
    }
    let status = Command::new("reg")
        .args(["delete", reg_key, "/v", value_name, "/f"])
        .status()
        .map_err(|error| {
            AppError::new("app_manager_cleanup_delete_failed", "删除注册表值失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }
    Err(
        AppError::new("app_manager_cleanup_delete_failed", "删除注册表值失败")
            .with_detail(format!("status={status}")),
    )
}

fn delete_residue_item(item_kind: &str, item_path: &str, delete_mode: &str) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        match item_kind {
            "registry_key" => return windows_delete_registry_key(item_path),
            "registry_value" => return windows_delete_registry_value(item_path),
            _ => {}
        }
    }

    #[cfg(not(target_os = "windows"))]
    if matches!(item_kind, "registry_key" | "registry_value") {
        return Err(AppError::new(
            "app_manager_cleanup_not_supported",
            "当前平台不支持注册表清理",
        ));
    }

    delete_path_with_mode(Path::new(item_path), delete_mode)
}

fn execute_cleanup_plan(
    app: &AppHandle,
    app_item: &ManagedAppDto,
    scan_result: &AppManagerResidueScanResultDto,
    input: AppManagerCleanupInputDto,
) -> AppResult<AppManagerCleanupResultDto> {
    let delete_mode = input.delete_mode.to_ascii_lowercase();
    if delete_mode != "trash" && delete_mode != "permanent" {
        return Err(AppError::new(
            "app_manager_cleanup_mode_invalid",
            "删除模式仅支持 trash 或 permanent",
        ));
    }
    let skip_on_error = input.skip_on_error.unwrap_or(true);
    let mut released_size_bytes = 0u64;
    let mut deleted = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    if input.include_main_app {
        if app_item.source == "rtool" {
            skipped.push(AppManagerCleanupItemResultDto {
                item_id: "main-app".to_string(),
                path: app_item.path.clone(),
                kind: "main_app".to_string(),
                status: "skipped".to_string(),
                reason_code: "self_uninstall_forbidden".to_string(),
                message: "当前运行中的应用不可在此流程卸载".to_string(),
                size_bytes: app_item.estimated_size_bytes,
            });
        } else {
            let confirmed_fingerprint = input.confirmed_fingerprint.clone().ok_or_else(|| {
                AppError::new("app_manager_fingerprint_missing", "缺少应用确认指纹")
            })?;
            if confirmed_fingerprint != app_item.fingerprint {
                return Err(AppError::new(
                    "app_manager_fingerprint_mismatch",
                    "应用信息已变化，请刷新后重试",
                ));
            }
            match platform_uninstall(app_item) {
                Ok(_) => {
                    released_size_bytes = released_size_bytes
                        .saturating_add(app_item.estimated_size_bytes.unwrap_or(0));
                    deleted.push(AppManagerCleanupItemResultDto {
                        item_id: "main-app".to_string(),
                        path: app_item.path.clone(),
                        kind: "main_app".to_string(),
                        status: "deleted".to_string(),
                        reason_code: "ok".to_string(),
                        message: "主程序卸载流程已执行".to_string(),
                        size_bytes: app_item.estimated_size_bytes,
                    });
                }
                Err(error) => {
                    let detail = error
                        .detail
                        .as_deref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| error.message.clone());
                    failed.push(AppManagerCleanupItemResultDto {
                        item_id: "main-app".to_string(),
                        path: app_item.path.clone(),
                        kind: "main_app".to_string(),
                        status: "failed".to_string(),
                        reason_code: error.code,
                        message: detail,
                        size_bytes: app_item.estimated_size_bytes,
                    });
                    if !skip_on_error {
                        return Err(AppError::new(
                            "app_manager_cleanup_failed",
                            "主程序卸载失败，已中止清理",
                        ));
                    }
                }
            }
        }
    }

    let selected = input
        .selected_item_ids
        .iter()
        .map(|value| value.as_str())
        .collect::<HashSet<_>>();
    for group in &scan_result.groups {
        for item in &group.items {
            if !selected.contains(item.item_id.as_str()) {
                continue;
            }

            if item
                .readonly_reason_code
                .as_deref()
                .is_some_and(|reason| reason == "managed_by_policy")
            {
                skipped.push(AppManagerCleanupItemResultDto {
                    item_id: item.item_id.clone(),
                    path: item.path.clone(),
                    kind: item.kind.clone(),
                    status: "skipped".to_string(),
                    reason_code: "managed_by_policy".to_string(),
                    message: "系统策略托管项，已跳过".to_string(),
                    size_bytes: Some(item.size_bytes),
                });
                continue;
            }

            let is_registry_item = matches!(item.kind.as_str(), "registry_key" | "registry_value");
            if !is_registry_item && !Path::new(item.path.as_str()).exists() {
                skipped.push(AppManagerCleanupItemResultDto {
                    item_id: item.item_id.clone(),
                    path: item.path.clone(),
                    kind: item.kind.clone(),
                    status: "skipped".to_string(),
                    reason_code: "not_found".to_string(),
                    message: "路径不存在，已跳过".to_string(),
                    size_bytes: Some(item.size_bytes),
                });
                continue;
            }

            match delete_residue_item(item.kind.as_str(), item.path.as_str(), delete_mode.as_str())
            {
                Ok(_) => {
                    released_size_bytes = released_size_bytes.saturating_add(item.size_bytes);
                    deleted.push(AppManagerCleanupItemResultDto {
                        item_id: item.item_id.clone(),
                        path: item.path.clone(),
                        kind: item.kind.clone(),
                        status: "deleted".to_string(),
                        reason_code: "ok".to_string(),
                        message: "删除成功".to_string(),
                        size_bytes: Some(item.size_bytes),
                    });
                }
                Err(error) => {
                    let detail = error
                        .detail
                        .as_deref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| error.message.clone());
                    failed.push(AppManagerCleanupItemResultDto {
                        item_id: item.item_id.clone(),
                        path: item.path.clone(),
                        kind: item.kind.clone(),
                        status: "failed".to_string(),
                        reason_code: error.code,
                        message: detail,
                        size_bytes: Some(item.size_bytes),
                    });
                    if !skip_on_error {
                        return Err(AppError::new(
                            "app_manager_cleanup_failed",
                            "残留清理失败，已按配置中止",
                        ));
                    }
                }
            }
        }
    }

    {
        let mut scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache.remove(app_item.id.as_str());
    }
    let _ = app;
    Ok(AppManagerCleanupResultDto {
        app_id: app_item.id.clone(),
        delete_mode,
        released_size_bytes,
        deleted,
        skipped,
        failed,
    })
}

fn build_app_index(app: &AppHandle) -> AppResult<Vec<ManagedAppDto>> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    if let Some(self_item) = build_self_item(app) {
        seen.insert(normalize_path_key(self_item.path.as_str()));
        items.push(self_item);
    }

    for item in collect_platform_apps(app) {
        let key = normalize_path_key(item.path.as_str());
        if seen.insert(key) {
            items.push(item);
        }
    }

    items.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(items)
}

fn build_self_item(app: &AppHandle) -> Option<ManagedAppDto> {
    let executable = std::env::current_exe().ok()?;
    let app_name = app.package_info().name.to_string();
    let app_path = executable.to_string_lossy().to_string();
    let id = stable_app_id("rtool", app_path.as_str());
    let (startup_enabled, startup_scope, startup_editable) =
        platform_detect_startup_state(id.as_str(), executable.as_path());
    let readonly_reason_code =
        startup_readonly_reason_code(startup_scope.as_str(), startup_editable);
    let icon = resolve_builtin_icon("i-noto:rocket");
    let bundle_or_app_id = Some(app.package_info().name.to_string());
    let aliases =
        collect_app_path_aliases_from_parts(app_name.as_str(), app_path.as_str(), bundle_or_app_id.as_deref());
    let identity = build_app_identity(
        normalize_path_key(app_path.as_str()),
        aliases,
        "path",
    );

    let mut item = ManagedAppDto {
        id,
        name: app_name.clone(),
        path: app_path,
        bundle_or_app_id,
        version: Some(app.package_info().version.to_string()),
        publisher: None,
        platform: platform_name().to_string(),
        source: "rtool".to_string(),
        icon_kind: icon.kind,
        icon_value: icon.value,
        estimated_size_bytes: try_get_path_size_bytes(executable.as_path()),
        startup_enabled,
        startup_scope,
        startup_editable,
        readonly_reason_code,
        uninstall_supported: false,
        uninstall_kind: None,
        capabilities: build_app_capabilities(
            cfg!(target_os = "macos") || cfg!(target_os = "windows"),
            false,
            true,
        ),
        identity,
        risk_level: "high".to_string(),
        fingerprint: String::new(),
    };
    item.fingerprint = fingerprint_for_app(&item);
    Some(item)
}

fn platform_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "unknown"
    }
}

fn collect_platform_apps(app: &AppHandle) -> Vec<ManagedAppDto> {
    #[cfg(target_os = "macos")]
    {
        collect_macos_apps(app)
    }
    #[cfg(target_os = "windows")]
    {
        collect_windows_apps(app)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app;
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn collect_macos_apps(app: &AppHandle) -> Vec<ManagedAppDto> {
    let mut items = Vec::new();
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    for root in mac_application_roots() {
        queue.push_back((root, 0usize));
    }

    while let Some((dir, depth)) = queue.pop_front() {
        if items.len() >= MAC_SCAN_MAX_ITEMS {
            break;
        }

        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if items.len() >= MAC_SCAN_MAX_ITEMS {
                break;
            }

            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if !file_type.is_dir() {
                continue;
            }

            let path_key = normalize_path_key(path.to_string_lossy().as_ref());
            if seen.contains(&path_key) {
                continue;
            }

            if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("app"))
            {
                if let Some(item) = build_macos_app_item(app, &path) {
                    seen.insert(path_key);
                    items.push(item);
                }
                continue;
            }

            let hidden = path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with('.'));
            if hidden {
                continue;
            }

            if depth < 3 {
                queue.push_back((path, depth + 1));
            }
        }
    }

    items
}

#[cfg(target_os = "macos")]
fn mac_application_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = home_dir() {
        roots.push(home.join("Applications"));
    }
    roots
}

#[cfg(target_os = "macos")]
fn build_macos_app_item(app: &AppHandle, app_path: &Path) -> Option<ManagedAppDto> {
    let path_str = app_path.to_string_lossy().to_string();
    let info = parse_macos_info_plist(app_path.join("Contents").join("Info.plist").as_path());
    let bundle = info.bundle_id.clone();
    let version = info.version.clone();
    let publisher = info.publisher.clone();
    let name = info
        .display_name
        .clone()
        .or_else(|| {
            app_path
                .file_stem()
                .map(|value| value.to_string_lossy().to_string())
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| path_str.clone());
    let id = stable_app_id("application", path_str.as_str());
    let icon = resolve_application_icon(app, app_path);
    let (startup_enabled, startup_scope, startup_editable) =
        platform_detect_startup_state(id.as_str(), app_path);
    let readonly_reason_code =
        startup_readonly_reason_code(startup_scope.as_str(), startup_editable);
    let aliases = collect_app_path_aliases_from_parts(name.as_str(), path_str.as_str(), bundle.as_deref());
    let identity = if let Some(bundle_id) = bundle.as_deref() {
        build_app_identity(bundle_id, aliases, "bundle_id")
    } else {
        build_app_identity(normalize_path_key(path_str.as_str()), aliases, "path")
    };
    let mut item = ManagedAppDto {
        id,
        name,
        path: path_str,
        bundle_or_app_id: bundle,
        version,
        publisher,
        platform: "macos".to_string(),
        source: "application".to_string(),
        icon_kind: icon.kind,
        icon_value: icon.value,
        estimated_size_bytes: try_get_path_size_bytes(app_path),
        startup_enabled,
        startup_scope,
        startup_editable,
        readonly_reason_code,
        uninstall_supported: true,
        uninstall_kind: Some("finder_trash".to_string()),
        capabilities: build_app_capabilities(true, true, true),
        identity,
        risk_level: "medium".to_string(),
        fingerprint: String::new(),
    };
    item.fingerprint = fingerprint_for_app(&item);
    Some(item)
}

#[cfg(target_os = "macos")]
struct MacAppInfo {
    display_name: Option<String>,
    bundle_id: Option<String>,
    version: Option<String>,
    publisher: Option<String>,
}

#[cfg(target_os = "macos")]
fn parse_macos_info_plist(path: &Path) -> MacAppInfo {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => {
            return MacAppInfo {
                display_name: None,
                bundle_id: None,
                version: None,
                publisher: None,
            };
        }
    };

    let display_name = plist_value(content.as_str(), "CFBundleDisplayName")
        .or_else(|| plist_value(content.as_str(), "CFBundleName"));
    let bundle_id = plist_value(content.as_str(), "CFBundleIdentifier");
    let version = plist_value(content.as_str(), "CFBundleShortVersionString")
        .or_else(|| plist_value(content.as_str(), "CFBundleVersion"));
    let publisher = bundle_id
        .as_deref()
        .and_then(|value| value.split('.').next())
        .map(ToString::to_string)
        .filter(|value| !value.is_empty());

    MacAppInfo {
        display_name,
        bundle_id,
        version,
        publisher,
    }
}

#[cfg(target_os = "macos")]
fn plist_value(content: &str, key: &str) -> Option<String> {
    let pattern = format!(
        r"<key>{}</key>\s*<string>([^<]+)</string>",
        regex::escape(key)
    );
    let regex = Regex::new(pattern.as_str()).ok()?;
    regex
        .captures(content)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().trim().to_string())
}

#[cfg(target_os = "windows")]
fn collect_windows_apps(app: &AppHandle) -> Vec<ManagedAppDto> {
    let uninstall_entries = windows_list_uninstall_entries();
    let mut seen_path_keys = HashSet::new();
    let mut seen_identity_keys = HashSet::new();
    let mut items = windows_collect_apps_from_uninstall_entries(
        app,
        uninstall_entries.as_slice(),
        &mut seen_path_keys,
        &mut seen_identity_keys,
    );
    for root in windows_application_roots() {
        scan_windows_root(
            root.as_path(),
            4,
            WIN_SCAN_MAX_ITEMS,
            &mut items,
            &mut seen_path_keys,
            &mut seen_identity_keys,
            app,
            uninstall_entries.as_slice(),
        );
        if items.len() >= WIN_SCAN_MAX_ITEMS {
            break;
        }
    }
    items
}

#[cfg(target_os = "windows")]
fn windows_normalize_registry_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
fn windows_is_generic_uninstall_binary(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    matches!(file_name.as_str(), "msiexec.exe" | "rundll32.exe" | "cmd.exe")
}

#[cfg(target_os = "windows")]
fn windows_extract_executable_from_command(command: &str) -> Option<PathBuf> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    let raw = if let Some(quoted) = trimmed.strip_prefix('"') {
        let end = quoted.find('"')?;
        quoted[..end].to_string()
    } else {
        trimmed
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches('"')
            .to_string()
    };
    if raw.trim().is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    if windows_is_generic_uninstall_binary(path.as_path()) {
        return None;
    }
    Some(path)
}

#[cfg(target_os = "windows")]
fn windows_discovery_path_from_uninstall_entry(entry: &WindowsUninstallEntry) -> Option<PathBuf> {
    if let Some(location) = entry.install_location.as_deref() {
        let location = location.trim().trim_matches('"');
        if !location.is_empty() {
            return Some(PathBuf::from(location));
        }
    }
    entry
        .quiet_uninstall_string
        .as_deref()
        .and_then(windows_extract_executable_from_command)
        .or_else(|| {
            entry
                .uninstall_string
                .as_deref()
                .and_then(windows_extract_executable_from_command)
        })
}

#[cfg(target_os = "windows")]
fn windows_uninstall_entry_matches_path(entry: &WindowsUninstallEntry, app_path: &Path) -> bool {
    let app_path_key = normalize_path_key(app_path.to_string_lossy().as_ref());
    if app_path_key.is_empty() {
        return false;
    }

    if let Some(location) = entry.install_location.as_deref() {
        let install_key = normalize_path_key(location);
        if !install_key.is_empty()
            && (app_path_key.starts_with(install_key.as_str())
                || install_key.starts_with(app_path_key.as_str()))
        {
            return true;
        }
    }

    for command in [
        entry.quiet_uninstall_string.as_deref(),
        entry.uninstall_string.as_deref(),
    ] {
        let Some(command) = command else {
            continue;
        };
        if let Some(command_exe) = windows_extract_executable_from_command(command) {
            let command_key = normalize_path_key(command_exe.to_string_lossy().as_ref());
            if !command_key.is_empty()
                && (app_path_key.starts_with(command_key.as_str())
                    || command_key.starts_with(app_path_key.as_str()))
            {
                return true;
            }
        } else if normalize_path_key(command).contains(app_path_key.as_str()) {
            return true;
        }
    }

    false
}

#[cfg(target_os = "windows")]
fn windows_build_item_from_uninstall_entry(
    app: &AppHandle,
    entry: &WindowsUninstallEntry,
    path: &Path,
) -> ManagedAppDto {
    let path_str = path.to_string_lossy().to_string();
    let id = stable_app_id("application", path_str.as_str());
    let icon = resolve_application_icon(app, path);
    let (startup_enabled, startup_scope, startup_editable) =
        platform_detect_startup_state(id.as_str(), path);
    let readonly_reason_code = startup_readonly_reason_code(startup_scope.as_str(), startup_editable);
    let aliases = collect_app_path_aliases_from_parts(entry.display_name.as_str(), path_str.as_str(), None);

    let mut item = ManagedAppDto {
        id,
        name: entry.display_name.clone(),
        path: path_str,
        bundle_or_app_id: None,
        version: entry.display_version.clone(),
        publisher: entry.publisher.clone(),
        platform: "windows".to_string(),
        source: "application".to_string(),
        icon_kind: icon.kind,
        icon_value: icon.value,
        estimated_size_bytes: try_get_path_size_bytes(path),
        startup_enabled,
        startup_scope,
        startup_editable,
        readonly_reason_code,
        uninstall_supported: true,
        uninstall_kind: Some("registry_command".to_string()),
        capabilities: build_app_capabilities(true, true, true),
        identity: build_app_identity(entry.registry_key.as_str(), aliases, "registry"),
        risk_level: "medium".to_string(),
        fingerprint: String::new(),
    };
    item.fingerprint = fingerprint_for_app(&item);
    item
}

#[cfg(target_os = "windows")]
fn windows_collect_apps_from_uninstall_entries(
    app: &AppHandle,
    entries: &[WindowsUninstallEntry],
    seen_path_keys: &mut HashSet<String>,
    seen_identity_keys: &mut HashSet<String>,
) -> Vec<ManagedAppDto> {
    let mut items = Vec::new();
    for entry in entries {
        if items.len() >= WIN_SCAN_MAX_ITEMS {
            break;
        }
        let Some(path) = windows_discovery_path_from_uninstall_entry(entry) else {
            continue;
        };
        let path_key = normalize_path_key(path.to_string_lossy().as_ref());
        if path_key.is_empty() || !seen_path_keys.insert(path_key) {
            continue;
        }
        let identity_key = windows_normalize_registry_key(entry.registry_key.as_str());
        if !seen_identity_keys.insert(identity_key) {
            continue;
        }
        items.push(windows_build_item_from_uninstall_entry(app, entry, path.as_path()));
    }
    items
}

#[cfg(target_os = "windows")]
fn scan_windows_root(
    root: &Path,
    max_depth: usize,
    max_items: usize,
    items: &mut Vec<ManagedAppDto>,
    seen_path_keys: &mut HashSet<String>,
    seen_identity_keys: &mut HashSet<String>,
    app: &AppHandle,
    uninstall_entries: &[WindowsUninstallEntry],
) {
    if !root.exists() {
        return;
    }

    let mut queue = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0usize));
    while let Some((dir, depth)) = queue.pop_front() {
        if items.len() >= max_items {
            break;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if items.len() >= max_items {
                break;
            }

            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                if depth < max_depth {
                    queue.push_back((path, depth + 1));
                }
                continue;
            }

            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase())
                .unwrap_or_default();
            if !matches!(ext.as_str(), "exe" | "appref-ms") {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();
            let name = path
                .file_stem()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.clone());
            let uninstall_match =
                windows_find_best_uninstall_entry(name.as_str(), path.as_path(), uninstall_entries);
            let Some(uninstall_match) = uninstall_match else {
                continue;
            };
            if !windows_uninstall_entry_matches_path(&uninstall_match, path.as_path()) {
                continue;
            }

            let identity_key = windows_normalize_registry_key(uninstall_match.registry_key.as_str());
            if seen_identity_keys.contains(identity_key.as_str()) {
                continue;
            }
            let path_key = normalize_path_key(path_str.as_str());
            if path_key.is_empty() || seen_path_keys.contains(path_key.as_str()) {
                continue;
            }
            let item = windows_build_item_from_uninstall_entry(app, &uninstall_match, path.as_path());
            seen_identity_keys.insert(identity_key);
            seen_path_keys.insert(path_key);
            items.push(item);
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
struct WindowsUninstallEntry {
    display_name: String,
    uninstall_string: Option<String>,
    quiet_uninstall_string: Option<String>,
    install_location: Option<String>,
    publisher: Option<String>,
    display_version: Option<String>,
    registry_key: String,
}

#[cfg(target_os = "windows")]
fn windows_uninstall_roots() -> [&'static str; 3] {
    [
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ]
}

#[cfg(target_os = "windows")]
fn windows_list_uninstall_entries() -> Vec<WindowsUninstallEntry> {
    let mut entries = Vec::new();
    let mut seen_keys = HashSet::new();
    for root in windows_uninstall_roots() {
        for entry in windows_query_uninstall_root(root) {
            let dedup_key = format!(
                "{}|{}",
                entry.display_name.to_ascii_lowercase(),
                entry.registry_key.to_ascii_lowercase()
            );
            if seen_keys.insert(dedup_key) {
                entries.push(entry);
            }
        }
    }
    entries
}

#[cfg(target_os = "windows")]
fn windows_query_uninstall_root(root: &str) -> Vec<WindowsUninstallEntry> {
    let output = match Command::new("reg").args(["query", root, "/s"]).output() {
        Ok(output) => output,
        Err(error) => {
            tracing::debug!(
                event = "app_manager_windows_reg_query_failed",
                root = root,
                error = error.to_string()
            );
            return Vec::new();
        }
    };
    if !output.status.success() {
        tracing::debug!(
            event = "app_manager_windows_reg_query_failed",
            root = root,
            status = format!("{}", output.status)
        );
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    let mut current_key: Option<String> = None;
    let mut values: HashMap<String, String> = HashMap::new();

    let flush_current = |entries: &mut Vec<WindowsUninstallEntry>,
                         current_key: &Option<String>,
                         values: &HashMap<String, String>| {
        let Some(key) = current_key else {
            return;
        };
        let Some(display_name) = values
            .get("DisplayName")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        else {
            return;
        };

        let uninstall_string = values
            .get("UninstallString")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let quiet_uninstall_string = values
            .get("QuietUninstallString")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let install_location = values
            .get("InstallLocation")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let publisher = values
            .get("Publisher")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let display_version = values
            .get("DisplayVersion")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        entries.push(WindowsUninstallEntry {
            display_name: display_name.to_string(),
            uninstall_string,
            quiet_uninstall_string,
            install_location,
            publisher,
            display_version,
            registry_key: key.clone(),
        });
    };

    for raw_line in stdout.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() {
            continue;
        }

        if !raw_line.starts_with(' ') && line.starts_with("HKEY_") {
            flush_current(&mut entries, &current_key, &values);
            current_key = Some(line.trim().to_string());
            values.clear();
            continue;
        }

        if current_key.is_none() {
            continue;
        }

        if let Some((name, value)) = windows_parse_reg_value_line(line.trim_start()) {
            values.insert(name, value);
        }
    }
    flush_current(&mut entries, &current_key, &values);
    entries
}

#[cfg(target_os = "windows")]
fn windows_parse_reg_value_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split_whitespace();
    let name = parts.next()?;
    let type_name = parts.next()?;
    if !type_name.starts_with("REG_") {
        return None;
    }
    let start = line.find(type_name)? + type_name.len();
    let value = line[start..].trim().to_string();
    Some((name.to_string(), value))
}

#[cfg(target_os = "windows")]
fn windows_query_registry_values(root: &str) -> Vec<(String, String)> {
    let output = match Command::new("reg").args(["query", root]).output() {
        Ok(output) => output,
        Err(_) => return Vec::new(),
    };
    if !output.status.success() {
        return Vec::new();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| windows_parse_reg_value_line(line.trim_start()))
        .collect()
}

#[cfg(target_os = "windows")]
fn windows_registry_value_exists(root: &str, value_name: &str) -> bool {
    Command::new("reg")
        .args(["query", root, "/v", value_name])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn windows_find_best_uninstall_entry(
    app_name: &str,
    app_path: &Path,
    entries: &[WindowsUninstallEntry],
) -> Option<WindowsUninstallEntry> {
    let app_path_key = normalize_path_key(app_path.to_string_lossy().as_ref());
    let app_name_key = app_name.trim().to_ascii_lowercase();

    let mut best_score = 0i32;
    let mut best_has_path_evidence = false;
    let mut best: Option<&WindowsUninstallEntry> = None;
    for entry in entries {
        let mut score = 0i32;
        let mut has_path_evidence = false;
        let display_name_key = entry.display_name.to_ascii_lowercase();
        if display_name_key == app_name_key {
            score += 120;
        } else if display_name_key.contains(app_name_key.as_str())
            || app_name_key.contains(display_name_key.as_str())
        {
            score += 80;
        }

        if let Some(location) = entry.install_location.as_deref() {
            let install_key = normalize_path_key(location);
            if !install_key.is_empty()
                && (app_path_key.starts_with(install_key.as_str())
                    || install_key.starts_with(app_path_key.as_str()))
            {
                score += 140;
                has_path_evidence = true;
            }
        }

        for command in [
            entry.quiet_uninstall_string.as_deref(),
            entry.uninstall_string.as_deref(),
        ] {
            let Some(command) = command else {
                continue;
            };
            if command.trim().is_empty() {
                continue;
            }
            score += 12;
            if let Some(command_path) = windows_extract_executable_from_command(command) {
                let command_key = normalize_path_key(command_path.to_string_lossy().as_ref());
                if !command_key.is_empty()
                    && (app_path_key.starts_with(command_key.as_str())
                        || command_key.starts_with(app_path_key.as_str()))
                {
                    score += 90;
                    has_path_evidence = true;
                }
            } else if normalize_path_key(command).contains(app_path_key.as_str()) {
                score += 60;
                has_path_evidence = true;
            }
        }

        if score > best_score || (score == best_score && has_path_evidence && !best_has_path_evidence) {
            best_score = score;
            best_has_path_evidence = has_path_evidence;
            best = Some(entry);
        }
    }

    if best_score >= 120 && best_has_path_evidence {
        return best.cloned();
    }
    None
}

#[cfg(target_os = "windows")]
fn windows_application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(app_data).join("Microsoft/Windows/Start Menu/Programs"));
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(program_data).join("Microsoft/Windows/Start Menu/Programs"));
    }
    roots
}

fn platform_detect_startup_state(app_id: &str, app_path: &Path) -> (bool, String, bool) {
    #[cfg(target_os = "macos")]
    {
        let user_label_enabled = mac_startup_file_path(app_id).is_some_and(|path| path.exists());
        let cache = mac_get_startup_cache_snapshot();
        let target = app_path.to_string_lossy().to_ascii_lowercase();
        let escaped_target = xml_escape(app_path.to_string_lossy().as_ref()).to_ascii_lowercase();
        let user_match = cache
            .user_plist_blobs
            .iter()
            .any(|blob| blob.contains(target.as_str()) || blob.contains(escaped_target.as_str()));
        let system_match = cache
            .system_plist_blobs
            .iter()
            .any(|blob| blob.contains(target.as_str()) || blob.contains(escaped_target.as_str()));

        if system_match {
            return (true, "system".to_string(), false);
        }
        if user_label_enabled || user_match {
            return (true, "user".to_string(), true);
        }
        (false, "none".to_string(), true)
    }
    #[cfg(target_os = "windows")]
    {
        let user_enabled = windows_startup_enabled(app_id)
            || windows_run_registry_contains(
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                app_path,
            );
        let system_enabled = windows_run_registry_contains(
            r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run",
            app_path,
        );
        if system_enabled {
            return (true, "system".to_string(), false);
        }
        if user_enabled {
            return (true, "user".to_string(), true);
        }
        (false, "none".to_string(), true)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app_id;
        let _ = app_path;
        (false, "none".to_string(), false)
    }
}

fn platform_set_startup(app_id: &str, app_path: &Path, enabled: bool) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        return mac_set_startup(app_id, app_path, enabled);
    }
    #[cfg(target_os = "windows")]
    {
        return windows_set_startup(app_id, app_path, enabled);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app_id;
        let _ = app_path;
        let _ = enabled;
        Err(AppError::new(
            "app_manager_startup_not_supported",
            "当前平台暂不支持启动项修改",
        ))
    }
}

#[cfg(target_os = "macos")]
fn mac_startup_file_path(app_id: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    let label = startup_label(app_id);
    Some(
        home.join("Library")
            .join("LaunchAgents")
            .join(format!("{label}.plist")),
    )
}

#[cfg(target_os = "macos")]
fn mac_get_startup_cache_snapshot() -> MacStartupCache {
    let stale = {
        let cache = mac_startup_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.is_stale()
    };
    if stale {
        let user_blobs = home_dir()
            .map(|home| home.join("Library").join("LaunchAgents"))
            .map(|path| mac_collect_plist_blobs(path.as_path()))
            .unwrap_or_default();
        let mut system_blobs = Vec::new();
        system_blobs.extend(mac_collect_plist_blobs(Path::new("/Library/LaunchAgents")));
        system_blobs.extend(mac_collect_plist_blobs(Path::new("/Library/LaunchDaemons")));

        let mut cache = mac_startup_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.user_plist_blobs = user_blobs;
        cache.system_plist_blobs = system_blobs;
        cache.refreshed_at = Some(Instant::now());
        return cache.clone();
    }

    let cache = mac_startup_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.clone()
}

#[cfg(target_os = "macos")]
fn mac_collect_plist_blobs(root: &Path) -> Vec<String> {
    if !root.exists() {
        return Vec::new();
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut blobs = Vec::new();
    for entry in entries.flatten().take(500) {
        let path = entry.path();
        if !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("plist"))
        {
            continue;
        }
        if let Some(content) = mac_read_plist_text(path.as_path()) {
            blobs.push(content.to_ascii_lowercase());
        }
    }
    blobs
}

#[cfg(target_os = "macos")]
fn mac_read_plist_text(path: &Path) -> Option<String> {
    if let Ok(content) = fs::read_to_string(path) {
        return Some(content);
    }
    let output = Command::new("plutil")
        .args([
            "-convert",
            "xml1",
            "-o",
            "-",
            path.to_string_lossy().as_ref(),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "macos")]
fn mac_set_startup(app_id: &str, app_path: &Path, enabled: bool) -> AppResult<()> {
    let startup_path = mac_startup_file_path(app_id).ok_or_else(|| {
        AppError::new(
            "app_manager_startup_path_missing",
            "无法定位启动项目录，请检查 HOME 环境",
        )
    })?;

    if enabled {
        let parent = startup_path.parent().ok_or_else(|| {
            AppError::new("app_manager_startup_path_invalid", "启动项路径无效")
                .with_detail(startup_path.to_string_lossy().to_string())
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            AppError::new(
                "app_manager_startup_dir_create_failed",
                "创建启动项目录失败",
            )
            .with_detail(error.to_string())
        })?;

        let label = startup_label(app_id);
        let app_str = app_path.to_string_lossy().to_string();
        let program_arguments = if app_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("app"))
        {
            format!(
                "<array><string>/usr/bin/open</string><string>-a</string><string>{}</string></array>",
                xml_escape(app_str.as_str())
            )
        } else {
            format!(
                "<array><string>{}</string></array>",
                xml_escape(app_str.as_str())
            )
        };

        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{}</string>
  <key>ProgramArguments</key>
  {}
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#,
            xml_escape(label.as_str()),
            program_arguments
        );

        fs::write(startup_path, plist).map_err(|error| {
            AppError::new("app_manager_startup_write_failed", "写入启动项失败")
                .with_detail(error.to_string())
        })?;
        return Ok(());
    }

    match fs::remove_file(&startup_path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(
            AppError::new("app_manager_startup_delete_failed", "删除启动项失败")
                .with_detail(error.to_string()),
        ),
    }
}

#[cfg(target_os = "windows")]
fn windows_startup_value_name(app_id: &str) -> String {
    format!(
        "RToolStartup_{}",
        stable_hash(app_id).chars().take(10).collect::<String>()
    )
}

#[cfg(target_os = "windows")]
fn windows_startup_enabled(app_id: &str) -> bool {
    let value_name = windows_startup_value_name(app_id);
    Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            value_name.as_str(),
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn windows_run_registry_contains(root: &str, app_path: &Path) -> bool {
    let target = normalize_path_key(app_path.to_string_lossy().as_ref());
    if target.is_empty() {
        return false;
    }

    let output = match Command::new("reg").args(["query", root]).output() {
        Ok(output) => output,
        Err(_) => return false,
    };
    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(|line| normalize_path_key(line))
        .any(|line| line.contains(target.as_str()))
}

#[cfg(target_os = "windows")]
fn windows_set_startup(app_id: &str, app_path: &Path, enabled: bool) -> AppResult<()> {
    let value_name = windows_startup_value_name(app_id);
    if enabled {
        let command_value = format!("\"{}\"", app_path.to_string_lossy());
        let status = Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                value_name.as_str(),
                "/t",
                "REG_SZ",
                "/d",
                command_value.as_str(),
                "/f",
            ])
            .status()
            .map_err(|error| {
                AppError::new("app_manager_startup_update_failed", "更新启动项失败")
                    .with_detail(error.to_string())
            })?;
        if !status.success() {
            return Err(
                AppError::new("app_manager_startup_update_failed", "更新启动项失败")
                    .with_detail(format!("status={status}")),
            );
        }
        return Ok(());
    }

    let status = Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            value_name.as_str(),
            "/f",
        ])
        .status()
        .map_err(|error| {
            AppError::new("app_manager_startup_update_failed", "更新启动项失败")
                .with_detail(error.to_string())
        })?;
    if !status.success() {
        return Err(
            AppError::new("app_manager_startup_update_failed", "更新启动项失败")
                .with_detail(format!("status={status}")),
        );
    }
    Ok(())
}

fn platform_uninstall(item: &ManagedAppDto) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        mac_uninstall(item)
    }
    #[cfg(target_os = "windows")]
    {
        windows_uninstall(item)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = item;
        Err(AppError::new(
            "app_manager_uninstall_not_supported",
            "当前平台暂不支持卸载功能",
        ))
    }
}

fn platform_open_uninstall_help(item: &ManagedAppDto) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        if item.path.trim().is_empty() {
            return Err(AppError::new(
                "app_manager_open_help_invalid",
                "无有效应用路径",
            ));
        }
        open_with_command(
            "open",
            &["-R", item.path.as_str()],
            "app_manager_open_help_failed",
        )
    }
    #[cfg(target_os = "windows")]
    {
        let _ = item;
        open_with_command(
            "cmd",
            &["/C", "start", "", "ms-settings:appsfeatures"],
            "app_manager_open_help_failed",
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = item;
        Err(AppError::new(
            "app_manager_open_help_not_supported",
            "当前平台暂不支持该操作",
        ))
    }
}

#[cfg(target_os = "macos")]
fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\"', "\\\"")
}

#[cfg(target_os = "macos")]
fn mac_uninstall(item: &ManagedAppDto) -> AppResult<()> {
    if item.path.trim().is_empty() {
        return Err(AppError::new(
            "app_manager_uninstall_invalid_path",
            "应用路径为空",
        ));
    }
    if !Path::new(item.path.as_str()).exists() {
        return Err(AppError::new(
            "app_manager_uninstall_not_found",
            "应用路径不存在，无法卸载",
        ));
    }

    let script = format!(
        "tell application \"Finder\" to delete POSIX file \"{}\"",
        applescript_escape(item.path.as_str())
    );
    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|error| {
            AppError::new("app_manager_uninstall_failed", "调用系统卸载失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }

    Err(
        AppError::new("app_manager_uninstall_failed", "系统卸载执行失败")
            .with_detail(format!("status={status}")),
    )
}

#[cfg(target_os = "windows")]
fn windows_uninstall(item: &ManagedAppDto) -> AppResult<()> {
    let entries = windows_list_uninstall_entries();
    let matched = windows_find_best_uninstall_entry(
        item.name.as_str(),
        Path::new(item.path.as_str()),
        entries.as_slice(),
    );

    if let Some(entry) = matched {
        let command = entry
            .quiet_uninstall_string
            .as_deref()
            .or(entry.uninstall_string.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if let Some(command) = command {
            if windows_execute_uninstall_command(command) {
                return Ok(());
            }

            tracing::warn!(
                event = "app_manager_windows_uninstall_command_failed",
                app_name = item.name.as_str(),
                command = command
            );
        }
    }

    open_with_command(
        "cmd",
        &["/C", "start", "", "ms-settings:appsfeatures"],
        "app_manager_uninstall_failed",
    )
}

#[cfg(target_os = "windows")]
fn windows_execute_uninstall_command(command: &str) -> bool {
    let direct_status = Command::new("cmd").args(["/C", command]).status();
    if direct_status.as_ref().is_ok_and(|status| status.success()) {
        return true;
    }

    let escaped = windows_powershell_escape(command);
    let script = format!(
        "$cmd='{}'; Start-Process -FilePath 'cmd.exe' -ArgumentList '/C', $cmd -Verb RunAs",
        escaped
    );
    let elevated_status = Command::new("powershell")
        .args(["-NoProfile", "-Command", script.as_str()])
        .status();
    elevated_status
        .as_ref()
        .is_ok_and(|status| status.success())
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
                vec!["AppCleaner".to_string(), "net.freemacsoft.AppCleaner".to_string()],
                "bundle_id",
            ),
            risk_level: "low".to_string(),
            fingerprint: "fp".to_string(),
        }
    }

    fn has_root_path(roots: &[RelatedRootSpec], expected: &Path) -> bool {
        let expected_key = normalize_path_key(expected.to_string_lossy().as_ref());
        roots.iter().any(|root| {
            normalize_path_key(root.path.to_string_lossy().as_ref()) == expected_key
        })
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

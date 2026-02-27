use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopePlatform {
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
    pub(crate) fn normalize(mut self) -> Self {
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

    pub(super) fn to_dto(&self) -> LauncherSearchSettingsDto {
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

pub(super) fn is_default_scope_profile(settings: &LauncherSearchSettingsRecord) -> bool {
    settings.roots == default_search_roots()
        && settings.max_scan_depth == DEFAULT_MAX_SCAN_DEPTH
        && settings.max_items_per_root == DEFAULT_MAX_ITEMS_PER_ROOT
        && settings.max_total_items == DEFAULT_MAX_TOTAL_ITEMS
        && settings.refresh_interval_secs == DEFAULT_REFRESH_INTERVAL_SECS
        && settings.exclude_patterns == default_exclude_patterns()
}

pub(super) fn has_single_system_root_scope(settings: &LauncherSearchSettingsRecord) -> bool {
    if settings.roots.len() != 1 {
        return false;
    }
    let normalized = normalize_path_for_match(Path::new(settings.roots[0].as_str()));
    normalized == "/" || is_windows_drive_root(normalized.as_str())
}

pub(super) fn resolve_effective_max_items_per_root(
    configured_max_items_per_root: usize,
    remaining_total: usize,
    single_system_root_scope: bool,
) -> usize {
    if single_system_root_scope {
        return remaining_total.max(1);
    }
    configured_max_items_per_root.max(1)
}

pub async fn get_search_settings_async(db_conn: &DbConn) -> AppResult<LauncherSearchSettingsDto> {
    let settings = load_or_init_settings(db_conn).await?;
    Ok(settings.to_dto())
}

pub async fn update_search_settings_async(
    db_conn: &DbConn,
    input: LauncherUpdateSearchSettingsInputDto,
) -> AppResult<LauncherSearchSettingsDto> {
    let current = load_or_init_settings(db_conn).await?;
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

    save_settings(db_conn, &next).await?;
    Ok(next.to_dto())
}

pub async fn reset_search_settings_async(db_conn: &DbConn) -> AppResult<LauncherSearchSettingsDto> {
    let next = LauncherSearchSettingsRecord::default().normalize();
    save_settings(db_conn, &next).await?;
    set_app_setting(
        db_conn,
        LAUNCHER_SCOPE_POLICY_VERSION_KEY,
        LAUNCHER_SCOPE_POLICY_VERSION_VALUE,
    )
    .await?;
    Ok(next.to_dto())
}

pub(super) async fn load_or_init_settings(
    db_conn: &DbConn,
) -> AppResult<LauncherSearchSettingsRecord> {
    let fallback = LauncherSearchSettingsRecord::default().normalize();
    let settings = match get_app_setting(db_conn, SEARCH_SETTINGS_KEY).await? {
        Some(raw_value) => {
            match serde_json::from_str::<LauncherSearchSettingsRecord>(raw_value.as_str()) {
                Ok(parsed) => {
                    let normalized = parsed.clone().normalize();
                    if normalized != parsed {
                        save_settings(db_conn, &normalized).await?;
                    }
                    normalized
                }
                Err(error) => {
                    tracing::warn!(
                        event = "launcher_settings_parse_failed",
                        error = error.to_string()
                    );
                    save_settings(db_conn, &fallback).await?;
                    fallback
                }
            }
        }
        None => {
            save_settings(db_conn, &fallback).await?;
            fallback
        }
    };

    let from_state = get_app_setting(db_conn, LAUNCHER_SCOPE_POLICY_VERSION_KEY)
        .await?
        .filter(|value| !value.trim().is_empty());
    if from_state.as_deref() != Some(LAUNCHER_SCOPE_POLICY_VERSION_VALUE) {
        set_app_setting(
            db_conn,
            LAUNCHER_SCOPE_POLICY_VERSION_KEY,
            LAUNCHER_SCOPE_POLICY_VERSION_VALUE,
        )
        .await?;
    }

    Ok(settings)
}

pub(crate) async fn save_settings(
    db_conn: &DbConn,
    settings: &LauncherSearchSettingsRecord,
) -> DbResult<()> {
    let serialized = serde_json::to_string(settings).map_err(|error| {
        AppError::new("launcher_settings_serialize_failed", "启动器设置序列化失败")
            .with_source(error)
    })?;
    set_app_setting(db_conn, SEARCH_SETTINGS_KEY, serialized.as_str()).await
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

pub(super) fn normalize_path_pattern(value: &str) -> String {
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
    let local_app_data_dir = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let candidates = build_default_search_root_candidates(
        current_scope_platform(),
        home_dir.as_deref(),
        app_data_dir.as_deref(),
        program_data_dir.as_deref(),
        local_app_data_dir.as_deref(),
    );
    let mut roots = collect_existing_roots(candidates);

    if roots.is_empty() {
        let mut fallback_candidates = Vec::new();
        if let Some(home) = home_dir {
            fallback_candidates.push(home);
        }
        if let Ok(current_dir) = std::env::current_dir() {
            fallback_candidates.push(current_dir);
        }
        fallback_candidates.push(std::env::temp_dir());
        roots = collect_existing_roots(fallback_candidates);
    }

    roots
}

pub(crate) fn build_default_search_root_candidates(
    platform: ScopePlatform,
    home_dir: Option<&Path>,
    app_data_dir: Option<&Path>,
    program_data_dir: Option<&Path>,
    local_app_data_dir: Option<&Path>,
) -> Vec<PathBuf> {
    match platform {
        ScopePlatform::Macos => {
            let mut roots = Vec::new();
            if let Some(home) = home_dir {
                roots.push(home.join("Applications"));
            }
            roots.push(PathBuf::from("/Applications"));
            if let Some(home) = home_dir {
                roots.push(home.join("Desktop"));
                roots.push(home.join("Documents"));
                roots.push(home.join("Downloads"));
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
            if let Some(home) = home_dir {
                roots.push(home.join("Desktop"));
                roots.push(home.join("Documents"));
                roots.push(home.join("Downloads"));
            }
            if let Some(local_app_data) = local_app_data_dir {
                roots.push(local_app_data.join("Programs"));
            }
            roots
        }
        ScopePlatform::Linux => {
            let mut roots = Vec::new();
            if let Some(home) = home_dir {
                roots.push(home.join(".local/share/applications"));
            }
            roots.push(PathBuf::from("/usr/share/applications"));
            roots.push(PathBuf::from("/usr/local/share/applications"));
            if let Some(home) = home_dir {
                roots.push(home.join("Desktop"));
                roots.push(home.join("Documents"));
                roots.push(home.join("Downloads"));
            }
            roots
        }
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
        if normalized.is_empty()
            || is_system_root_normalized(normalized.as_str())
            || !dedup.insert(normalized)
        {
            continue;
        }
        roots.push(candidate.to_string_lossy().to_string());
    }
    roots
}

pub(super) fn default_exclude_patterns() -> Vec<String> {
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

pub(super) fn is_truthy_flag(value: &str) -> bool {
    matches!(value, "1" | "true" | "TRUE")
}

pub(crate) fn normalize_path_for_match(path: &Path) -> String {
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

fn is_system_root_normalized(normalized: &str) -> bool {
    normalized == "/" || is_windows_drive_root(normalized)
}

pub(super) fn normalize_query(value: &str) -> String {
    value.trim().to_lowercase()
}

pub(crate) fn escape_like_pattern(value: &str) -> String {
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

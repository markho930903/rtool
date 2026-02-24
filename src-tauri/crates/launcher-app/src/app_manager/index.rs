use super::*;

pub(super) fn build_app_index(app: &dyn LauncherHost) -> AppResult<Vec<ManagedAppDto>> {
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

pub(super) fn build_self_item(app: &dyn LauncherHost) -> Option<ManagedAppDto> {
    let executable = std::env::current_exe().ok()?;
    let package_info = app.package_info();
    let app_name = package_info.name.clone();
    let app_path = executable.to_string_lossy().to_string();
    let size_path = resolve_app_size_path(executable.as_path());
    let size_snapshot = resolve_app_size_snapshot(size_path.as_path());
    let id = stable_app_id("rtool", app_path.as_str());
    let (startup_enabled, startup_scope, startup_editable) =
        platform_detect_startup_state(id.as_str(), executable.as_path());
    let readonly_reason_code = startup_readonly_reason_code(startup_scope, startup_editable);
    let icon = resolve_builtin_icon("i-noto:rocket");
    let bundle_or_app_id = Some(package_info.name);
    let aliases = collect_app_path_aliases_from_parts(
        app_name.as_str(),
        app_path.as_str(),
        bundle_or_app_id.as_deref(),
    );
    let identity = build_app_identity(
        normalize_path_key(app_path.as_str()),
        aliases,
        AppManagerIdentitySource::Path,
    );

    let mut item = ManagedAppDto {
        id,
        name: app_name.clone(),
        path: app_path,
        bundle_or_app_id,
        version: Some(package_info.version),
        publisher: None,
        platform: AppManagerPlatform::current(),
        source: AppManagerSource::Rtool,
        icon_kind: AppManagerIconKind::from_raw(icon.kind.as_str()),
        icon_value: icon.value,
        size_bytes: size_snapshot.size_bytes,
        size_accuracy: size_snapshot.size_accuracy,
        size_computed_at: size_snapshot.size_computed_at,
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
        risk_level: AppManagerRiskLevel::High,
        fingerprint: String::new(),
    };
    item.fingerprint = fingerprint_for_app(&item);
    Some(item)
}

pub(super) fn collect_platform_apps(app: &dyn LauncherHost) -> Vec<ManagedAppDto> {
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

pub(super) fn collect_index_source_fingerprint() -> String {
    #[cfg(target_os = "macos")]
    {
        collect_macos_source_fingerprint()
    }
    #[cfg(target_os = "windows")]
    {
        collect_windows_source_fingerprint()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "unsupported-platform".to_string()
    }
}

#[cfg(target_os = "macos")]
fn collect_macos_source_fingerprint() -> String {
    let mut entries = Vec::new();
    for root in mac_application_roots() {
        let root_key = normalize_path_key(root.to_string_lossy().as_ref());
        if root_key.is_empty() {
            continue;
        }
        entries.push(format!("root:{root_key}"));
        let Ok(read_dir) = fs::read_dir(root) else {
            continue;
        };
        for entry in read_dir.flatten().take(2_000) {
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            if !ext.eq_ignore_ascii_case("app") {
                continue;
            }
            let path_key = normalize_path_key(path.to_string_lossy().as_ref());
            if path_key.is_empty() {
                continue;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|value| value.modified().ok())
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|value| value.as_secs())
                .unwrap_or(0);
            entries.push(format!("app:{path_key}:{modified}"));
        }
    }
    entries.sort();
    stable_hash(entries.join("|").as_str())
}

#[cfg(target_os = "windows")]
fn collect_windows_source_fingerprint() -> String {
    let mut entries = Vec::new();
    for item in windows_list_uninstall_entries() {
        let display_name = item.display_name.trim().to_ascii_lowercase();
        let registry_key = windows_normalize_registry_key(item.registry_key.as_str());
        let location = item
            .install_location
            .as_deref()
            .map(normalize_path_key)
            .unwrap_or_default();
        let version = item
            .display_version
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        entries.push(format!(
            "uninstall:{registry_key}:{display_name}:{location}:{version}"
        ));
    }
    for root in [
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run",
    ] {
        let mut values = windows_query_registry_values(root);
        values.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        for (name, value) in values {
            entries.push(format!(
                "run:{}:{}:{}",
                windows_normalize_registry_key(root),
                name.to_ascii_lowercase(),
                normalize_path_key(value.as_str())
            ));
        }
    }
    entries.sort();
    stable_hash(entries.join("|").as_str())
}

#[cfg(target_os = "macos")]
pub(super) fn collect_macos_apps(app: &dyn LauncherHost) -> Vec<ManagedAppDto> {
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
pub(super) fn mac_application_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = home_dir() {
        roots.push(home.join("Applications"));
    }
    roots
}

#[cfg(target_os = "macos")]
pub(super) fn build_macos_app_item(
    app: &dyn LauncherHost,
    app_path: &Path,
) -> Option<ManagedAppDto> {
    let path_str = app_path.to_string_lossy().to_string();
    let size_path = resolve_app_size_path(app_path);
    let size_snapshot = resolve_app_size_snapshot(size_path.as_path());
    let info = parse_macos_info_plist(app_path.join("Contents").join("Info.plist").as_path());
    let bundle = info.bundle_id.clone();
    let version = info.version.clone();
    let publisher = info.publisher.clone();
    let mut name_candidates = Vec::new();
    push_display_name_candidate(&mut name_candidates, info.bundle_display_name.clone(), 90);
    push_display_name_candidate(&mut name_candidates, info.bundle_name.clone(), 70);
    push_display_name_candidate(&mut name_candidates, path_stem_string(app_path), 85);
    let name = resolve_application_display_name(app_path, path_str.as_str(), name_candidates);
    let id = stable_app_id("application", path_str.as_str());
    let icon = resolve_application_icon(app, app_path);
    let (startup_enabled, startup_scope, startup_editable) =
        platform_detect_startup_state(id.as_str(), app_path);
    let readonly_reason_code = startup_readonly_reason_code(startup_scope, startup_editable);
    let aliases =
        collect_app_path_aliases_from_parts(name.as_str(), path_str.as_str(), bundle.as_deref());
    let identity = if let Some(bundle_id) = bundle.as_deref() {
        build_app_identity(bundle_id, aliases, AppManagerIdentitySource::BundleId)
    } else {
        build_app_identity(
            normalize_path_key(path_str.as_str()),
            aliases,
            AppManagerIdentitySource::Path,
        )
    };
    let mut item = ManagedAppDto {
        id,
        name,
        path: path_str,
        bundle_or_app_id: bundle,
        version,
        publisher,
        platform: AppManagerPlatform::Macos,
        source: AppManagerSource::Application,
        icon_kind: AppManagerIconKind::from_raw(icon.kind.as_str()),
        icon_value: icon.value,
        size_bytes: size_snapshot.size_bytes,
        size_accuracy: size_snapshot.size_accuracy,
        size_computed_at: size_snapshot.size_computed_at,
        startup_enabled,
        startup_scope,
        startup_editable,
        readonly_reason_code,
        uninstall_supported: true,
        uninstall_kind: Some(AppManagerUninstallKind::FinderTrash),
        capabilities: build_app_capabilities(true, true, true),
        identity,
        risk_level: AppManagerRiskLevel::Medium,
        fingerprint: String::new(),
    };
    item.fingerprint = fingerprint_for_app(&item);
    Some(item)
}

#[cfg(target_os = "macos")]
pub(super) struct MacAppInfo {
    bundle_display_name: Option<String>,
    bundle_name: Option<String>,
    bundle_id: Option<String>,
    version: Option<String>,
    publisher: Option<String>,
}

#[cfg(target_os = "macos")]
pub(super) fn parse_macos_info_plist(path: &Path) -> MacAppInfo {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => {
            return MacAppInfo {
                bundle_display_name: None,
                bundle_name: None,
                bundle_id: None,
                version: None,
                publisher: None,
            };
        }
    };

    let bundle_display_name = plist_value(content.as_str(), "CFBundleDisplayName");
    let bundle_name = plist_value(content.as_str(), "CFBundleName");
    let bundle_id = plist_value(content.as_str(), "CFBundleIdentifier");
    let version = plist_value(content.as_str(), "CFBundleShortVersionString")
        .or_else(|| plist_value(content.as_str(), "CFBundleVersion"));
    let publisher = bundle_id
        .as_deref()
        .and_then(|value| value.split('.').next())
        .map(ToString::to_string)
        .filter(|value| !value.is_empty());

    MacAppInfo {
        bundle_display_name,
        bundle_name,
        bundle_id,
        version,
        publisher,
    }
}

#[cfg(target_os = "macos")]
pub(super) fn plist_value(content: &str, key: &str) -> Option<String> {
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
pub(super) fn collect_windows_apps(app: &dyn LauncherHost) -> Vec<ManagedAppDto> {
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
pub(super) fn windows_normalize_registry_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
pub(super) fn windows_is_generic_uninstall_binary(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    matches!(
        file_name.as_str(),
        "msiexec.exe" | "rundll32.exe" | "cmd.exe"
    )
}

#[cfg(target_os = "windows")]
pub(super) fn windows_extract_executable_from_command(command: &str) -> Option<PathBuf> {
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
pub(super) fn windows_discovery_path_from_uninstall_entry(
    entry: &WindowsUninstallEntry,
) -> Option<PathBuf> {
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
fn windows_size_measurement_path(entry: &WindowsUninstallEntry, fallback_path: &Path) -> PathBuf {
    if let Some(location) = entry.install_location.as_deref() {
        let location = location.trim().trim_matches('"');
        if !location.is_empty() {
            return PathBuf::from(location);
        }
    }
    resolve_app_size_path(fallback_path)
}

#[cfg(target_os = "windows")]
pub(super) fn windows_uninstall_entry_matches_path(
    entry: &WindowsUninstallEntry,
    app_path: &Path,
) -> bool {
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
pub(super) fn windows_build_item_from_uninstall_entry(
    app: &dyn LauncherHost,
    entry: &WindowsUninstallEntry,
    path: &Path,
) -> ManagedAppDto {
    let path_str = path.to_string_lossy().to_string();
    let size_path = windows_size_measurement_path(entry, path);
    let size_snapshot = resolve_app_size_snapshot(size_path.as_path());
    let mut size_bytes = size_snapshot.size_bytes;
    let mut size_accuracy = size_snapshot.size_accuracy;
    let mut size_computed_at = size_snapshot.size_computed_at;
    if size_bytes.is_none() {
        if let Some(estimated_size_kb) = entry.estimated_size_kb {
            size_bytes = Some(estimated_size_kb.saturating_mul(1024));
            size_accuracy = AppManagerSizeAccuracy::Estimated;
            if size_computed_at.is_none() {
                size_computed_at = Some(now_unix_seconds());
            }
        }
    }
    let parent_stem = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|value| value.to_str())
        .map(ToString::to_string);
    let mut name_candidates = Vec::new();
    push_display_name_candidate(&mut name_candidates, Some(entry.display_name.clone()), 90);
    push_display_name_candidate(
        &mut name_candidates,
        path.file_stem()
            .and_then(|value| value.to_str())
            .map(ToString::to_string),
        80,
    );
    push_display_name_candidate(&mut name_candidates, parent_stem, 45);
    let name = resolve_application_display_name(path, path_str.as_str(), name_candidates);

    let id = stable_app_id("application", path_str.as_str());
    let icon = resolve_application_icon(app, path);
    let (startup_enabled, startup_scope, startup_editable) =
        platform_detect_startup_state(id.as_str(), path);
    let readonly_reason_code = startup_readonly_reason_code(startup_scope, startup_editable);
    let aliases = collect_app_path_aliases_from_parts(name.as_str(), path_str.as_str(), None);

    let mut item = ManagedAppDto {
        id,
        name,
        path: path_str,
        bundle_or_app_id: None,
        version: entry.display_version.clone(),
        publisher: entry.publisher.clone(),
        platform: AppManagerPlatform::Windows,
        source: AppManagerSource::Application,
        icon_kind: AppManagerIconKind::from_raw(icon.kind.as_str()),
        icon_value: icon.value,
        size_bytes,
        size_accuracy,
        size_computed_at,
        startup_enabled,
        startup_scope,
        startup_editable,
        readonly_reason_code,
        uninstall_supported: true,
        uninstall_kind: Some(AppManagerUninstallKind::RegistryCommand),
        capabilities: build_app_capabilities(true, true, true),
        identity: build_app_identity(
            entry.registry_key.as_str(),
            aliases,
            AppManagerIdentitySource::Registry,
        ),
        risk_level: AppManagerRiskLevel::Medium,
        fingerprint: String::new(),
    };
    item.fingerprint = fingerprint_for_app(&item);
    item
}

#[cfg(target_os = "windows")]
pub(super) fn windows_collect_apps_from_uninstall_entries(
    app: &dyn LauncherHost,
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
        items.push(windows_build_item_from_uninstall_entry(
            app,
            entry,
            path.as_path(),
        ));
    }
    items
}

#[cfg(target_os = "windows")]
pub(super) fn scan_windows_root(
    root: &Path,
    max_depth: usize,
    max_items: usize,
    items: &mut Vec<ManagedAppDto>,
    seen_path_keys: &mut HashSet<String>,
    seen_identity_keys: &mut HashSet<String>,
    app: &dyn LauncherHost,
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

            let identity_key =
                windows_normalize_registry_key(uninstall_match.registry_key.as_str());
            if seen_identity_keys.contains(identity_key.as_str()) {
                continue;
            }
            let path_key = normalize_path_key(path_str.as_str());
            if path_key.is_empty() || seen_path_keys.contains(path_key.as_str()) {
                continue;
            }
            let item =
                windows_build_item_from_uninstall_entry(app, &uninstall_match, path.as_path());
            seen_identity_keys.insert(identity_key);
            seen_path_keys.insert(path_key);
            items.push(item);
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub(super) struct WindowsUninstallEntry {
    display_name: String,
    uninstall_string: Option<String>,
    quiet_uninstall_string: Option<String>,
    install_location: Option<String>,
    publisher: Option<String>,
    display_version: Option<String>,
    estimated_size_kb: Option<u64>,
    registry_key: String,
}

#[cfg(target_os = "windows")]
pub(super) fn windows_uninstall_roots() -> [&'static str; 3] {
    [
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ]
}

#[cfg(target_os = "windows")]
pub(super) fn windows_list_uninstall_entries() -> Vec<WindowsUninstallEntry> {
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
pub(super) fn windows_query_uninstall_root(root: &str) -> Vec<WindowsUninstallEntry> {
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
        let estimated_size_kb = values
            .get("EstimatedSize")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .and_then(|value| value.parse::<u64>().ok());

        entries.push(WindowsUninstallEntry {
            display_name: display_name.to_string(),
            uninstall_string,
            quiet_uninstall_string,
            install_location,
            publisher,
            display_version,
            estimated_size_kb,
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
pub(super) fn windows_parse_reg_value_line(line: &str) -> Option<(String, String)> {
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
pub(super) fn windows_query_registry_values(root: &str) -> Vec<(String, String)> {
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
pub(super) fn windows_registry_value_exists(root: &str, value_name: &str) -> bool {
    Command::new("reg")
        .args(["query", root, "/v", value_name])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub(super) fn windows_find_best_uninstall_entry(
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

        if score > best_score
            || (score == best_score && has_path_evidence && !best_has_path_evidence)
        {
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
pub(super) fn windows_application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(app_data).join("Microsoft/Windows/Start Menu/Programs"));
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(program_data).join("Microsoft/Windows/Start Menu/Programs"));
    }
    roots
}

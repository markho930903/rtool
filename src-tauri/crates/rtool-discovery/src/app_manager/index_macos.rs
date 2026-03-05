use super::*;

#[cfg(target_os = "macos")]
pub(crate) fn collect_macos_source_fingerprint() -> String {
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

#[cfg(target_os = "macos")]
pub(crate) fn collect_macos_apps(app: &dyn LauncherHost) -> Vec<ManagedAppDto> {
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
pub(crate) fn mac_application_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = home_dir() {
        roots.push(home.join("Applications"));
    }
    roots
}

#[cfg(target_os = "macos")]
pub(crate) fn build_macos_app_item(
    app: &dyn LauncherHost,
    app_path: &Path,
) -> Option<ManagedAppDto> {
    let path_str = app_path.to_string_lossy().to_string();
    let size_resolution = resolve_app_size_path(app_path);
    let size_snapshot = resolve_app_size_snapshot(size_resolution.path.as_path());
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
        size_source: size_resolution.size_source,
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
pub(crate) struct MacAppInfo {
    bundle_display_name: Option<String>,
    bundle_name: Option<String>,
    bundle_id: Option<String>,
    version: Option<String>,
    publisher: Option<String>,
}

#[cfg(target_os = "macos")]
pub(crate) fn parse_macos_info_plist(path: &Path) -> MacAppInfo {
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
pub(crate) fn plist_value(content: &str, key: &str) -> Option<String> {
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

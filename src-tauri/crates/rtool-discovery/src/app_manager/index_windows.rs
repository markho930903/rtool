use super::*;

#[cfg(target_os = "windows")]
#[path = "index_windows_helpers.rs"]
mod index_windows_helpers;
#[cfg(target_os = "windows")]
pub(crate) use index_windows_helpers::*;

#[cfg(target_os = "windows")]
#[path = "index_windows_registry.rs"]
mod index_windows_registry;
#[cfg(target_os = "windows")]
pub(crate) use index_windows_registry::*;

#[cfg(target_os = "windows")]
pub(crate) fn collect_windows_source_fingerprint() -> String {
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
        r"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        r"HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
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

#[cfg(target_os = "windows")]
pub(crate) fn collect_windows_apps(app: &dyn LauncherHost) -> Vec<ManagedAppDto> {
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
pub(crate) fn windows_collect_apps_from_uninstall_entries(
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
pub(crate) fn scan_windows_root(
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

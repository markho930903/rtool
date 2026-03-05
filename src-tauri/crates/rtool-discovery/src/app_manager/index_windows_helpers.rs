use super::*;

#[cfg(target_os = "windows")]
pub(crate) fn windows_normalize_registry_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_is_generic_uninstall_binary(path: &Path) -> bool {
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
pub(crate) fn windows_extract_executable_from_command(command: &str) -> Option<PathBuf> {
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
pub(crate) fn windows_discovery_path_from_uninstall_entry(
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
fn windows_size_measurement_path(
    entry: &WindowsUninstallEntry,
    fallback_path: &Path,
) -> AppSizePathResolution {
    if let Some(location) = entry.install_location.as_deref() {
        let location = location.trim().trim_matches('"');
        if !location.is_empty() {
            return AppSizePathResolution {
                path: PathBuf::from(location),
                size_source: AppManagerSizeSource::Path,
            };
        }
    }
    resolve_app_size_path(fallback_path)
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_uninstall_entry_matches_path(
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
pub(crate) fn windows_build_item_from_uninstall_entry(
    app: &dyn LauncherHost,
    entry: &WindowsUninstallEntry,
    path: &Path,
) -> ManagedAppDto {
    let path_str = path.to_string_lossy().to_string();
    let size_resolution = windows_size_measurement_path(entry, path);
    let size_snapshot = resolve_app_size_snapshot(size_resolution.path.as_path());
    let mut size_bytes = size_snapshot.size_bytes;
    let mut size_accuracy = size_snapshot.size_accuracy;
    let mut size_source = size_resolution.size_source;
    let mut size_computed_at = size_snapshot.size_computed_at;
    if size_bytes.is_none() {
        if let Some(estimated_size_kb) = entry.estimated_size_kb {
            size_bytes = Some(estimated_size_kb.saturating_mul(1024));
            size_accuracy = AppManagerSizeAccuracy::Estimated;
            size_source = AppManagerSizeSource::RegistryEstimated;
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
        size_source,
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

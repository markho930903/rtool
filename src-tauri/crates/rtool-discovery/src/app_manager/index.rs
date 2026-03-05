use super::*;

#[cfg(target_os = "macos")]
#[path = "index_macos.rs"]
mod index_macos;
#[cfg(target_os = "macos")]
pub(super) use index_macos::*;

#[cfg(target_os = "windows")]
#[path = "index_windows.rs"]
mod index_windows;
#[cfg(target_os = "windows")]
pub(super) use index_windows::*;

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

    sort_managed_apps_for_list(items.as_mut_slice());
    Ok(items)
}

pub(super) fn build_self_item(app: &dyn LauncherHost) -> Option<ManagedAppDto> {
    let executable = std::env::current_exe().ok()?;
    let package_info = app.package_info();
    let app_name = package_info.name.clone();
    let app_path = executable.to_string_lossy().to_string();
    let size_resolution = resolve_self_app_size_path(executable.as_path());
    let size_snapshot = resolve_app_size_snapshot(size_resolution.path.as_path());
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
        size_source: size_resolution.size_source,
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

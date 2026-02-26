use super::*;

#[derive(Debug, Clone)]
pub(super) struct RelatedRootSpec {
    pub(super) label: String,
    pub(super) path: PathBuf,
    pub(super) scope: AppManagerScope,
    pub(super) kind: AppManagerResidueKind,
}

#[derive(Debug, Clone)]
pub(super) struct ResidueCandidate {
    pub(super) path: PathBuf,
    pub(super) scope: AppManagerScope,
    pub(super) kind: AppManagerResidueKind,
    pub(super) exists: bool,
    pub(super) filesystem: bool,
    pub(super) match_reason: AppManagerResidueMatchReason,
    pub(super) confidence: AppManagerResidueConfidence,
    pub(super) evidence: Vec<String>,
    pub(super) risk_level: AppManagerRiskLevel,
    pub(super) recommended: bool,
    pub(super) readonly_reason_code: Option<AppReadonlyReasonCode>,
}

fn push_related_root(
    roots: &mut Vec<RelatedRootSpec>,
    label: impl Into<String>,
    path: PathBuf,
    scope: AppManagerScope,
    kind: AppManagerResidueKind,
) {
    roots.push(RelatedRootSpec {
        label: label.into(),
        path,
        scope,
        kind,
    });
}

#[cfg(target_os = "macos")]
fn mac_is_var_folders_temp_root(path: &Path) -> bool {
    let key = normalize_path_key(path.to_string_lossy().as_ref());
    key.contains("/var/folders/")
}

#[cfg(target_os = "macos")]
pub(super) fn mac_collect_temp_alias_roots(alias: &str) -> Vec<PathBuf> {
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

pub(super) fn collect_related_root_specs(item: &ManagedAppDto) -> Vec<RelatedRootSpec> {
    let mut roots = Vec::new();
    let install_root = app_install_root(item);
    let install_scope = home_dir()
        .as_ref()
        .filter(|home| install_root.starts_with(home))
        .map(|_| AppManagerScope::User)
        .unwrap_or(AppManagerScope::System);
    push_related_root(
        &mut roots,
        "安装目录",
        install_root,
        install_scope,
        AppManagerResidueKind::Install,
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
                    AppManagerScope::User,
                    AppManagerResidueKind::AppSupport,
                );
                push_related_root(
                    &mut roots,
                    "用户缓存目录",
                    home.join("Library/Caches").join(alias),
                    AppManagerScope::User,
                    AppManagerResidueKind::Cache,
                );
                push_related_root(
                    &mut roots,
                    "用户 HTTP 存储目录",
                    home.join("Library/HTTPStorages").join(alias),
                    AppManagerScope::User,
                    AppManagerResidueKind::Cache,
                );
                for temp_root in mac_collect_temp_alias_roots(alias.as_str()) {
                    push_related_root(
                        &mut roots,
                        "用户临时缓存目录",
                        temp_root,
                        AppManagerScope::User,
                        AppManagerResidueKind::Cache,
                    );
                }
                push_related_root(
                    &mut roots,
                    "用户偏好设置",
                    home.join("Library/Preferences")
                        .join(format!("{alias}.plist")),
                    AppManagerScope::User,
                    AppManagerResidueKind::Preferences,
                );
                push_related_root(
                    &mut roots,
                    "用户日志目录",
                    home.join("Library/Logs").join(alias),
                    AppManagerScope::User,
                    AppManagerResidueKind::Logs,
                );
            }
            if let Some(startup_path) = mac_startup_file_path(item.id.as_str()) {
                push_related_root(
                    &mut roots,
                    "用户启动项",
                    startup_path,
                    AppManagerScope::User,
                    AppManagerResidueKind::Startup,
                );
            }
        }
        for alias in &aliases {
            push_related_root(
                &mut roots,
                "系统应用支持目录",
                PathBuf::from("/Library/Application Support").join(alias),
                AppManagerScope::System,
                AppManagerResidueKind::AppSupport,
            );
            push_related_root(
                &mut roots,
                "系统缓存目录",
                PathBuf::from("/Library/Caches").join(alias),
                AppManagerScope::System,
                AppManagerResidueKind::Cache,
            );
            push_related_root(
                &mut roots,
                "系统偏好设置",
                PathBuf::from("/Library/Preferences").join(format!("{alias}.plist")),
                AppManagerScope::System,
                AppManagerResidueKind::Preferences,
            );
            push_related_root(
                &mut roots,
                "系统日志目录",
                PathBuf::from("/Library/Logs").join(alias),
                AppManagerScope::System,
                AppManagerResidueKind::Logs,
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
                    AppManagerScope::User,
                    AppManagerResidueKind::AppData,
                );
            }
            let startup_name = format!("{}.lnk", item.name);
            push_related_root(
                &mut roots,
                "用户启动项目录",
                PathBuf::from(app_data)
                    .join("Microsoft/Windows/Start Menu/Programs/Startup")
                    .join(startup_name),
                AppManagerScope::User,
                AppManagerResidueKind::Startup,
            );
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            for alias in &aliases {
                push_related_root(
                    &mut roots,
                    "Local 数据目录",
                    PathBuf::from(&local_app_data).join(alias),
                    AppManagerScope::User,
                    AppManagerResidueKind::AppData,
                );
            }
        }
        if let Some(program_data) = std::env::var_os("ProgramData") {
            for alias in &aliases {
                push_related_root(
                    &mut roots,
                    "ProgramData 目录",
                    PathBuf::from(&program_data).join(alias),
                    AppManagerScope::System,
                    AppManagerResidueKind::AppData,
                );
            }
            let startup_name = format!("{}.lnk", item.name);
            push_related_root(
                &mut roots,
                "系统启动项目录",
                PathBuf::from(program_data)
                    .join("Microsoft/Windows/Start Menu/Programs/Startup")
                    .join(startup_name),
                AppManagerScope::System,
                AppManagerResidueKind::Startup,
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

fn residue_kind_prefers_file(kind: AppManagerResidueKind) -> bool {
    matches!(
        kind,
        AppManagerResidueKind::Preferences
            | AppManagerResidueKind::Startup
            | AppManagerResidueKind::AppScript
            | AppManagerResidueKind::LaunchAgent
            | AppManagerResidueKind::LaunchDaemon
            | AppManagerResidueKind::RegistryValue
    )
}

fn has_directory_suffix_hint(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let lower = file_name.to_ascii_lowercase();
    const DIRECTORY_SUFFIXES: [&str; 8] = [
        ".app",
        ".bundle",
        ".framework",
        ".appex",
        ".plugin",
        ".kext",
        ".savedstate",
        ".photoslibrary",
    ];
    DIRECTORY_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn has_file_extension_hint(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "plist"
            | "lnk"
            | "json"
            | "xml"
            | "toml"
            | "yaml"
            | "yml"
            | "ini"
            | "conf"
            | "cfg"
            | "log"
            | "txt"
            | "db"
            | "sqlite"
            | "sqlite3"
    )
}

pub(super) fn detect_path_type(
    path: &Path,
    kind: AppManagerResidueKind,
    filesystem: bool,
) -> AppManagerPathType {
    if filesystem && path.is_dir() {
        return AppManagerPathType::Directory;
    }
    if filesystem && path.is_file() {
        return AppManagerPathType::File;
    }
    if kind == AppManagerResidueKind::RegistryKey {
        return AppManagerPathType::Directory;
    }
    if kind == AppManagerResidueKind::RegistryValue {
        return AppManagerPathType::File;
    }
    if has_directory_suffix_hint(path) {
        return AppManagerPathType::Directory;
    }
    if residue_kind_prefers_file(kind) && has_file_extension_hint(path) {
        return AppManagerPathType::File;
    }
    AppManagerPathType::Directory
}

pub(super) fn build_app_detail(app: ManagedAppDto) -> ManagedAppDetailDto {
    let app_size_path = resolve_app_size_path(Path::new(app.path.as_str()));
    let size_snapshot = resolve_app_size_snapshot(app_size_path.as_path());
    let app_size_bytes = size_snapshot.size_bytes.or(app.size_bytes);
    let related_roots = collect_related_root_specs(&app)
        .into_iter()
        .map(|root| {
            let exists = root.path.exists();
            let mut readonly_reason_code = None;
            let readonly = if exists {
                let is_policy_managed = root.scope == AppManagerScope::System
                    && root.kind == AppManagerResidueKind::Startup;
                let ro = is_policy_managed || path_is_readonly(root.path.as_path());
                if is_policy_managed {
                    readonly_reason_code = Some(AppReadonlyReasonCode::ManagedByPolicy);
                } else if ro {
                    readonly_reason_code = Some(AppReadonlyReasonCode::PermissionDenied);
                }
                ro
            } else {
                false
            };
            AppRelatedRootDto {
                id: stable_hash(
                    format!(
                        "{}|{}|{}",
                        app.id,
                        root.kind.as_str(),
                        root.path.to_string_lossy()
                    )
                    .as_str(),
                ),
                label: root.label,
                path: root.path.to_string_lossy().to_string(),
                path_type: detect_path_type(root.path.as_path(), root.kind, true),
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
            app_bytes: app_size_bytes,
            residue_bytes: None,
            total_bytes: app_size_bytes,
        },
        related_roots,
        app,
    }
}

pub(super) fn append_scan_size_warnings(
    warnings: &mut Vec<AppManagerScanWarningDto>,
    warning_keys: &mut HashSet<(
        AppManagerScanWarningCode,
        String,
        AppManagerScanWarningDetailCode,
    )>,
    path_warnings: Vec<PathSizeWarning>,
) {
    const MAX_SCAN_WARNINGS: usize = 20;
    for warning in path_warnings {
        if warnings.len() >= MAX_SCAN_WARNINGS {
            break;
        }
        let key = (warning.code, warning.path.clone(), warning.detail_code);
        if !warning_keys.insert(key) {
            continue;
        }
        warnings.push(AppManagerScanWarningDto {
            code: warning.code,
            path: Some(warning.path),
            detail_code: Some(warning.detail_code),
        });
    }
}

fn append_scan_warning(
    warnings: &mut Vec<AppManagerScanWarningDto>,
    warning_keys: &mut HashSet<(
        AppManagerScanWarningCode,
        String,
        AppManagerScanWarningDetailCode,
    )>,
    warning: AppManagerScanWarningDto,
) {
    let path = warning.path.clone().unwrap_or_default();
    let detail_code = warning
        .detail_code
        .unwrap_or(AppManagerScanWarningDetailCode::IoOther);
    let key = (warning.code, path, detail_code);
    if warning_keys.insert(key) {
        warnings.push(warning);
    }
}

pub(super) fn collect_quick_residue_candidates(
    item: &ManagedAppDto,
    profile: &ResidueIdentityProfile,
) -> Vec<ResidueCandidate> {
    let mut candidates = collect_related_root_specs(item)
        .iter()
        .filter_map(candidate_from_related_root)
        .collect::<Vec<_>>();

    let identifiers = profile_identifiers(profile);

    #[cfg(target_os = "macos")]
    {
        for identifier in &identifiers {
            push_macos_identifier_templates(&mut candidates, identifier);
        }
        for alias in collect_app_path_aliases(item) {
            for temp_root in mac_collect_temp_alias_roots(alias.as_str()) {
                candidates.push(ResidueCandidate {
                    path: temp_root,
                    scope: AppManagerScope::User,
                    kind: AppManagerResidueKind::Cache,
                    exists: false,
                    filesystem: true,
                    match_reason: AppManagerResidueMatchReason::RelatedRoot,
                    confidence: AppManagerResidueConfidence::High,
                    evidence: vec![format!("temp_alias:{}", alias)],
                    risk_level: AppManagerRiskLevel::Low,
                    recommended: true,
                    readonly_reason_code: None,
                });
            }
        }
        if let Some(startup_path) = mac_startup_file_path(item.id.as_str()) {
            candidates.push(ResidueCandidate {
                path: startup_path,
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::Startup,
                exists: false,
                filesystem: true,
                match_reason: AppManagerResidueMatchReason::StartupLabel,
                confidence: AppManagerResidueConfidence::Exact,
                evidence: vec!["startup_label_exact".to_string()],
                risk_level: AppManagerRiskLevel::Medium,
                recommended: true,
                readonly_reason_code: None,
            });
        }
        if profile.has_file_provider_extension
            && let Some(home) = home_dir()
        {
            candidates.push(ResidueCandidate {
                path: home.join("Library/Application Support/FileProvider"),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::AppSupport,
                exists: false,
                filesystem: true,
                match_reason: AppManagerResidueMatchReason::ExtensionBundle,
                confidence: AppManagerResidueConfidence::Medium,
                evidence: vec!["file_provider_extension_detected".to_string()],
                risk_level: AppManagerRiskLevel::Low,
                recommended: false,
                readonly_reason_code: None,
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        for identifier in &identifiers {
            push_windows_identifier_templates(&mut candidates, identifier);
        }

        if let Some(app_data) = std::env::var_os("APPDATA") {
            let startup_name = format!("{}.lnk", item.name);
            candidates.push(ResidueCandidate {
                path: PathBuf::from(app_data)
                    .join("Microsoft/Windows/Start Menu/Programs/Startup")
                    .join(startup_name),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::Startup,
                exists: false,
                filesystem: true,
                match_reason: AppManagerResidueMatchReason::StartupShortcut,
                confidence: AppManagerResidueConfidence::High,
                evidence: vec!["startup_shortcut_path".to_string()],
                risk_level: AppManagerRiskLevel::Medium,
                recommended: true,
                readonly_reason_code: None,
            });
        }
        candidates.extend(windows_collect_registry_residue_candidates(item));
    }

    candidates
}

#[cfg(target_os = "macos")]
fn push_macos_identifier_templates(
    candidates: &mut Vec<ResidueCandidate>,
    identifier: &ResidueIdentifier,
) {
    if let Some(home) = home_dir() {
        let base = home.join("Library");
        push_fs_template_candidate(
            candidates,
            base.join("Application Scripts")
                .join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::AppScript,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("Containers").join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::Container,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("Group Containers")
                .join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::GroupContainer,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("Application Support")
                .join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::AppSupport,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("Caches").join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::Cache,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("HTTPStorages").join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::Cache,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("Preferences")
                .join(format!("{}.plist", identifier.value)),
            AppManagerScope::User,
            AppManagerResidueKind::Preferences,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("Logs").join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::Logs,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("Saved Application State")
                .join(format!("{}.savedState", identifier.value)),
            AppManagerScope::User,
            AppManagerResidueKind::SavedState,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            base.join("WebKit").join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::WebkitData,
            identifier,
        );
    }

    push_fs_template_candidate(
        candidates,
        PathBuf::from("/Library/Application Support").join(identifier.value.as_str()),
        AppManagerScope::System,
        AppManagerResidueKind::AppSupport,
        identifier,
    );
    push_fs_template_candidate(
        candidates,
        PathBuf::from("/Library/Caches").join(identifier.value.as_str()),
        AppManagerScope::System,
        AppManagerResidueKind::Cache,
        identifier,
    );
    push_fs_template_candidate(
        candidates,
        PathBuf::from("/Library/Preferences").join(format!("{}.plist", identifier.value)),
        AppManagerScope::System,
        AppManagerResidueKind::Preferences,
        identifier,
    );
    push_fs_template_candidate(
        candidates,
        PathBuf::from("/Library/Logs").join(identifier.value.as_str()),
        AppManagerScope::System,
        AppManagerResidueKind::Logs,
        identifier,
    );
    push_fs_template_candidate(
        candidates,
        PathBuf::from("/Library/LaunchAgents").join(format!("{}.plist", identifier.value)),
        AppManagerScope::System,
        AppManagerResidueKind::LaunchAgent,
        identifier,
    );
    push_fs_template_candidate(
        candidates,
        PathBuf::from("/Library/LaunchDaemons").join(format!("{}.plist", identifier.value)),
        AppManagerScope::System,
        AppManagerResidueKind::LaunchDaemon,
        identifier,
    );
    push_fs_template_candidate(
        candidates,
        PathBuf::from("/Library/PrivilegedHelperTools").join(identifier.value.as_str()),
        AppManagerScope::System,
        AppManagerResidueKind::HelperTool,
        identifier,
    );
    push_fs_template_candidate(
        candidates,
        PathBuf::from("/private/var/db/receipts").join(format!("{}.bom", identifier.value)),
        AppManagerScope::System,
        AppManagerResidueKind::AppSupport,
        identifier,
    );
    push_fs_template_candidate(
        candidates,
        PathBuf::from("/private/var/db/receipts").join(format!("{}.plist", identifier.value)),
        AppManagerScope::System,
        AppManagerResidueKind::AppSupport,
        identifier,
    );
}

#[cfg(target_os = "windows")]
fn push_windows_identifier_templates(
    candidates: &mut Vec<ResidueCandidate>,
    identifier: &ResidueIdentifier,
) {
    if let Some(app_data) = std::env::var_os("APPDATA") {
        push_fs_template_candidate(
            candidates,
            PathBuf::from(app_data).join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::AppData,
            identifier,
        );
    }

    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        let local_root = PathBuf::from(local_app_data);
        push_fs_template_candidate(
            candidates,
            local_root.join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::AppData,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            local_root.join("Packages").join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::AppData,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            local_root.join("Programs").join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::AppData,
            identifier,
        );
        push_fs_template_candidate(
            candidates,
            local_root.join("Temp").join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::Cache,
            identifier,
        );
    }

    if let Some(home) = home_dir() {
        push_fs_template_candidate(
            candidates,
            home.join("AppData/LocalLow")
                .join(identifier.value.as_str()),
            AppManagerScope::User,
            AppManagerResidueKind::AppData,
            identifier,
        );
    }

    if let Some(program_data) = std::env::var_os("ProgramData") {
        push_fs_template_candidate(
            candidates,
            PathBuf::from(program_data).join(identifier.value.as_str()),
            AppManagerScope::System,
            AppManagerResidueKind::AppData,
            identifier,
        );
    }
}

fn push_fs_template_candidate(
    candidates: &mut Vec<ResidueCandidate>,
    path: PathBuf,
    scope: AppManagerScope,
    kind: AppManagerResidueKind,
    identifier: &ResidueIdentifier,
) {
    candidates.push(ResidueCandidate {
        path,
        scope,
        kind,
        exists: false,
        filesystem: true,
        match_reason: identifier.match_reason,
        confidence: AppManagerResidueConfidence::Exact,
        evidence: vec![format!(
            "template_identifier:{}:{}",
            format!("{:?}", identifier.match_reason).to_ascii_lowercase(),
            identifier.value
        )],
        risk_level: AppManagerRiskLevel::Low,
        recommended: true,
        readonly_reason_code: None,
    });
}

#[cfg(target_os = "windows")]
fn windows_registry_scope(reg_path: &str) -> AppManagerScope {
    if reg_path.to_ascii_uppercase().starts_with("HKCU\\")
        || reg_path
            .to_ascii_uppercase()
            .starts_with("HKEY_CURRENT_USER\\")
    {
        return AppManagerScope::User;
    }
    AppManagerScope::System
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
        let scope = windows_registry_scope(entry.registry_key.as_str());
        candidates.push(ResidueCandidate {
            path: PathBuf::from(entry.registry_key),
            scope,
            kind: AppManagerResidueKind::RegistryKey,
            exists: true,
            filesystem: false,
            match_reason: AppManagerResidueMatchReason::UninstallRegistry,
            confidence: AppManagerResidueConfidence::Exact,
            evidence: vec!["uninstall_registry_match".to_string()],
            risk_level: if scope == AppManagerScope::System {
                AppManagerRiskLevel::High
            } else {
                AppManagerRiskLevel::Medium
            },
            recommended: scope == AppManagerScope::User,
            readonly_reason_code: if scope == AppManagerScope::System {
                Some(AppReadonlyReasonCode::ManagedByPolicy)
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
            scope: AppManagerScope::User,
            kind: AppManagerResidueKind::RegistryValue,
            exists: true,
            filesystem: false,
            match_reason: AppManagerResidueMatchReason::StartupRegistry,
            confidence: AppManagerResidueConfidence::Exact,
            evidence: vec!["startup_registry_value".to_string()],
            risk_level: AppManagerRiskLevel::Medium,
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
            let scope = windows_registry_scope(root);
            candidates.push(ResidueCandidate {
                path: PathBuf::from(format!("{root}::{value_name}")),
                scope,
                kind: AppManagerResidueKind::RegistryValue,
                exists: true,
                filesystem: false,
                match_reason: AppManagerResidueMatchReason::RunRegistry,
                confidence: AppManagerResidueConfidence::High,
                evidence: vec!["run_registry_path_match".to_string()],
                risk_level: if scope == AppManagerScope::System {
                    AppManagerRiskLevel::High
                } else {
                    AppManagerRiskLevel::Medium
                },
                recommended: scope == AppManagerScope::User,
                readonly_reason_code: if scope == AppManagerScope::System {
                    Some(AppReadonlyReasonCode::ManagedByPolicy)
                } else {
                    None
                },
            });
        }
    }

    candidates
}

fn group_label(kind: AppManagerResidueKind, scope: AppManagerScope) -> String {
    let kind_label = match kind {
        AppManagerResidueKind::Install => "安装目录",
        AppManagerResidueKind::AppSupport => "应用支持目录",
        AppManagerResidueKind::Cache => "缓存目录",
        AppManagerResidueKind::Preferences => "偏好设置",
        AppManagerResidueKind::Logs => "日志目录",
        AppManagerResidueKind::Startup => "启动项",
        AppManagerResidueKind::AppScript => "应用脚本目录",
        AppManagerResidueKind::Container => "容器目录",
        AppManagerResidueKind::GroupContainer => "组容器目录",
        AppManagerResidueKind::SavedState => "保存状态目录",
        AppManagerResidueKind::WebkitData => "WebKit 数据目录",
        AppManagerResidueKind::LaunchAgent => "启动代理",
        AppManagerResidueKind::LaunchDaemon => "启动守护进程",
        AppManagerResidueKind::HelperTool => "辅助工具",
        AppManagerResidueKind::AppData => "应用数据目录",
        AppManagerResidueKind::RegistryKey => "注册表键",
        AppManagerResidueKind::RegistryValue => "注册表值",
        AppManagerResidueKind::MainApp => "关联目录",
    };
    let scope_label = if scope == AppManagerScope::System {
        "系统级"
    } else {
        "用户级"
    };
    format!("{kind_label} · {scope_label}")
}

pub(super) fn should_replace_residue_candidate(
    current: &ResidueCandidate,
    next: &ResidueCandidate,
) -> bool {
    let current_rank = current.confidence.rank();
    let next_rank = next.confidence.rank();
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

fn risk_level_for_kind(kind: AppManagerResidueKind, scope: AppManagerScope) -> AppManagerRiskLevel {
    if matches!(
        kind,
        AppManagerResidueKind::LaunchDaemon | AppManagerResidueKind::HelperTool
    ) {
        return AppManagerRiskLevel::High;
    }
    if matches!(
        kind,
        AppManagerResidueKind::Preferences
            | AppManagerResidueKind::Startup
            | AppManagerResidueKind::LaunchAgent
    ) {
        return AppManagerRiskLevel::Medium;
    }
    if matches!(
        kind,
        AppManagerResidueKind::RegistryKey | AppManagerResidueKind::RegistryValue
    ) {
        return if scope == AppManagerScope::System {
            AppManagerRiskLevel::High
        } else {
            AppManagerRiskLevel::Medium
        };
    }
    AppManagerRiskLevel::Low
}

fn default_readonly_reason(
    kind: AppManagerResidueKind,
    scope: AppManagerScope,
) -> Option<AppReadonlyReasonCode> {
    if scope == AppManagerScope::System && kind == AppManagerResidueKind::Startup {
        return Some(AppReadonlyReasonCode::ManagedByPolicy);
    }
    None
}

fn default_recommended(
    kind: AppManagerResidueKind,
    scope: AppManagerScope,
    confidence: AppManagerResidueConfidence,
) -> bool {
    if confidence == AppManagerResidueConfidence::Medium {
        return false;
    }
    if matches!(
        kind,
        AppManagerResidueKind::LaunchDaemon | AppManagerResidueKind::HelperTool
    ) {
        return false;
    }
    if scope == AppManagerScope::System
        && matches!(
            kind,
            AppManagerResidueKind::RegistryKey | AppManagerResidueKind::RegistryValue
        )
    {
        return false;
    }
    true
}

fn normalize_candidate(candidate: &mut ResidueCandidate) {
    candidate.risk_level = risk_level_for_kind(candidate.kind, candidate.scope);
    if candidate.readonly_reason_code.is_none() {
        candidate.readonly_reason_code = default_readonly_reason(candidate.kind, candidate.scope);
    }
    candidate.recommended = candidate.recommended
        && default_recommended(candidate.kind, candidate.scope, candidate.confidence);
}

fn candidate_from_related_root(root: &RelatedRootSpec) -> Option<ResidueCandidate> {
    if root.kind == AppManagerResidueKind::Install {
        return None;
    }
    let mut candidate = ResidueCandidate {
        path: root.path.clone(),
        scope: root.scope,
        kind: root.kind,
        exists: false,
        filesystem: true,
        match_reason: AppManagerResidueMatchReason::RelatedRoot,
        confidence: AppManagerResidueConfidence::Exact,
        evidence: vec![format!("related_root:{}", root.kind.as_str())],
        risk_level: AppManagerRiskLevel::Low,
        recommended: true,
        readonly_reason_code: None,
    };
    normalize_candidate(&mut candidate);
    Some(candidate)
}

pub(super) fn build_residue_scan_result(
    item: &ManagedAppDto,
    mode: AppManagerResidueScanMode,
) -> AppManagerResidueScanResultDto {
    let identity = build_residue_identity_profile(item);
    let mut warnings = Vec::new();
    let mut warning_keys: HashSet<(
        AppManagerScanWarningCode,
        String,
        AppManagerScanWarningDetailCode,
    )> = HashSet::new();

    let mut candidates = collect_quick_residue_candidates(item, &identity);

    if mode == AppManagerResidueScanMode::Deep {
        let discovery_result = discover_residue_candidates(&identity);
        candidates.extend(discovery_result.candidates);
        for warning in discovery_result.warnings {
            append_scan_warning(&mut warnings, &mut warning_keys, warning);
        }
    }

    let mut dedup = HashMap::<String, ResidueCandidate>::new();
    for mut candidate in candidates {
        normalize_candidate(&mut candidate);
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
            let computation = exact_path_size_bytes_with_warnings(Path::new(path.as_str()));
            if let Some(computation) = computation {
                append_scan_size_warnings(&mut warnings, &mut warning_keys, computation.warnings);
                computation.size_bytes
            } else {
                0
            }
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
            candidate.readonly_reason_code
        } else if readonly {
            Some(AppReadonlyReasonCode::PermissionDenied)
        } else {
            None
        };
        let item_id =
            stable_hash(format!("{}|{}|{}", item.id, candidate.kind.as_str(), path).as_str());
        let group_key = format!("{}|{}", candidate.scope.as_str(), candidate.kind.as_str());
        let group = grouped
            .entry(group_key.clone())
            .or_insert_with(|| AppManagerResidueGroupDto {
                group_id: stable_hash(group_key.as_str()),
                label: group_label(candidate.kind, candidate.scope),
                scope: candidate.scope,
                kind: candidate.kind,
                total_size_bytes: 0,
                items: Vec::new(),
            });
        group.total_size_bytes = group.total_size_bytes.saturating_add(size_bytes);
        group.items.push(AppManagerResidueItemDto {
            item_id,
            path,
            path_type: detect_path_type(
                candidate.path.as_path(),
                candidate.kind,
                candidate.filesystem,
            ),
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
            .as_str()
            .cmp(right.scope.as_str())
            .then_with(|| left.kind.as_str().cmp(right.kind.as_str()))
    });
    for group in &mut groups {
        group
            .items
            .sort_by(|left, right| left.path.cmp(&right.path));
    }

    AppManagerResidueScanResultDto {
        app_id: item.id.clone(),
        scan_mode: mode,
        total_size_bytes,
        groups,
        warnings,
    }
}

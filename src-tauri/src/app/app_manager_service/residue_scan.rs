use super::*;

#[derive(Debug, Clone)]
pub(super) struct RelatedRootSpec {
    pub(super) label: String,
    pub(super) path: PathBuf,
    pub(super) scope: String,
    pub(super) kind: String,
}

#[derive(Debug, Clone)]
pub(super) struct ResidueCandidate {
    pub(super) path: PathBuf,
    pub(super) scope: String,
    pub(super) kind: String,
    pub(super) exists: bool,
    pub(super) filesystem: bool,
    pub(super) match_reason: String,
    pub(super) confidence: String,
    pub(super) evidence: Vec<String>,
    pub(super) risk_level: String,
    pub(super) recommended: bool,
    pub(super) readonly_reason_code: Option<String>,
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
                    home.join("Library/Preferences")
                        .join(format!("{alias}.plist")),
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

fn detect_path_type(path: &Path) -> &'static str {
    if path.is_dir() {
        return "directory";
    }
    if path.is_file() {
        return "file";
    }
    "unknown"
}

pub(super) fn build_app_detail(app: ManagedAppDto) -> ManagedAppDetailDto {
    let app_size_path = resolve_app_size_path(Path::new(app.path.as_str()));
    let app_size_bytes = exact_path_size_bytes(app_size_path.as_path());
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
                path_type: detect_path_type(root.path.as_path()).to_string(),
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

fn scan_size_warning_message(code: &str) -> &'static str {
    match code {
        "read_dir_failed"
        | "read_dir_entry_failed"
        | "read_file_type_failed"
        | "read_metadata_failed"
        | "metadata_read_failed" => "部分目录读取失败，大小可能偏小",
        "size_estimate_truncated" => "扫描范围受限，大小可能为估算值",
        _ => "大小统计失败，已自动跳过部分路径",
    }
}

fn append_scan_size_warnings(
    warnings: &mut Vec<AppManagerScanWarningDto>,
    warning_keys: &mut HashSet<String>,
    path_warnings: Vec<PathSizeWarning>,
) {
    const MAX_SCAN_WARNINGS: usize = 20;
    for warning in path_warnings {
        if warnings.len() >= MAX_SCAN_WARNINGS {
            break;
        }
        let key = format!("{}|{}", warning.code, warning.path);
        if !warning_keys.insert(key) {
            continue;
        }
        warnings.push(AppManagerScanWarningDto {
            code: format!("app_manager_size_{}", warning.code),
            message: format!(
                "{}：{}",
                scan_size_warning_message(warning.code),
                warning.path
            ),
            detail: Some(warning.detail),
        });
    }
}

fn collect_known_residue_candidates(item: &ManagedAppDto) -> Vec<ResidueCandidate> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if let Some(bundle) = item.bundle_or_app_id.as_deref()
            && let Some(home) = home_dir()
        {
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

pub(super) fn should_replace_residue_candidate(
    current: &ResidueCandidate,
    next: &ResidueCandidate,
) -> bool {
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

pub(super) fn build_residue_scan_result(item: &ManagedAppDto) -> AppManagerResidueScanResultDto {
    let roots = collect_related_root_specs(item);
    let mut warnings = Vec::new();
    let mut warning_keys = HashSet::new();
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
            path_type: detect_path_type(candidate.path.as_path()).to_string(),
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

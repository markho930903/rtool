use super::*;

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub(crate) struct WindowsUninstallEntry {
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
pub(crate) fn windows_uninstall_roots() -> [&'static str; 3] {
    [
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ]
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_list_uninstall_entries() -> Vec<WindowsUninstallEntry> {
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
pub(crate) fn windows_query_uninstall_root(root: &str) -> Vec<WindowsUninstallEntry> {
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
pub(crate) fn windows_parse_reg_value_line(line: &str) -> Option<(String, String)> {
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
pub(crate) fn windows_query_registry_values(root: &str) -> Vec<(String, String)> {
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
pub(crate) fn windows_registry_value_exists(root: &str, value_name: &str) -> bool {
    Command::new("reg")
        .args(["query", root, "/v", value_name])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_find_best_uninstall_entry(
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
pub(crate) fn windows_application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(app_data).join("Microsoft/Windows/Start Menu/Programs"));
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(program_data).join("Microsoft/Windows/Start Menu/Programs"));
    }
    roots
}

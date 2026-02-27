use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanWarningKind {
    ReadDir,
    ReadDirEntry,
    FileType,
    Metadata,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct ScanWarningAggregator {
    pub(crate) read_dir_failed: u64,
    pub(crate) read_dir_entry_failed: u64,
    pub(crate) file_type_failed: u64,
    pub(crate) metadata_failed: u64,
    pub(crate) read_dir_samples: Vec<String>,
    pub(crate) read_dir_entry_samples: Vec<String>,
    pub(crate) file_type_samples: Vec<String>,
    pub(crate) metadata_samples: Vec<String>,
}

impl ScanWarningAggregator {
    pub(crate) fn record(&mut self, kind: ScanWarningKind, path: &Path) {
        let path_text = path.to_string_lossy().to_string();
        match kind {
            ScanWarningKind::ReadDir => {
                self.read_dir_failed = self.read_dir_failed.saturating_add(1);
                push_scan_warning_sample(&mut self.read_dir_samples, path_text);
            }
            ScanWarningKind::ReadDirEntry => {
                self.read_dir_entry_failed = self.read_dir_entry_failed.saturating_add(1);
                push_scan_warning_sample(&mut self.read_dir_entry_samples, path_text);
            }
            ScanWarningKind::FileType => {
                self.file_type_failed = self.file_type_failed.saturating_add(1);
                push_scan_warning_sample(&mut self.file_type_samples, path_text);
            }
            ScanWarningKind::Metadata => {
                self.metadata_failed = self.metadata_failed.saturating_add(1);
                push_scan_warning_sample(&mut self.metadata_samples, path_text);
            }
        }
    }

    fn total_warnings(&self) -> u64 {
        self.read_dir_failed
            .saturating_add(self.read_dir_entry_failed)
            .saturating_add(self.file_type_failed)
            .saturating_add(self.metadata_failed)
    }

    fn log_summary(&self, event_name: &str, root: &str, reason: &str) {
        let total_warnings = self.total_warnings();
        if total_warnings == 0 {
            return;
        }

        tracing::info!(
            event = event_name,
            root,
            reason,
            total_warnings,
            read_dir_failed = self.read_dir_failed,
            read_dir_entry_failed = self.read_dir_entry_failed,
            file_type_failed = self.file_type_failed,
            metadata_failed = self.metadata_failed,
            read_dir_samples = self.read_dir_samples.join(" | "),
            read_dir_entry_samples = self.read_dir_entry_samples.join(" | "),
            file_type_samples = self.file_type_samples.join(" | "),
            metadata_samples = self.metadata_samples.join(" | "),
        );
    }
}

fn push_scan_warning_sample(samples: &mut Vec<String>, value: String) {
    if samples.len() >= SCAN_WARNING_SAMPLE_LIMIT {
        return;
    }
    samples.push(value);
}

#[derive(Debug, Clone)]
pub(super) enum ExclusionRule {
    Segment(String),
    Prefix(String),
    Subpath(String),
    Wildcard(Regex),
}

pub(super) fn scan_index_root_with_rules(
    root: &Path,
    max_depth: usize,
    max_items: usize,
    remaining_total: usize,
    exclusion_rules: &[ExclusionRule],
    source_root: &str,
    reason: RefreshReason,
) -> ScanOutcome {
    if !root.exists() {
        return ScanOutcome {
            entries: Vec::new(),
            truncated: false,
        };
    }

    let hard_limit = max_items.max(1).min(remaining_total.max(1));
    let mut queue = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0usize));

    let mut entries = Vec::new();
    let mut truncated = false;
    let mut processed: usize = 0;
    let mut warning_aggregator = ScanWarningAggregator::default();
    let home_normalized = current_home_dir()
        .map(|value| normalize_path_for_match(value.as_path()))
        .filter(|value| !value.is_empty());

    while let Some((current_dir, depth)) = queue.pop_front() {
        if entries.len() >= hard_limit {
            truncated = true;
            break;
        }

        let dir_entries = match fs::read_dir(&current_dir) {
            Ok(dir_entries) => dir_entries,
            Err(_error) => {
                warning_aggregator.record(ScanWarningKind::ReadDir, current_dir.as_path());
                continue;
            }
        };

        let is_root_level = depth == 0 && normalize_path_for_match(root) == "/";
        let mut dir_entries = dir_entries
            .filter_map(|entry| match entry {
                Ok(entry) => Some(entry),
                Err(_error) => {
                    warning_aggregator.record(ScanWarningKind::ReadDirEntry, current_dir.as_path());
                    None
                }
            })
            .map(|entry| {
                let path = entry.path();
                let normalized = normalize_path_for_match(path.as_path());
                let priority = scan_priority_for_path(
                    normalized.as_str(),
                    is_root_level,
                    home_normalized.as_deref(),
                );
                (entry, normalized, priority)
            })
            .collect::<Vec<_>>();
        dir_entries.sort_by(|left, right| left.2.cmp(&right.2).then_with(|| left.1.cmp(&right.1)));

        for (dir_entry, _, _) in dir_entries {
            if entries.len() >= hard_limit {
                truncated = true;
                break;
            }

            processed = processed.saturating_add(1);
            if processed.is_multiple_of(SCAN_YIELD_EVERY) {
                thread::sleep(SCAN_YIELD_SLEEP);
            }

            let path = dir_entry.path();
            let file_type = match dir_entry.file_type() {
                Ok(file_type) => file_type,
                Err(_error) => {
                    warning_aggregator.record(ScanWarningKind::FileType, path.as_path());
                    continue;
                }
            };

            if file_type.is_symlink() {
                continue;
            }

            let normalized_path = normalize_path_for_match(path.as_path());
            if should_exclude_path(path.as_path(), normalized_path.as_str(), exclusion_rules) {
                continue;
            }

            if file_type.is_dir() {
                let kind = if is_application_candidate(path.as_path(), true) {
                    IndexedEntryKind::Application
                } else {
                    IndexedEntryKind::Directory
                };
                if let Some(entry) = build_index_entry(
                    path.as_path(),
                    kind,
                    source_root,
                    Some(&mut warning_aggregator),
                ) {
                    entries.push(entry);
                }

                if depth < max_depth && !should_skip_dir_traversal(path.as_path()) {
                    queue.push_back((path, depth + 1));
                }
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            let kind = if is_application_candidate(path.as_path(), false) {
                IndexedEntryKind::Application
            } else {
                IndexedEntryKind::File
            };
            if let Some(entry) = build_index_entry(
                path.as_path(),
                kind,
                source_root,
                Some(&mut warning_aggregator),
            ) {
                entries.push(entry);
            }
        }
    }

    warning_aggregator.log_summary(
        "launcher_index_scan_warning_summary",
        source_root,
        reason.as_str(),
    );

    ScanOutcome { entries, truncated }
}

pub(crate) fn build_index_entry(
    path: &Path,
    kind: IndexedEntryKind,
    source_root: &str,
    warning_aggregator: Option<&mut ScanWarningAggregator>,
) -> Option<LauncherIndexEntry> {
    let path_value = path.to_string_lossy().to_string();
    if path_value.trim().is_empty() {
        return None;
    }

    let raw_name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| path_value.clone());
    let name = if matches!(kind, IndexedEntryKind::Application) {
        application_title(path, raw_name.as_str())
    } else {
        raw_name
    };
    let parent = path
        .parent()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path_value.clone());
    let ext = if !matches!(kind, IndexedEntryKind::Directory) {
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
    } else {
        None
    };

    let metadata = if matches!(kind, IndexedEntryKind::File) {
        match fs::metadata(path) {
            Ok(metadata) => Some(metadata),
            Err(_error) => {
                if let Some(aggregator) = warning_aggregator {
                    aggregator.record(ScanWarningKind::Metadata, path);
                }
                None
            }
        }
    } else {
        None
    };
    let mtime = metadata
        .as_ref()
        .and_then(|value| value.modified().ok())
        .and_then(system_time_to_unix_millis);
    let size = metadata
        .as_ref()
        .map(|value| value.len())
        .and_then(|value| i64::try_from(value).ok());

    let searchable_text = normalize_query(
        format!(
            "{} {} {}",
            name,
            path_value,
            ext.clone().unwrap_or_default()
        )
        .as_str(),
    );

    Some(LauncherIndexEntry {
        path: path_value,
        kind,
        name,
        parent,
        ext,
        mtime,
        size,
        source_root: source_root.to_string(),
        searchable_text,
    })
}

pub(super) fn build_exclusion_rules(patterns: &[String]) -> Vec<ExclusionRule> {
    patterns
        .iter()
        .map(|pattern| normalize_path_pattern(pattern))
        .filter(|pattern| !pattern.is_empty())
        .filter_map(|pattern| {
            if pattern.contains('*') || pattern.contains('?') {
                return wildcard_to_regex(pattern.as_str()).map(ExclusionRule::Wildcard);
            }
            if pattern.contains('/') || pattern.contains(':') {
                if !is_absolute_pattern(pattern.as_str()) {
                    return Some(ExclusionRule::Subpath(pattern));
                }
                return Some(ExclusionRule::Prefix(pattern));
            }
            Some(ExclusionRule::Segment(pattern))
        })
        .collect()
}

fn should_exclude_path(path: &Path, normalized_path: &str, rules: &[ExclusionRule]) -> bool {
    if is_hidden(path) {
        return true;
    }

    for rule in rules {
        match rule {
            ExclusionRule::Segment(value) => {
                if path_has_component(path, value.as_str()) {
                    return true;
                }
            }
            ExclusionRule::Prefix(value) => {
                if normalized_path == value
                    || normalized_path
                        .strip_prefix(value.as_str())
                        .is_some_and(|tail| tail.starts_with('/'))
                {
                    return true;
                }
            }
            ExclusionRule::Subpath(value) => {
                if normalized_path == value
                    || normalized_path.ends_with(format!("/{value}").as_str())
                    || normalized_path.contains(format!("/{value}/").as_str())
                {
                    return true;
                }
            }
            ExclusionRule::Wildcard(regex) => {
                if regex.is_match(normalized_path) {
                    return true;
                }
            }
        }
    }
    false
}

fn wildcard_to_regex(pattern: &str) -> Option<Regex> {
    let mut regex = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '\\' | '.' | '+' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '$' | '|' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }
    regex.push('$');
    Regex::new(regex.as_str()).ok()
}

fn is_absolute_pattern(pattern: &str) -> bool {
    if pattern.starts_with('/') {
        return true;
    }
    if pattern.len() >= 2 {
        let bytes = pattern.as_bytes();
        if bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
            return true;
        }
    }
    false
}

fn path_has_component(path: &Path, target: &str) -> bool {
    path.components().any(|component| match component {
        Component::Normal(value) => value
            .to_string_lossy()
            .to_ascii_lowercase()
            .eq_ignore_ascii_case(target),
        _ => false,
    })
}

pub(crate) fn scan_priority_for_path(
    normalized_path: &str,
    is_root_level: bool,
    home_normalized: Option<&str>,
) -> u8 {
    if !is_root_level {
        return 3;
    }

    if home_normalized.is_some_and(|home| path_is_same_or_ancestor(normalized_path, home)) {
        return 0;
    }
    if normalized_path == "/applications" {
        return 1;
    }
    if path_is_same_or_ancestor(normalized_path, "/system/applications") {
        return 2;
    }
    3
}

fn path_is_same_or_ancestor(path: &str, target: &str) -> bool {
    path == target
        || target
            .strip_prefix(path)
            .is_some_and(|tail| tail.starts_with('/'))
}

fn should_skip_dir_traversal(path: &Path) -> bool {
    cfg!(target_os = "macos") && has_extension_ignore_ascii_case(path, "app")
}

fn is_application_candidate(path: &Path, is_dir: bool) -> bool {
    if cfg!(target_os = "macos") {
        return is_dir && has_extension_ignore_ascii_case(path, "app");
    }

    if cfg!(target_os = "windows") {
        return path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "lnk" | "exe" | "url" | "appref-ms"
                )
            });
    }

    has_extension_ignore_ascii_case(path, "desktop")
}

fn application_title(path: &Path, fallback_name: &str) -> String {
    if is_linux_desktop_entry(path) {
        if let Some(title) = read_linux_desktop_name(path) {
            return title;
        }
    }

    path.file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| fallback_name.to_string())
}

fn has_extension_ignore_ascii_case(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn is_linux_desktop_entry(path: &Path) -> bool {
    cfg!(target_os = "linux") && has_extension_ignore_ascii_case(path, "desktop")
}

fn read_linux_desktop_name(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("Name=") {
            let title = value.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn current_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn system_time_to_unix_millis(value: SystemTime) -> Option<i64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

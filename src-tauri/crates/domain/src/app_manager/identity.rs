use super::*;

#[derive(Debug, Clone, Default)]
pub(super) struct ResidueIdentityProfile {
    pub(super) bundle_ids: Vec<String>,
    pub(super) extension_bundle_ids: Vec<String>,
    pub(super) app_group_ids: Vec<String>,
    pub(super) team_ids: Vec<String>,
    pub(super) name_aliases: Vec<String>,
    pub(super) token_aliases: Vec<String>,
    pub(super) has_file_provider_extension: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ResidueIdentifier {
    pub(super) value: String,
    pub(super) match_reason: AppManagerResidueMatchReason,
}

pub(super) fn build_residue_identity_profile(item: &ManagedAppDto) -> ResidueIdentityProfile {
    let mut profile = ResidueIdentityProfile::default();

    if let Some(bundle_id) = item.bundle_or_app_id.as_deref() {
        push_unique(&mut profile.bundle_ids, bundle_id);
    }

    for alias in collect_app_path_aliases(item) {
        push_unique(&mut profile.name_aliases, alias.as_str());
    }

    #[cfg(target_os = "macos")]
    fill_macos_identity_profile(item, &mut profile);

    if profile.bundle_ids.is_empty() {
        if let Some(bundle_id) = item.bundle_or_app_id.as_deref() {
            push_unique(&mut profile.bundle_ids, bundle_id);
        }
    }
    if profile.name_aliases.is_empty() {
        for alias in collect_app_path_aliases(item) {
            push_unique(&mut profile.name_aliases, alias.as_str());
        }
    }

    let mut raw_tokens = Vec::new();
    for value in profile
        .bundle_ids
        .iter()
        .chain(profile.extension_bundle_ids.iter())
        .chain(profile.app_group_ids.iter())
        .chain(profile.team_ids.iter())
        .chain(profile.name_aliases.iter())
    {
        raw_tokens.extend(extract_tokens(value));
    }
    for token in raw_tokens {
        if token.len() >= 3 {
            push_unique(&mut profile.token_aliases, token.as_str());
        }
    }

    profile
}

pub(super) fn profile_identifiers(profile: &ResidueIdentityProfile) -> Vec<ResidueIdentifier> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |value: &str, match_reason: AppManagerResidueMatchReason| {
        let normalized = value.trim();
        if normalized.is_empty() {
            return;
        }
        let key = normalize_path_key(normalized);
        if key.is_empty() || !seen.insert(key) {
            return;
        }
        entries.push(ResidueIdentifier {
            value: normalized.to_string(),
            match_reason,
        });
    };

    for value in &profile.bundle_ids {
        push(value, AppManagerResidueMatchReason::BundleId);
    }
    for value in &profile.extension_bundle_ids {
        push(value, AppManagerResidueMatchReason::ExtensionBundle);
    }
    for value in &profile.app_group_ids {
        push(value, AppManagerResidueMatchReason::EntitlementGroup);
    }
    entries
}

fn push_unique(target: &mut Vec<String>, value: &str) {
    let normalized = value.trim();
    if normalized.is_empty() {
        return;
    }
    let key = normalize_path_key(normalized);
    if key.is_empty() {
        return;
    }
    let exists = target
        .iter()
        .any(|candidate| normalize_path_key(candidate.as_str()) == key);
    if !exists {
        target.push(normalized.to_string());
    }
}

fn extract_tokens(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

#[cfg(target_os = "macos")]
fn fill_macos_identity_profile(item: &ManagedAppDto, profile: &mut ResidueIdentityProfile) {
    let app_root = app_install_root(item);
    let info_plist = app_root.join("Contents").join("Info.plist");
    if let Ok(content) = fs::read_to_string(info_plist)
        && let Some(bundle_id) = plist_value(content.as_str(), "CFBundleIdentifier")
    {
        push_unique(&mut profile.bundle_ids, bundle_id.as_str());
    }

    let plugins_root = app_root.join("Contents").join("PlugIns");
    if let Ok(entries) = fs::read_dir(plugins_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_extension = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("appex"));
            if !is_extension {
                continue;
            }
            let info_path = path.join("Contents").join("Info.plist");
            let Ok(content) = fs::read_to_string(info_path) else {
                continue;
            };
            if let Some(extension_bundle_id) = plist_value(content.as_str(), "CFBundleIdentifier") {
                if extension_bundle_id
                    .to_ascii_lowercase()
                    .contains("fileprovider")
                {
                    profile.has_file_provider_extension = true;
                }
                push_unique(
                    &mut profile.extension_bundle_ids,
                    extension_bundle_id.as_str(),
                );
            }
        }
    }

    let entitlements = read_codesign_entitlements(app_root.as_path());
    apply_entitlements_from_text(profile, entitlements.as_str());
}

#[cfg(target_os = "macos")]
fn read_codesign_entitlements(path: &Path) -> String {
    let output = Command::new("codesign")
        .args(["-d", "--entitlements", ":-"])
        .arg(path)
        .output();
    let Ok(output) = output else {
        return String::new();
    };
    let mut text = String::new();
    text.push_str(String::from_utf8_lossy(&output.stdout).as_ref());
    text.push('\n');
    text.push_str(String::from_utf8_lossy(&output.stderr).as_ref());
    text
}

#[cfg(target_os = "macos")]
pub(super) fn apply_entitlements_from_text(
    profile: &mut ResidueIdentityProfile,
    entitlements: &str,
) {
    if entitlements.trim().is_empty() {
        return;
    }

    let app_groups = plist_array_values(entitlements, "com.apple.security.application-groups");
    for group in app_groups {
        push_unique(&mut profile.app_group_ids, group.as_str());
        if let Some(team_id) = group.split('.').next()
            && team_id.len() >= 6
        {
            push_unique(&mut profile.team_ids, team_id);
        }
    }

    if let Some(team_id) = plist_string_value(entitlements, "com.apple.developer.team-identifier") {
        push_unique(&mut profile.team_ids, team_id.as_str());
    }

    if let Some(application_identifier) = plist_string_value(entitlements, "application-identifier")
        && let Some(team_id) = application_identifier.split('.').next()
    {
        push_unique(&mut profile.team_ids, team_id);
    }
}

#[cfg(target_os = "macos")]
fn plist_string_value(content: &str, key: &str) -> Option<String> {
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

#[cfg(target_os = "macos")]
fn plist_array_values(content: &str, key: &str) -> Vec<String> {
    let pattern = format!(
        r"(?s)<key>{}</key>\s*<array>(.*?)</array>",
        regex::escape(key)
    );
    let Ok(regex) = Regex::new(pattern.as_str()) else {
        return Vec::new();
    };
    let Some(array_body) = regex
        .captures(content)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str())
    else {
        return Vec::new();
    };
    let Ok(string_regex) = Regex::new(r"<string>([^<]+)</string>") else {
        return Vec::new();
    };
    string_regex
        .captures_iter(array_body)
        .filter_map(|captures| captures.get(1))
        .map(|value| value.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

use super::*;

const DISCOVERY_ROOT_ENTRY_LIMIT: usize = 3_000;

#[derive(Debug, Clone)]
struct DiscoveryRootSpec {
    path: PathBuf,
    scope: AppManagerScope,
    kind: AppManagerResidueKind,
}

#[derive(Debug, Clone, Default)]
pub(super) struct DiscoveryResult {
    pub(super) candidates: Vec<ResidueCandidate>,
    pub(super) warnings: Vec<AppManagerScanWarningDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiscoveryPatternMatchKind {
    Exact,
    PrefixOrSuffix,
    Contains,
}

impl DiscoveryPatternMatchKind {
    fn confidence(self) -> AppManagerResidueConfidence {
        match self {
            Self::Exact => AppManagerResidueConfidence::Exact,
            Self::PrefixOrSuffix | Self::Contains => AppManagerResidueConfidence::High,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Exact => 3,
            Self::PrefixOrSuffix => 2,
            Self::Contains => 1,
        }
    }
}

pub(super) fn discover_residue_candidates(profile: &ResidueIdentityProfile) -> DiscoveryResult {
    let identifiers = profile_identifiers(profile);
    if identifiers.is_empty() && profile.token_aliases.is_empty() {
        return DiscoveryResult::default();
    }

    let mut result = DiscoveryResult::default();
    for root in discovery_roots() {
        scan_discovery_root(
            root,
            identifiers.as_slice(),
            profile.token_aliases.as_slice(),
            &mut result,
        );
    }
    result
}

fn scan_discovery_root(
    root: DiscoveryRootSpec,
    identifiers: &[ResidueIdentifier],
    token_aliases: &[String],
    result: &mut DiscoveryResult,
) {
    let entries = match fs::read_dir(root.path.as_path()) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut count = 0usize;
    let mut truncated = false;
    for entry in entries.flatten() {
        count = count.saturating_add(1);
        if count > DISCOVERY_ROOT_ENTRY_LIMIT {
            truncated = true;
            break;
        }

        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if !file_type.is_dir() {
            continue;
        }

        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.trim().is_empty() {
            continue;
        }

        if let Some((match_kind, identifier)) = best_pattern_match(name, identifiers) {
            result.candidates.push(ResidueCandidate {
                path,
                scope: root.scope,
                kind: root.kind,
                exists: true,
                filesystem: true,
                match_reason: AppManagerResidueMatchReason::IdentifierPattern,
                confidence: match_kind.confidence(),
                evidence: vec![format!(
                    "discovery_pattern:{:?}:{}",
                    match_kind, identifier.value
                )],
                risk_level: AppManagerRiskLevel::Low,
                recommended: true,
                readonly_reason_code: None,
            });
            continue;
        }

        let name_tokens = split_discovery_tokens(name);
        let overlap_score = token_overlap_score(name_tokens.as_slice(), token_aliases);
        if overlap_score >= 2 {
            result.candidates.push(ResidueCandidate {
                path,
                scope: root.scope,
                kind: root.kind,
                exists: true,
                filesystem: true,
                match_reason: AppManagerResidueMatchReason::KeywordToken,
                confidence: AppManagerResidueConfidence::Medium,
                evidence: vec![format!("discovery_token_overlap:{overlap_score}")],
                risk_level: AppManagerRiskLevel::Low,
                recommended: false,
                readonly_reason_code: None,
            });
        }
    }

    if truncated {
        result.warnings.push(AppManagerScanWarningDto {
            code: AppManagerScanWarningCode::AppManagerSizeEstimateTruncated,
            path: Some(root.path.to_string_lossy().to_string()),
            detail_code: Some(AppManagerScanWarningDetailCode::LimitReached),
        });
    }
}

fn best_pattern_match<'a>(
    entry_name: &str,
    identifiers: &'a [ResidueIdentifier],
) -> Option<(DiscoveryPatternMatchKind, &'a ResidueIdentifier)> {
    let mut best: Option<(DiscoveryPatternMatchKind, &ResidueIdentifier)> = None;
    for identifier in identifiers {
        let Some(kind) = match_pattern(entry_name, identifier.value.as_str()) else {
            continue;
        };
        match best {
            Some((best_kind, _)) if best_kind.rank() >= kind.rank() => {}
            _ => {
                best = Some((kind, identifier));
            }
        }
    }
    best
}

pub(super) fn match_pattern(
    entry_name: &str,
    identifier: &str,
) -> Option<DiscoveryPatternMatchKind> {
    let entry = entry_name.to_ascii_lowercase();
    let id = identifier.to_ascii_lowercase();
    if entry == id {
        return Some(DiscoveryPatternMatchKind::Exact);
    }
    if entry.starts_with(format!("{id}.").as_str()) || entry.ends_with(format!(".{id}").as_str()) {
        return Some(DiscoveryPatternMatchKind::PrefixOrSuffix);
    }
    if id.len() >= 8 && entry.contains(id.as_str()) {
        return Some(DiscoveryPatternMatchKind::Contains);
    }
    None
}

pub(super) fn split_discovery_tokens(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| token.len() >= 5)
        .map(|token| token.to_ascii_lowercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn token_overlap_score(name_tokens: &[String], token_aliases: &[String]) -> usize {
    if name_tokens.is_empty() || token_aliases.is_empty() {
        return 0;
    }
    let token_set = token_aliases
        .iter()
        .filter(|token| token.len() >= 5)
        .map(|token| token.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    name_tokens
        .iter()
        .filter(|token| token_set.contains(token.as_str()))
        .count()
}

fn discovery_roots() -> Vec<DiscoveryRootSpec> {
    #[cfg(target_os = "macos")]
    {
        let mut roots = Vec::new();
        if let Some(home) = home_dir() {
            roots.push(DiscoveryRootSpec {
                path: home.join("Library/Application Scripts"),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::AppScript,
            });
            roots.push(DiscoveryRootSpec {
                path: home.join("Library/Containers"),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::Container,
            });
            roots.push(DiscoveryRootSpec {
                path: home.join("Library/Group Containers"),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::GroupContainer,
            });
            roots.push(DiscoveryRootSpec {
                path: home.join("Library/Application Support"),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::AppSupport,
            });
            roots.push(DiscoveryRootSpec {
                path: home.join("Library/Caches"),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::Cache,
            });
        }
        roots
    }

    #[cfg(target_os = "windows")]
    {
        let mut roots = Vec::new();
        if let Some(app_data) = std::env::var_os("APPDATA") {
            roots.push(DiscoveryRootSpec {
                path: PathBuf::from(app_data),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::AppData,
            });
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let local_root = PathBuf::from(local_app_data);
            roots.push(DiscoveryRootSpec {
                path: local_root.clone(),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::AppData,
            });
            roots.push(DiscoveryRootSpec {
                path: local_root.join("Packages"),
                scope: AppManagerScope::User,
                kind: AppManagerResidueKind::AppData,
            });
        }
        if let Some(program_data) = std::env::var_os("ProgramData") {
            roots.push(DiscoveryRootSpec {
                path: PathBuf::from(program_data),
                scope: AppManagerScope::System,
                kind: AppManagerResidueKind::AppData,
            });
        }
        roots
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

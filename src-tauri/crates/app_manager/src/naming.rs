use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub(super) struct DisplayNameCandidate {
    value: String,
    confidence: u8,
}

pub(super) fn normalize_display_name(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return None;
    }
    Some(normalized)
}

pub(super) fn path_stem_string(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .and_then(normalize_display_name)
}

pub(super) fn normalize_name_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn split_name_tokens(value: &str) -> Vec<String> {
    normalize_name_key(value)
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

pub(super) fn push_display_name_candidate(
    candidates: &mut Vec<DisplayNameCandidate>,
    value: Option<String>,
    confidence: u8,
) {
    let Some(value) = value.as_deref().and_then(normalize_display_name) else {
        return;
    };
    candidates.push(DisplayNameCandidate { value, confidence });
}

pub(super) fn score_display_name_candidate(
    candidate: &DisplayNameCandidate,
    stem_key: &str,
    stem_tokens: &[String],
) -> i32 {
    let mut score = i32::from(candidate.confidence) * 10;
    let candidate_key = normalize_name_key(candidate.value.as_str());
    let candidate_tokens = candidate_key
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if candidate_tokens.len() >= 2 {
        score += 30;
    }
    if candidate.value.chars().count() >= 8 {
        score += 15;
    }

    if !stem_key.is_empty() && !candidate_key.is_empty() {
        if candidate_key == stem_key {
            score += 80;
        } else {
            if stem_key.contains(candidate_key.as_str()) {
                score += 35;
            }
            if candidate_key.contains(stem_key) {
                score += 20;
            }
            let shared = candidate_tokens
                .iter()
                .filter(|token| stem_tokens.iter().any(|stem| stem == *token))
                .count();
            score += (shared as i32) * 18;
        }
    }

    if candidate_tokens.len() == 1 {
        let len = candidate.value.chars().count();
        let stem_word_count = stem_tokens.len();
        if len <= 4 && stem_word_count >= 2 {
            score -= 90;
        } else if len <= 5 && stem_word_count >= 2 {
            score -= 40;
        }
    }

    if matches!(candidate_key.as_str(), "app" | "application" | "program") {
        score -= 60;
    }

    score
}

pub(super) fn resolve_application_display_name(
    path: &Path,
    path_fallback: &str,
    candidates: Vec<DisplayNameCandidate>,
) -> String {
    let mut dedup = HashMap::<String, DisplayNameCandidate>::new();
    for candidate in candidates {
        let key = candidate.value.to_ascii_lowercase();
        match dedup.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if candidate.confidence > entry.get().confidence {
                    entry.insert(candidate);
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(candidate);
            }
        }
    }

    let mut dedup_candidates = dedup.into_values().collect::<Vec<_>>();
    if dedup_candidates.is_empty() {
        return path_stem_string(path).unwrap_or_else(|| path_fallback.to_string());
    }

    let stem = path_stem_string(path).unwrap_or_else(|| path_fallback.to_string());
    let stem_key = normalize_name_key(stem.as_str());
    let stem_tokens = split_name_tokens(stem.as_str());

    dedup_candidates.sort_by(|left, right| {
        let left_score =
            score_display_name_candidate(left, stem_key.as_str(), stem_tokens.as_slice());
        let right_score =
            score_display_name_candidate(right, stem_key.as_str(), stem_tokens.as_slice());
        right_score
            .cmp(&left_score)
            .then_with(|| right.value.chars().count().cmp(&left.value.chars().count()))
            .then_with(|| left.value.cmp(&right.value))
    });

    dedup_candidates
        .into_iter()
        .next()
        .map(|candidate| candidate.value)
        .unwrap_or_else(|| stem)
}

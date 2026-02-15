use serde::{Deserialize, Serialize};
use std::path::Path;

pub const APP_LOCALE_PREFERENCE_KEY: &str = "app.locale.preference";
pub const SYSTEM_LOCALE_PREFERENCE: &str = "system";
pub const DEFAULT_RESOLVED_LOCALE: &str = "zh-CN";

pub type AppLocalePreference = String;
pub type ResolvedAppLocale = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppLocaleState {
    pub preference: AppLocalePreference,
    pub resolved: ResolvedAppLocale,
}

impl AppLocaleState {
    pub fn new(preference: AppLocalePreference, resolved: ResolvedAppLocale) -> Self {
        Self {
            preference,
            resolved,
        }
    }

    pub fn to_dto(self) -> LocaleStateDto {
        LocaleStateDto {
            preference: self.preference,
            resolved: self.resolved,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocaleStateDto {
    pub preference: AppLocalePreference,
    pub resolved: ResolvedAppLocale,
}

pub fn resolve_system_locale() -> ResolvedAppLocale {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(value) = std::env::var(key)
            && let Some(locale) = normalize_locale_value(&value)
        {
            return locale;
        }
    }

    DEFAULT_RESOLVED_LOCALE.to_string()
}

pub fn normalize_locale_preference(value: &str) -> Option<AppLocalePreference> {
    let normalized = value.trim();
    if normalized.eq_ignore_ascii_case(SYSTEM_LOCALE_PREFERENCE) {
        return Some(SYSTEM_LOCALE_PREFERENCE.to_string());
    }

    normalize_locale_value(normalized)
}

pub fn resolve_locale(preference: &str) -> ResolvedAppLocale {
    if preference == SYSTEM_LOCALE_PREFERENCE {
        return resolve_system_locale();
    }

    normalize_locale_value(preference).unwrap_or_else(|| DEFAULT_RESOLVED_LOCALE.to_string())
}

fn normalize_locale_value(raw: &str) -> Option<String> {
    let normalized = raw.trim().replace('_', "-");
    if normalized.is_empty() {
        return None;
    }

    let mut parts = normalized.split('-');
    let language = parts.next()?.trim().to_lowercase();
    if language.len() != 2 || !language.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }

    let region = parts
        .find(|part| part.len() == 2 && part.chars().all(|ch| ch.is_ascii_alphabetic()))
        .map(|part| part.to_ascii_uppercase());

    if let Some(region) = region {
        return Some(format!("{}-{}", language, region));
    }

    match language.as_str() {
        "zh" => Some("zh-CN".to_string()),
        "en" => Some("en-US".to_string()),
        _ => None,
    }
}

pub fn init_i18n_catalog(app_data_dir: &Path) -> Result<(), String> {
    super::i18n_catalog::initialize(app_data_dir)
}

pub fn t(locale: &str, key: &str) -> String {
    if let Some(value) = super::i18n_catalog::translate(locale, DEFAULT_RESOLVED_LOCALE, key) {
        return value;
    }

    tracing::warn!(event = "i18n_missing_key", locale = locale, key = key);
    key.to_string()
}

use rtool_contracts::models::SettingsDto;
use rtool_kernel::i18n::{
    AppLocalePreference, AppLocaleState, ResolvedAppLocale, SYSTEM_LOCALE_PREFERENCE,
    init_i18n_catalog, normalize_locale_preference, resolve_locale, t,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, Default)]
pub struct LocaleApplicationService;

impl LocaleApplicationService {
    pub fn init_catalog(self, app_data_dir: &Path) -> std::io::Result<()> {
        init_i18n_catalog(app_data_dir).map_err(std::io::Error::other)
    }

    pub fn normalize_preference(self, value: &str) -> Option<AppLocalePreference> {
        normalize_locale_preference(value)
    }

    pub fn resolve(self, preference: &str) -> ResolvedAppLocale {
        resolve_locale(preference)
    }

    pub fn make_state(self, preference: AppLocalePreference) -> AppLocaleState {
        let resolved = self.resolve(preference.as_str());
        AppLocaleState::new(preference, resolved)
    }

    pub fn state_from_settings(self, settings: &SettingsDto) -> AppLocaleState {
        let preference = self
            .normalize_preference(settings.locale.preference.as_str())
            .unwrap_or_else(|| SYSTEM_LOCALE_PREFERENCE.to_string());
        self.make_state(preference)
    }

    pub fn translate(self, locale: &str, key: &str) -> String {
        t(locale, key)
    }
}

use app_application::{ApplicationServices, RuntimeState};
use app_core::i18n::{LocaleStateDto, ResolvedAppLocale};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub app_services: ApplicationServices,
    pub runtime_state: RuntimeState,
}

impl AppState {
    pub fn started_at(&self) -> Instant {
        self.runtime_state.started_at()
    }

    pub fn locale_snapshot(&self) -> LocaleStateDto {
        self.runtime_state.locale_snapshot()
    }

    pub fn resolved_locale(&self) -> ResolvedAppLocale {
        self.runtime_state.resolved_locale()
    }

    pub fn update_locale(&self, preference: String, resolved: ResolvedAppLocale) -> LocaleStateDto {
        self.runtime_state.update_locale(preference, resolved)
    }

    pub fn clipboard_window_compact(&self) -> bool {
        self.runtime_state.clipboard_window_compact()
    }

    pub fn set_clipboard_window_compact(&self, compact: bool) {
        self.runtime_state.set_clipboard_window_compact(compact);
    }
}

use rtool_i18n::i18n::{AppLocaleState, LocaleStateDto, ResolvedAppLocale};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct RuntimeState {
    locale_state: Arc<Mutex<AppLocaleState>>,
    clipboard_window_compact: Arc<Mutex<bool>>,
    started_at: Instant,
}

impl RuntimeState {
    pub fn new(initial_locale_state: AppLocaleState, started_at: Instant) -> Self {
        Self {
            locale_state: Arc::new(Mutex::new(initial_locale_state)),
            clipboard_window_compact: Arc::new(Mutex::new(false)),
            started_at,
        }
    }

    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    fn read_locale_state(&self) -> AppLocaleState {
        match self.locale_state.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn locale_snapshot(&self) -> LocaleStateDto {
        self.read_locale_state().into_dto()
    }

    pub fn resolved_locale(&self) -> ResolvedAppLocale {
        self.read_locale_state().resolved
    }

    pub fn update_locale(&self, preference: String, resolved: ResolvedAppLocale) -> LocaleStateDto {
        let next = AppLocaleState::new(preference, resolved);
        match self.locale_state.lock() {
            Ok(mut guard) => {
                *guard = next.clone();
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = next.clone();
            }
        }
        next.into_dto()
    }

    pub fn clipboard_window_compact(&self) -> bool {
        match self.clipboard_window_compact.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => *poisoned.into_inner(),
        }
    }

    pub fn set_clipboard_window_compact(&self, compact: bool) {
        match self.clipboard_window_compact.lock() {
            Ok(mut guard) => {
                *guard = compact;
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = compact;
            }
        }
    }
}

use crate::i18n::{AppLocaleState, LocaleStateDto, ResolvedAppLocale};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct RuntimeState {
    locale_state: Arc<Mutex<AppLocaleState>>,
    clipboard_window_compact: Arc<Mutex<bool>>,
    screenshot_shortcut_id: Arc<Mutex<Option<u32>>>,
    started_at: Instant,
}

impl RuntimeState {
    pub fn new(
        initial_locale_state: AppLocaleState,
        started_at: Instant,
        screenshot_shortcut_id: Option<u32>,
    ) -> Self {
        Self {
            locale_state: Arc::new(Mutex::new(initial_locale_state)),
            clipboard_window_compact: Arc::new(Mutex::new(false)),
            screenshot_shortcut_id: Arc::new(Mutex::new(screenshot_shortcut_id)),
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

    pub fn screenshot_shortcut_id(&self) -> Option<u32> {
        match self.screenshot_shortcut_id.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => *poisoned.into_inner(),
        }
    }

    pub fn set_screenshot_shortcut_id(&self, shortcut_id: Option<u32>) {
        match self.screenshot_shortcut_id.lock() {
            Ok(mut guard) => {
                *guard = shortcut_id;
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = shortcut_id;
            }
        }
    }
}

use app_clipboard::service::ClipboardService;
use app_core::i18n::{AppLocaleState, LocaleStateDto, ResolvedAppLocale};
use app_infra::db::DbPool;
use app_transfer::service::TransferService;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub db_pool: DbPool,
    pub clipboard_service: ClipboardService,
    pub transfer_service: TransferService,
    pub locale_state: Arc<Mutex<AppLocaleState>>,
    pub clipboard_window_compact: Arc<Mutex<bool>>,
    pub started_at: Instant,
}

impl AppState {
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

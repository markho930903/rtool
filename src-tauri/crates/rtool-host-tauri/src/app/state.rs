use rtool_app::{ApplicationServices, LocaleStateDto, ResolvedAppLocale};
use rtool_kernel::{RuntimeOrchestrator, RuntimeState, RuntimeWorkerStatus};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone)]
pub struct AppContext {
    pub db_path: PathBuf,
    pub app_services: ApplicationServices,
    pub runtime_state: RuntimeState,
    pub runtime_orchestrator: RuntimeOrchestrator,
}

impl AppContext {
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

    pub fn screenshot_shortcut_id(&self) -> Option<u32> {
        self.runtime_state.screenshot_shortcut_id()
    }

    pub fn set_screenshot_shortcut_id(&self, shortcut_id: Option<u32>) {
        self.runtime_state.set_screenshot_shortcut_id(shortcut_id);
    }

    pub fn worker_snapshot(&self) -> Vec<RuntimeWorkerStatus> {
        self.runtime_orchestrator.worker_snapshot()
    }
}

pub type AppState = AppContext;

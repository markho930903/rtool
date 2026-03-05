mod app_manager;
mod bootstrap;
mod clipboard;
mod launcher;
mod locale;
mod logging;
mod screenshot;
mod settings;

pub use app_manager::AppManagerApplicationService;
pub use bootstrap::BootstrapApplicationService;
pub use clipboard::ClipboardApplicationService;
pub use launcher::LauncherApplicationService;
pub use locale::LocaleApplicationService;
pub use logging::LoggingApplicationService;
pub use screenshot::ScreenshotApplicationService;
pub use settings::SettingsApplicationService;

use rtool_capture::service::ClipboardService;
use rtool_data::db::DbConn;

#[derive(Clone)]
pub struct ApplicationServices {
    pub app_manager: AppManagerApplicationService,
    pub clipboard: ClipboardApplicationService,
    pub launcher: LauncherApplicationService,
    pub locale: LocaleApplicationService,
    pub logging: LoggingApplicationService,
    pub screenshot: ScreenshotApplicationService,
    pub settings: SettingsApplicationService,
}

impl ApplicationServices {
    pub fn new(db_conn: DbConn, clipboard_service: ClipboardService) -> Self {
        Self {
            app_manager: AppManagerApplicationService,
            clipboard: ClipboardApplicationService::new(db_conn.clone(), clipboard_service),
            launcher: LauncherApplicationService::new(db_conn.clone()),
            locale: LocaleApplicationService,
            logging: LoggingApplicationService,
            screenshot: ScreenshotApplicationService,
            settings: SettingsApplicationService::new(db_conn),
        }
    }

    pub fn start_background_workers(&self) {
        self.launcher.start_background_indexer();
    }

    pub fn shutdown(&self) {
        LauncherApplicationService::stop_background_indexer();
    }
}

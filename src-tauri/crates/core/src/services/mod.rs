mod app_manager;
mod clipboard;
mod dashboard;
mod launcher;
mod logging;
mod transfer;
mod user_settings;

pub use app_manager::AppManagerApplicationService;
pub use clipboard::ClipboardApplicationService;
pub use dashboard::DashboardApplicationService;
pub use launcher::LauncherApplicationService;
pub use logging::LoggingApplicationService;
pub use transfer::TransferApplicationService;
pub use user_settings::UserSettingsApplicationService;

use rtool_clipboard::service::ClipboardService;
use rtool_db::db::DbConn;
use rtool_transfer::service::TransferService;

#[derive(Clone)]
pub struct ApplicationServices {
    pub app_manager: AppManagerApplicationService,
    pub clipboard: ClipboardApplicationService,
    pub dashboard: DashboardApplicationService,
    pub launcher: LauncherApplicationService,
    pub logging: LoggingApplicationService,
    pub transfer: TransferApplicationService,
    pub settings: UserSettingsApplicationService,
}

impl ApplicationServices {
    pub fn new(
        db_conn: DbConn,
        clipboard_service: ClipboardService,
        transfer_service: TransferService,
    ) -> Self {
        Self {
            app_manager: AppManagerApplicationService,
            clipboard: ClipboardApplicationService::new(db_conn.clone(), clipboard_service),
            dashboard: DashboardApplicationService,
            launcher: LauncherApplicationService::new(db_conn),
            logging: LoggingApplicationService,
            transfer: TransferApplicationService::new(transfer_service),
            settings: UserSettingsApplicationService,
        }
    }
}

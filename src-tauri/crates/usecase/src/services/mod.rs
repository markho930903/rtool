mod app_manager;
mod clipboard;
mod dashboard;
mod launcher;
mod logging;
mod resource_monitor;
mod transfer;
mod user_settings;

pub use app_manager::AppManagerApplicationService;
pub use clipboard::ClipboardApplicationService;
pub use dashboard::DashboardApplicationService;
pub use launcher::LauncherApplicationService;
pub use logging::LoggingApplicationService;
pub use resource_monitor::ResourceMonitorApplicationService;
pub use transfer::TransferApplicationService;
pub use user_settings::UserSettingsApplicationService;

use domain::service::ClipboardService;
use foundation::db::DbConn;
use domain::service::TransferService;

#[derive(Clone)]
pub struct ApplicationServices {
    pub app_manager: AppManagerApplicationService,
    pub clipboard: ClipboardApplicationService,
    pub dashboard: DashboardApplicationService,
    pub launcher: LauncherApplicationService,
    pub logging: LoggingApplicationService,
    pub resource_monitor: ResourceMonitorApplicationService,
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
            resource_monitor: ResourceMonitorApplicationService,
            transfer: TransferApplicationService::new(transfer_service),
            settings: UserSettingsApplicationService,
        }
    }
}

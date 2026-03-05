use rtool_capture::service::ClipboardService;
use rtool_contracts::models::{LogConfigDto, SettingsClipboardDto};
use rtool_contracts::{AppError, AppResult};
use rtool_data::db::{DbConn, init_db, open_db};
use rtool_logging::{LoggingEventSink, LoggingGuard, init_log_center, init_logging};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub struct BootstrapApplicationService;

impl BootstrapApplicationService {
    pub fn init_logging(self, app_data_dir: &Path) -> AppResult<LoggingGuard> {
        init_logging(app_data_dir)
    }

    pub async fn init_log_center(
        self,
        db_conn: DbConn,
        log_dir: PathBuf,
        event_sink: Option<Arc<dyn LoggingEventSink>>,
    ) -> AppResult<LogConfigDto> {
        init_log_center(db_conn, log_dir, event_sink).await
    }

    pub async fn init_database(self, app_data_dir: &Path) -> AppResult<(PathBuf, DbConn)> {
        std::fs::create_dir_all(app_data_dir).map_err(|error| {
            AppError::new(
                "bootstrap_app_data_dir_create_failed",
                "创建应用数据目录失败",
            )
            .with_source(error)
            .with_context("path", app_data_dir.to_string_lossy().to_string())
        })?;

        let db_path = app_data_dir.join("rtool-turso.db");
        let db_conn = open_db(&db_path).await.map_err(AppError::from)?;
        init_db(&db_conn).await.map_err(AppError::from)?;
        Ok((db_path, db_conn))
    }

    pub async fn init_clipboard_service(
        self,
        db_conn: DbConn,
        db_path: PathBuf,
        settings: SettingsClipboardDto,
    ) -> AppResult<ClipboardService> {
        ClipboardService::new(db_conn, db_path, settings).await
    }
}

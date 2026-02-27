use protocol::AppResult;
use protocol::models::{
    LauncherActionDto, LauncherIndexStatusDto, LauncherItemDto, LauncherRebuildResultDto,
    LauncherSearchSettingsDto, LauncherUpdateSearchSettingsInputDto, ResourceModuleIdDto,
};
use rtool_db::db::DbConn;
use rtool_launcher::host::LauncherHost;
use rtool_launcher::launcher::index::{
    get_index_status_async, get_search_settings_async, rebuild_index_now_async,
    reset_search_settings_async, start_background_indexer, stop_background_indexer,
    update_search_settings_async,
};
use rtool_launcher::launcher::service::{execute_launcher_action, search_launcher_async};

#[derive(Clone)]
pub struct LauncherApplicationService {
    db_conn: DbConn,
}

impl LauncherApplicationService {
    pub fn new(db_conn: DbConn) -> Self {
        Self { db_conn }
    }

    pub async fn search(
        &self,
        host: &dyn LauncherHost,
        query: &str,
        limit: Option<u16>,
    ) -> Vec<LauncherItemDto> {
        let (items, diagnostics) = search_launcher_async(host, &self.db_conn, query, limit).await;
        let should_record_index = diagnostics.index_used || diagnostics.index_failed;
        if should_record_index {
            if let Some(duration_ms) = diagnostics.index_query_duration_ms {
                rtool_system::record_module_observation(
                    ResourceModuleIdDto::LauncherIndex,
                    !diagnostics.index_failed,
                    duration_ms,
                );
            }
        }
        items
    }

    pub fn execute(
        &self,
        host: &dyn LauncherHost,
        action: &LauncherActionDto,
    ) -> AppResult<String> {
        execute_launcher_action(host, action)
    }

    pub async fn get_search_settings(&self) -> AppResult<LauncherSearchSettingsDto> {
        get_search_settings_async(&self.db_conn).await
    }

    pub async fn update_search_settings(
        &self,
        input: LauncherUpdateSearchSettingsInputDto,
    ) -> AppResult<LauncherSearchSettingsDto> {
        update_search_settings_async(&self.db_conn, input).await
    }

    pub async fn get_index_status(&self) -> AppResult<LauncherIndexStatusDto> {
        get_index_status_async(&self.db_conn).await
    }

    pub async fn rebuild_index(&self) -> AppResult<LauncherRebuildResultDto> {
        rebuild_index_now_async(&self.db_conn).await
    }

    pub async fn reset_search_settings(&self) -> AppResult<LauncherSearchSettingsDto> {
        reset_search_settings_async(&self.db_conn).await
    }

    pub fn start_background_indexer(&self) {
        start_background_indexer(self.db_conn.clone());
    }

    pub fn stop_background_indexer() {
        stop_background_indexer();
    }
}

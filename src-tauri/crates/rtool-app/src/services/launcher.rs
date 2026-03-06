use rtool_contracts::AppResult;
use rtool_contracts::models::{
    LauncherActionDto, LauncherIndexStatusDto, LauncherRebuildResultDto, LauncherRuntimeStatusDto,
    LauncherSearchDiagnosticsDto, LauncherSearchIndexStateDto, LauncherSearchResponseDto,
    LauncherSearchSettingsDto, LauncherStatusDto, LauncherUpdateSearchSettingsInputDto,
};
use rtool_data::db::DbConn;
use rtool_discovery::launcher::index::{
    get_index_status_async, get_indexer_runtime_status, get_search_settings_async,
    rebuild_index_now_async, reset_search_settings_async, start_background_indexer,
    stop_background_indexer, update_search_settings_async,
};
use rtool_discovery::launcher::service::{
    LauncherSearchDiagnostics, LauncherSearchResult, execute_launcher_action, search_launcher_async,
};
use rtool_platform::launcher::LauncherHost;

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
    ) -> LauncherSearchResponseDto {
        let result = search_launcher_async(host, &self.db_conn, query, limit).await;
        let runtime = get_indexer_runtime_status();
        let index_status = get_index_status_async(&self.db_conn).await.ok();

        build_search_response(query, result, &runtime, index_status.as_ref())
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

    pub async fn get_status(&self) -> AppResult<LauncherStatusDto> {
        let settings = get_search_settings_async(&self.db_conn).await?;
        let index = get_index_status_async(&self.db_conn).await?;
        let runtime = get_indexer_runtime_status();

        Ok(build_status(settings, index, runtime))
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

fn build_search_response(
    query: &str,
    result: LauncherSearchResult,
    runtime: &LauncherRuntimeStatusDto,
    index_status: Option<&LauncherIndexStatusDto>,
) -> LauncherSearchResponseDto {
    let LauncherSearchResult {
        items,
        diagnostics,
        limit,
    } = result;

    LauncherSearchResponseDto {
        query: query.to_string(),
        limit,
        items,
        index: build_search_index_state(runtime, index_status),
        diagnostics: build_search_diagnostics(diagnostics),
    }
}

fn build_search_index_state(
    runtime: &LauncherRuntimeStatusDto,
    index_status: Option<&LauncherIndexStatusDto>,
) -> LauncherSearchIndexStateDto {
    LauncherSearchIndexStateDto {
        ready: index_status.map(|status| status.ready).unwrap_or(false),
        building: runtime.building,
        indexed_items: index_status.map(|status| status.indexed_items).unwrap_or(0),
        truncated: index_status.map(|status| status.truncated).unwrap_or(false),
        last_build_ms: index_status.and_then(|status| status.last_build_ms),
        last_error: index_status.and_then(|status| status.last_error.clone()),
    }
}

fn build_search_diagnostics(
    diagnostics: LauncherSearchDiagnostics,
) -> LauncherSearchDiagnosticsDto {
    LauncherSearchDiagnosticsDto {
        index_used: diagnostics.index_used,
        fallback_to_like: diagnostics.fallback_to_like,
        query_duration_ms: diagnostics.index_query_duration_ms,
    }
}

fn build_status(
    settings: LauncherSearchSettingsDto,
    index: LauncherIndexStatusDto,
    runtime: LauncherRuntimeStatusDto,
) -> LauncherStatusDto {
    LauncherStatusDto {
        runtime,
        index,
        settings,
    }
}

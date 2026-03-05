use crate::app::state::AppState;
use crate::host::launcher::TauriLauncherHost;
use crate::shared::command_runtime::run_command_sync;
use rtool_app::AppManagerApplicationService;
use rtool_contracts::{AppError, AppResult, InvokeError};
use std::path::Path;
use tauri::State;

use super::reveal::reveal_path;
use super::runtime::run_app_manager_command;
use super::watcher::trigger_app_manager_watcher_refresh;

pub(super) async fn run_app_manager_operation<T, F>(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
    command_name: &'static str,
    refresh_on_success: bool,
    operation: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    F: FnOnce(AppManagerApplicationService, TauriLauncherHost) -> AppResult<T> + Send + 'static,
{
    let result = run_app_manager_command(
        app,
        state.app_services.app_manager,
        state.runtime_orchestrator.clone(),
        request_id,
        window_label,
        command_name,
        operation,
    )
    .await;

    if refresh_on_success && result.is_ok() {
        trigger_app_manager_watcher_refresh();
    }

    result
}

pub(super) fn run_reveal_path(
    path: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_sync(
        "app_manager_reveal_path",
        request_id,
        window_label,
        move || {
            let trimmed = path.trim();
            if trimmed.is_empty() {
                return Err(AppError::new(
                    "app_manager_reveal_invalid",
                    "定位失败：路径不能为空",
                ));
            }

            reveal_path(Path::new(trimmed))
        },
    )
}

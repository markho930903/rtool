use crate::host::launcher::TauriLauncherHost;
use crate::shared::command_runtime::run_blocking_command;
use rtool_app::AppManagerApplicationService;
use rtool_contracts::{AppResult, InvokeError};
use rtool_kernel::RuntimeOrchestrator;

use super::watcher::ensure_app_manager_watcher_started;

pub(super) async fn run_app_manager_command<T, F>(
    app: tauri::AppHandle,
    service: AppManagerApplicationService,
    orchestrator: RuntimeOrchestrator,
    request_id: Option<String>,
    window_label: Option<String>,
    command_name: &'static str,
    operation: F,
) -> Result<T, InvokeError>
where
    T: Send + 'static,
    F: FnOnce(AppManagerApplicationService, TauriLauncherHost) -> AppResult<T> + Send + 'static,
{
    ensure_app_manager_watcher_started(&app, service, orchestrator);
    let host = TauriLauncherHost::new(app);
    run_blocking_command(
        command_name,
        request_id,
        window_label,
        command_name,
        move || operation(service, host),
    )
    .await
}

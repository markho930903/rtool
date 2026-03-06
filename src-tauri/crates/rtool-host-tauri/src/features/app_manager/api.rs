use crate::app::state::AppState;
use crate::shared::request_context::InvokeMeta;
use rtool_contracts::{AppResult, InvokeError};
use serde::Serialize;
use serde_json::Value;
use tauri::State;

use super::operations::{run_app_manager_operation, run_reveal_path};
use super::types::{APP_MANAGER_COMMAND_CONTEXT, AppManagerRequest};

async fn dispatch_operation<T, F>(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
    kind: &'static str,
    command_name: &'static str,
    refresh_on_success: bool,
    operation: F,
) -> Result<Value, InvokeError>
where
    T: Serialize + Send + 'static,
    F: FnOnce(
            rtool_app::AppManagerApplicationService,
            crate::host::launcher::TauriLauncherHost,
        ) -> AppResult<T>
        + Send
        + 'static,
{
    let value = run_app_manager_operation(
        app,
        state,
        request_id,
        window_label,
        command_name,
        refresh_on_success,
        operation,
    )
    .await?;

    APP_MANAGER_COMMAND_CONTEXT.serialize(kind, value)
}

pub(crate) async fn handle_app_manager(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: AppManagerRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    let (request_id, window_label) = meta.unwrap_or_default().split();

    match request {
        AppManagerRequest::List(payload) => {
            let query = payload.query.unwrap_or_default();
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "list",
                "app_manager_list",
                false,
                move |service, host| service.list(&host, query),
            )
            .await
        }
        AppManagerRequest::ListSnapshotMeta => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "list_snapshot_meta",
                "app_manager_list_snapshot_meta",
                false,
                move |service, host| service.list_snapshot_meta(&host),
            )
            .await
        }
        AppManagerRequest::ResolveSizes(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "resolve_sizes",
                "app_manager_resolve_sizes",
                false,
                move |service, host| service.resolve_sizes(&host, payload.input),
            )
            .await
        }
        AppManagerRequest::GetDetailCore(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "get_detail_core",
                "app_manager_get_detail_core",
                false,
                move |service, host| service.get_detail_core(&host, payload.query),
            )
            .await
        }
        AppManagerRequest::GetDetailHeavy(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "get_detail_heavy",
                "app_manager_get_detail_heavy",
                false,
                move |service, host| service.get_detail_heavy(&host, payload.input),
            )
            .await
        }
        AppManagerRequest::Cleanup(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "cleanup",
                "app_manager_cleanup",
                true,
                move |service, host| service.cleanup(&host, payload.input),
            )
            .await
        }
        AppManagerRequest::ExportScanResult(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "export_scan_result",
                "app_manager_export_scan_result",
                false,
                move |service, host| service.export_scan_result(&host, payload.input),
            )
            .await
        }
        AppManagerRequest::RefreshIndex => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "refresh_index",
                "app_manager_refresh_index",
                true,
                move |service, host| service.refresh_index(&host),
            )
            .await
        }
        AppManagerRequest::SetStartup(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "set_startup",
                "app_manager_set_startup",
                true,
                move |service, host| service.set_startup(&host, payload.input),
            )
            .await
        }
        AppManagerRequest::Uninstall(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "uninstall",
                "app_manager_uninstall",
                true,
                move |service, host| service.uninstall(&host, payload.input),
            )
            .await
        }
        AppManagerRequest::OpenUninstallHelp(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "open_uninstall_help",
                "app_manager_open_uninstall_help",
                false,
                move |service, host| service.open_uninstall_help(&host, payload.app_id),
            )
            .await
        }
        AppManagerRequest::OpenPermissionHelp(payload) => {
            dispatch_operation(
                app,
                state,
                request_id,
                window_label,
                "open_permission_help",
                "app_manager_open_permission_help",
                false,
                move |service, host| service.open_permission_help(&host, payload.app_id),
            )
            .await
        }
        AppManagerRequest::RevealPath(payload) => {
            run_reveal_path(payload.path, request_id, window_label)?;
            APP_MANAGER_COMMAND_CONTEXT.serialize("reveal_path", Value::Null)
        }
    }
}

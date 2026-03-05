use crate::app::state::AppState;
use crate::shared::request_context::InvokeMeta;
use rtool_contracts::InvokeError;
use serde::Serialize;
use serde_json::Value;
use tauri::State;

use super::operations::{
    app_manager_cleanup, app_manager_export_scan_result, app_manager_get_detail,
    app_manager_get_detail_core, app_manager_get_detail_heavy, app_manager_list,
    app_manager_list_snapshot_meta, app_manager_open_permission_help,
    app_manager_open_uninstall_help, app_manager_refresh_index, app_manager_resolve_sizes,
    app_manager_reveal_path, app_manager_scan_residue, app_manager_set_startup,
    app_manager_uninstall,
};
use super::types::{APP_MANAGER_COMMAND_CONTEXT, AppManagerRequest};

fn serialize_response<T>(kind: &'static str, value: T) -> Result<Value, InvokeError>
where
    T: Serialize,
{
    APP_MANAGER_COMMAND_CONTEXT.serialize(kind, value)
}

macro_rules! dispatch_command {
    ($kind:literal, $expr:expr) => {
        serialize_response($kind, $expr.await?)
    };
}

async fn dispatch_query_commands(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: AppManagerRequest,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request {
        AppManagerRequest::List(payload) => dispatch_command!(
            "list",
            app_manager_list(app, state, payload.query, request_id, window_label)
        ),
        AppManagerRequest::GetDetail(payload) => dispatch_command!(
            "get_detail",
            app_manager_get_detail(app, state, payload.query, request_id, window_label)
        ),
        AppManagerRequest::ListSnapshotMeta => dispatch_command!(
            "list_snapshot_meta",
            app_manager_list_snapshot_meta(app, state, request_id, window_label)
        ),
        AppManagerRequest::ResolveSizes(payload) => dispatch_command!(
            "resolve_sizes",
            app_manager_resolve_sizes(app, state, payload.input, request_id, window_label)
        ),
        AppManagerRequest::GetDetailCore(payload) => dispatch_command!(
            "get_detail_core",
            app_manager_get_detail_core(app, state, payload.query, request_id, window_label)
        ),
        AppManagerRequest::GetDetailHeavy(payload) => dispatch_command!(
            "get_detail_heavy",
            app_manager_get_detail_heavy(app, state, payload.input, request_id, window_label)
        ),
        AppManagerRequest::ScanResidue(payload) => dispatch_command!(
            "scan_residue",
            app_manager_scan_residue(app, state, payload.input, request_id, window_label)
        ),
        AppManagerRequest::ExportScanResult(payload) => dispatch_command!(
            "export_scan_result",
            app_manager_export_scan_result(app, state, payload.input, request_id, window_label)
        ),
        _ => unreachable!("non-query command routed to dispatch_query_commands"),
    }
}

async fn dispatch_action_commands(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: AppManagerRequest,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request {
        AppManagerRequest::Cleanup(payload) => dispatch_command!(
            "cleanup",
            app_manager_cleanup(app, state, payload.input, request_id, window_label)
        ),
        AppManagerRequest::RefreshIndex => dispatch_command!(
            "refresh_index",
            app_manager_refresh_index(app, state, request_id, window_label)
        ),
        AppManagerRequest::SetStartup(payload) => dispatch_command!(
            "set_startup",
            app_manager_set_startup(app, state, payload.input, request_id, window_label)
        ),
        AppManagerRequest::Uninstall(payload) => dispatch_command!(
            "uninstall",
            app_manager_uninstall(app, state, payload.input, request_id, window_label)
        ),
        AppManagerRequest::OpenUninstallHelp(payload) => dispatch_command!(
            "open_uninstall_help",
            app_manager_open_uninstall_help(app, state, payload.app_id, request_id, window_label)
        ),
        AppManagerRequest::OpenPermissionHelp(payload) => dispatch_command!(
            "open_permission_help",
            app_manager_open_permission_help(app, state, payload.app_id, request_id, window_label)
        ),
        _ => unreachable!("non-action command routed to dispatch_action_commands"),
    }
}

fn dispatch_local_commands(
    request: AppManagerRequest,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request {
        AppManagerRequest::RevealPath(payload) => {
            app_manager_reveal_path(payload.path, request_id, window_label)?;
            Ok(Value::Null)
        }
        _ => unreachable!("non-local command routed to dispatch_local_commands"),
    }
}

pub(crate) async fn handle_app_manager(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: AppManagerRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    let (request_id, window_label) = meta.unwrap_or_default().split();

    match request {
        AppManagerRequest::List(_)
        | AppManagerRequest::GetDetail(_)
        | AppManagerRequest::ListSnapshotMeta
        | AppManagerRequest::ResolveSizes(_)
        | AppManagerRequest::GetDetailCore(_)
        | AppManagerRequest::GetDetailHeavy(_)
        | AppManagerRequest::ScanResidue(_)
        | AppManagerRequest::ExportScanResult(_) => {
            dispatch_query_commands(app, state, request, request_id, window_label).await
        }
        AppManagerRequest::Cleanup(_)
        | AppManagerRequest::RefreshIndex
        | AppManagerRequest::SetStartup(_)
        | AppManagerRequest::Uninstall(_)
        | AppManagerRequest::OpenUninstallHelp(_)
        | AppManagerRequest::OpenPermissionHelp(_) => {
            dispatch_action_commands(app, state, request, request_id, window_label).await
        }
        AppManagerRequest::RevealPath(_) => {
            dispatch_local_commands(request, request_id, window_label)
        }
    }
}

use crate::app::state::AppState;
use crate::features::app_manager::api::handle_app_manager;
use crate::features::app_manager::types::AppManagerRequest;
use crate::features::clipboard::api::{ClipboardRequest, handle_clipboard};
use crate::features::launcher::api::{LauncherRequest, handle_launcher};
use crate::features::locale::api::{LocaleRequest, handle_locale};
use crate::features::logging::api::{LoggingRequest, handle_logging};
use crate::features::screenshot::api::{ScreenshotRequest, handle_screenshot};
use crate::features::settings::api::{SettingsRequest, handle_settings};
use crate::shared::request_context::InvokeMeta;
use rtool_contracts::InvokeError;
use serde_json::Value;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) async fn rt_app_manager(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AppManagerRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    handle_app_manager(app, state, request, meta).await
}

#[tauri::command]
pub(crate) async fn rt_clipboard(
    app: AppHandle,
    state: State<'_, AppState>,
    clipboard_plugin: State<'_, tauri_plugin_clipboard::Clipboard>,
    request: ClipboardRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    handle_clipboard(app, state, clipboard_plugin, request, meta).await
}

#[tauri::command]
pub(crate) async fn rt_launcher(
    app: AppHandle,
    state: State<'_, AppState>,
    request: LauncherRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    handle_launcher(app, state, request, meta).await
}

#[tauri::command]
pub(crate) async fn rt_locale(
    app: AppHandle,
    state: State<'_, AppState>,
    request: LocaleRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    handle_locale(app, state, request, meta).await
}

#[tauri::command]
pub(crate) async fn rt_logging(
    request: LoggingRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    handle_logging(request, meta).await
}

#[tauri::command]
pub(crate) async fn rt_screenshot(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ScreenshotRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    handle_screenshot(app, state, request, meta).await
}

#[tauri::command]
pub(crate) async fn rt_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SettingsRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    handle_settings(app, state, request, meta).await
}

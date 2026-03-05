mod image_preview;
mod processor;

use crate::constants::CLIPBOARD_PLUGIN_UPDATE_EVENT;
use rtool_app::ClipboardApplicationService;
use rtool_contracts::{AppError, AppResult};
use std::sync::Arc;
use tauri::{AppHandle, Listener, Manager, Runtime};
use tokio::sync::Mutex;

pub(crate) fn start_clipboard_watcher<R: Runtime>(
    app_handle: AppHandle<R>,
    service: ClipboardApplicationService,
) -> AppResult<()> {
    let clipboard = app_handle.state::<tauri_plugin_clipboard::Clipboard>();
    clipboard
        .start_monitor(app_handle.clone())
        .map_err(|error| {
            AppError::new("clipboard_watcher_start_failed", "剪贴板监听启动失败")
                .with_context("detail", error.to_string())
        })?;

    let processor = Arc::new(Mutex::new(processor::ClipboardProcessor::new(
        app_handle.clone(),
        service,
    )));

    let processor_ref = Arc::clone(&processor);
    let _listener_id = app_handle.listen_any(CLIPBOARD_PLUGIN_UPDATE_EVENT, move |_| {
        let processor_ref = Arc::clone(&processor_ref);
        tauri::async_runtime::spawn(async move {
            let mut guard = processor_ref.lock().await;
            guard.handle_update_event().await;
        });
    });

    Ok(())
}

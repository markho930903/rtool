mod image_preview;
mod processor;

use crate::constants::CLIPBOARD_PLUGIN_UPDATE_EVENT;
use app_clipboard::service::ClipboardService;
use std::sync::Arc;
use tauri::{AppHandle, Listener, Manager, Runtime};
use tokio::sync::Mutex;

pub(crate) fn start_clipboard_watcher<R: Runtime>(
    app_handle: AppHandle<R>,
    service: ClipboardService,
) {
    let clipboard = app_handle.state::<tauri_plugin_clipboard::Clipboard>();
    if let Err(error) = clipboard.start_monitor(app_handle.clone()) {
        tracing::error!(event = "clipboard_watcher_start_failed", error = error);
        return;
    }

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
}

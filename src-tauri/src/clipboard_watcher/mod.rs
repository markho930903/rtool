mod image_preview;
mod processor;

use crate::app::clipboard_service::ClipboardService;
use crate::constants::CLIPBOARD_PLUGIN_UPDATE_EVENT;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Listener, Manager, Runtime};

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
        if let Ok(mut guard) = processor_ref.lock() {
            guard.handle_update_event();
        }
    });
}

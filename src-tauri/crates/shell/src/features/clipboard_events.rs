use protocol::models::ClipboardSyncPayload;
use tauri::{AppHandle, Emitter, Runtime};

const CLIPBOARD_SYNC_EVENT: &str = "rtool://clipboard/sync";

pub fn emit_clipboard_sync<R: Runtime>(app: &AppHandle<R>, payload: ClipboardSyncPayload) {
    if let Err(error) = app.emit(CLIPBOARD_SYNC_EVENT, payload) {
        tracing::warn!(
            event = "clipboard_event_emit_failed",
            event_name = CLIPBOARD_SYNC_EVENT,
            error = error.to_string()
        );
    }
}

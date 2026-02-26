use foundation::models::LogEntryDto;
use foundation::{AppError, AppResult};
use foundation::logging::LoggingEventSink;
use tauri::{AppHandle, Emitter};

const STREAM_EVENT_NAME: &str = "rtool://logging/stream";

#[derive(Clone)]
pub struct TauriLoggingEventSink {
    app_handle: AppHandle,
}

impl TauriLoggingEventSink {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl LoggingEventSink for TauriLoggingEventSink {
    fn emit_stream(&self, entry: &LogEntryDto) -> AppResult<()> {
        self.app_handle
            .emit(STREAM_EVENT_NAME, entry)
            .map_err(|error| {
                AppError::new("logging_event_emit_failed", "推送日志流事件失败")
                    .with_context("event", STREAM_EVENT_NAME)
                    .with_context("detail", error.to_string())
            })
    }
}

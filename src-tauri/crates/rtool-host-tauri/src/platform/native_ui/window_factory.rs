use crate::constants::{
    CLIPBOARD_WINDOW_LABEL, LAUNCHER_OPENED_EVENT, LAUNCHER_WINDOW_LABEL,
};
use rtool_contracts::{AppError, AppResult};
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow, WebviewWindowBuilder};

const WINDOW_PREWARM_LABELS: [&str; 2] = [LAUNCHER_WINDOW_LABEL, CLIPBOARD_WINDOW_LABEL];

#[derive(Default)]
pub(crate) struct WindowWarmupState {
    inner: Mutex<WindowWarmupStateInner>,
}

#[derive(Default)]
struct WindowWarmupStateInner {
    ready_labels: HashSet<String>,
    pending_show_labels: HashSet<String>,
}

impl WindowWarmupState {
    pub(crate) fn request_show_if_not_ready(&self, label: &str) -> bool {
        let mut inner = self.inner.lock().expect("window warmup state poisoned");
        if inner.ready_labels.contains(label) {
            return false;
        }
        inner.pending_show_labels.insert(label.to_string());
        true
    }

    fn mark_ready(&self, label: &str) -> bool {
        let mut inner = self.inner.lock().expect("window warmup state poisoned");
        inner.ready_labels.insert(label.to_string())
    }

    fn take_pending_show(&self, label: &str) -> bool {
        let mut inner = self.inner.lock().expect("window warmup state poisoned");
        inner.pending_show_labels.remove(label)
    }
}

fn finalize_launcher_show<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    if let Err(error) = window.show() {
        tracing::warn!(
            event = "window_show_failed",
            window = LAUNCHER_WINDOW_LABEL,
            error = error.to_string()
        );
        return;
    }

    if let Err(error) = window.set_focus() {
        tracing::warn!(
            event = "window_focus_failed",
            window = LAUNCHER_WINDOW_LABEL,
            error = error.to_string()
        );
    }

    if let Err(error) = app.emit(LAUNCHER_OPENED_EVENT, ()) {
        tracing::warn!(
            event = "window_event_emit_failed",
            event_name = LAUNCHER_OPENED_EVENT,
            error = error.to_string()
        );
    }
}

pub(crate) fn ensure_webview_window<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
) -> AppResult<WebviewWindow<R>> {
    if let Some(window) = app.get_webview_window(label) {
        return Ok(window);
    }

    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|item| item.label == label)
        .ok_or_else(|| {
            AppError::new("window_config_not_found", "窗口配置不存在")
                .with_context("windowLabel", label.to_string())
        })?;

    let builder = WebviewWindowBuilder::from_config(app, config)
        .map_err(|error| {
            AppError::new("window_create_failed", "创建窗口失败")
                .with_context("windowLabel", label.to_string())
                .with_context("detail", error.to_string())
        })?
        .on_page_load({
            let app = app.clone();
            move |window, payload| {
                if !matches!(payload.event(), PageLoadEvent::Finished) {
                    return;
                }

                let label = window.label().to_string();
                let Some(state) = app.try_state::<WindowWarmupState>() else {
                    return;
                };

                let first_ready = state.mark_ready(&label);
                tracing::debug!(
                    event = "window_page_load_finished",
                    window = label.as_str(),
                    first_ready
                );

                if label == LAUNCHER_WINDOW_LABEL && state.take_pending_show(&label) {
                    tracing::info!(
                        event = "window_show_deferred_until_ready",
                        window = label.as_str()
                    );
                    finalize_launcher_show(&app, &window);
                }
            }
        });

    match builder.build() {
        Ok(window) => Ok(window),
        Err(error) => {
            if let Some(window) = app.get_webview_window(label) {
                return Ok(window);
            }

            Err(AppError::new("window_create_failed", "创建窗口失败")
                .with_context("windowLabel", label.to_string())
                .with_context("detail", error.to_string()))
        }
    }
}

pub(crate) fn warmup_secondary_windows<R: Runtime + 'static>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        for label in WINDOW_PREWARM_LABELS {
            let started_at = std::time::Instant::now();
            match ensure_webview_window(&app, label) {
                Ok(_) => {
                    tracing::debug!(
                        event = "window_prewarm_done",
                        window = label,
                        duration_ms = started_at.elapsed().as_millis() as u64
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        event = "window_prewarm_failed",
                        window = label,
                        code = error.code.as_str(),
                        message = error.message.as_str()
                    );
                }
            }
        }
    });
}

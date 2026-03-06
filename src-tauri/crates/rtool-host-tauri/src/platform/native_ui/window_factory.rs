use crate::constants::{CLIPBOARD_WINDOW_LABEL, LAUNCHER_WINDOW_LABEL};
use rtool_contracts::{AppError, AppResult};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow, WebviewWindowBuilder};

const WINDOW_PREWARM_LABELS: [&str; 2] = [LAUNCHER_WINDOW_LABEL, CLIPBOARD_WINDOW_LABEL];

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

    let builder = WebviewWindowBuilder::from_config(app, config).map_err(|error| {
        AppError::new("window_create_failed", "创建窗口失败")
            .with_context("windowLabel", label.to_string())
            .with_context("detail", error.to_string())
    })?;

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

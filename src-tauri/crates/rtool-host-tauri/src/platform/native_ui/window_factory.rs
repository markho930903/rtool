use rtool_contracts::{AppError, AppResult};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow, WebviewWindowBuilder};

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

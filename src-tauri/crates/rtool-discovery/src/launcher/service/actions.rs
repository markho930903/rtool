use crate::host::LauncherHost;
use anyhow::Context;
use rtool_contracts::models::{
    ClipboardWindowOpenedPayload, LauncherActionDto, ScreenshotWindowOpenedPayload,
};
use rtool_contracts::{AppError, AppResult, ResultExt};
use serde::Serialize;
use serde_json::json;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NavigatePayload {
    route: String,
}

pub fn execute_launcher_action(
    app: &dyn LauncherHost,
    action: &LauncherActionDto,
) -> AppResult<String> {
    match action {
        LauncherActionDto::OpenBuiltinRoute { route } => {
            open_main_with_route(app, route.clone()).map(|_| format!("route:{route}"))
        }
        LauncherActionDto::OpenBuiltinTool { tool_id } => {
            let route = format!("/tools/{tool_id}");
            open_main_with_route(app, route.clone()).map(|_| format!("route:{route}"))
        }
        LauncherActionDto::OpenBuiltinWindow { window_label } => {
            let screenshot_payload = if window_label == "screenshot_overlay" {
                let session = rtool_capture::start_session(None)?;
                if let Some(display) = session
                    .displays
                    .iter()
                    .find(|item| item.id == session.active_display_id)
                    .or_else(|| session.displays.first())
                    && let Some(window) = app.get_webview_window(window_label)
                {
                    window.set_position(f64::from(display.x), f64::from(display.y))?;
                    window.set_size(f64::from(display.width), f64::from(display.height))?;
                }
                Some(
                    serde_json::to_value(ScreenshotWindowOpenedPayload { session })
                        .with_context(|| "构造截图窗口事件载荷失败".to_string())
                        .with_code("launcher_emit_payload_failed", "打开窗口失败")?,
                )
            } else {
                None
            };

            open_window(app, window_label)?;

            if window_label == "clipboard_history" {
                if let Err(error) = app.apply_clipboard_window_mode(false, "launcher_open") {
                    tracing::warn!(
                        event = "clipboard_window_mode_apply_failed",
                        source = "launcher_open",
                        compact = false,
                        error = error.to_string()
                    );
                }
                let payload = serde_json::to_value(ClipboardWindowOpenedPayload { compact: false })
                    .with_context(|| "构造剪贴板窗口事件载荷失败".to_string())
                    .with_code("launcher_emit_payload_failed", "打开窗口失败")?;
                app.emit("rtool://clipboard-window/opened", payload)
                    .map_err(|error| error.with_context("windowLabel", window_label))?;
            } else if let Some(payload) = screenshot_payload {
                app.emit("rtool://screenshot-window/opened", payload)
                    .map_err(|error| error.with_context("windowLabel", window_label))?;
            }

            Ok(format!("window:{window_label}"))
        }
        LauncherActionDto::OpenDirectory { path }
        | LauncherActionDto::OpenFile { path }
        | LauncherActionDto::OpenApplication { path } => {
            open_path(Path::new(path)).map(|_| format!("path:{path}"))
        }
    }
}

fn open_main_with_route(app: &dyn LauncherHost, route: String) -> AppResult<()> {
    open_window(app, "main")?;
    app.emit("rtool://main/navigate", json!(NavigatePayload { route }))
        .map_err(|error| {
            error
                .with_code("launcher_navigate_failed", "打开页面失败")
                .with_context("event", "rtool://main/navigate")
        })
}

fn open_window(app: &dyn LauncherHost, label: &str) -> AppResult<()> {
    let window = app.get_webview_window(label).ok_or_else(|| {
        AppError::new("launcher_window_not_found", "目标窗口不存在").with_context("label", label)
    })?;

    window
        .show()
        .with_context(|| format!("显示窗口失败: {label}"))
        .with_code("launcher_window_show_failed", "打开窗口失败")?;
    window
        .set_focus()
        .with_context(|| format!("聚焦窗口失败: {label}"))
        .with_code("launcher_window_focus_failed", "打开窗口失败")
}

fn open_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(
            AppError::new("launcher_path_not_found", "打开失败：路径不存在")
                .with_context("path", path.to_string_lossy().to_string()),
        );
    }

    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(path).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .status()
    } else {
        Command::new("xdg-open").arg(path).status()
    }
    .with_context(|| format!("执行系统打开命令失败: {}", path.to_string_lossy()))
    .with_code("launcher_path_open_failed", "打开失败")?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::new("launcher_path_open_failed", "打开失败")
            .with_context("status", status.to_string()))
    }
}

use crate::host::LauncherHost;
use anyhow::Context;
use rtool_contracts::models::{
    ClipboardWindowOpenedPayload, LauncherActionDto, ScreenshotWindowOpenedPayload,
};
use rtool_contracts::{AppError, AppResult, ResultExt};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;

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
        LauncherActionDto::OpenBuiltinRoute { route } => execute_builtin_route_action(app, route),
        LauncherActionDto::OpenBuiltinTool { tool_id } => execute_builtin_tool_action(app, tool_id),
        LauncherActionDto::OpenBuiltinWindow { window_label } => {
            execute_builtin_window_action(app, window_label)
        }
        LauncherActionDto::OpenDirectory { path }
        | LauncherActionDto::OpenFile { path }
        | LauncherActionDto::OpenApplication { path } => execute_open_path_action(app, path),
    }
}

fn execute_builtin_route_action(app: &dyn LauncherHost, route: &str) -> AppResult<String> {
    open_main_with_route(app, route.to_string())?;
    Ok(format!("route:{route}"))
}

fn execute_builtin_tool_action(app: &dyn LauncherHost, tool_id: &str) -> AppResult<String> {
    let route = format!("/tools/{tool_id}");
    execute_builtin_route_action(app, &route)
}

fn execute_builtin_window_action(app: &dyn LauncherHost, window_label: &str) -> AppResult<String> {
    let screenshot_payload = prepare_window_open(app, window_label)?;
    open_window(app, window_label)?;
    emit_window_opened_event(app, window_label, screenshot_payload)?;
    Ok(format!("window:{window_label}"))
}

fn execute_open_path_action(app: &dyn LauncherHost, path: &str) -> AppResult<String> {
    open_path(app, Path::new(path))?;
    Ok(format!("path:{path}"))
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

fn prepare_window_open(app: &dyn LauncherHost, window_label: &str) -> AppResult<Option<Value>> {
    if window_label != "screenshot_overlay" {
        return Ok(None);
    }

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

    let payload = serde_json::to_value(ScreenshotWindowOpenedPayload { session })
        .with_context(|| "构造截图窗口事件载荷失败".to_string())
        .with_code("launcher_emit_payload_failed", "打开窗口失败")?;

    Ok(Some(payload))
}

fn emit_window_opened_event(
    app: &dyn LauncherHost,
    window_label: &str,
    screenshot_payload: Option<Value>,
) -> AppResult<()> {
    if window_label == "clipboard_history" {
        return emit_clipboard_window_opened(app, window_label);
    }

    if let Some(payload) = screenshot_payload {
        return emit_screenshot_window_opened(app, window_label, payload);
    }

    Ok(())
}

fn emit_clipboard_window_opened(app: &dyn LauncherHost, window_label: &str) -> AppResult<()> {
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
        .map_err(|error| error.with_context("windowLabel", window_label))
}

fn emit_screenshot_window_opened(
    app: &dyn LauncherHost,
    window_label: &str,
    payload: Value,
) -> AppResult<()> {
    app.emit("rtool://screenshot-window/opened", payload)
        .map_err(|error| error.with_context("windowLabel", window_label))
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

fn open_path(app: &dyn LauncherHost, path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(
            AppError::new("launcher_path_not_found", "打开失败：路径不存在")
                .with_context("path", path.to_string_lossy().to_string()),
        );
    }

    app.open_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtool_contracts::models::ClipboardWindowModeAppliedDto;
    use rtool_platform::launcher::{AppPackageInfo, LauncherWindow};
    use serde_json::{Value, json};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[derive(Default)]
    struct MockLauncherWindow;

    impl LauncherWindow for MockLauncherWindow {
        fn show(&self) -> AppResult<()> {
            Ok(())
        }

        fn set_focus(&self) -> AppResult<()> {
            Ok(())
        }

        fn set_position(&self, _x: f64, _y: f64) -> AppResult<()> {
            Ok(())
        }

        fn set_size(&self, _width: f64, _height: f64) -> AppResult<()> {
            Ok(())
        }
    }

    #[derive(Default, Clone)]
    struct MockLauncherHost {
        emitted_events: Arc<Mutex<Vec<(String, Value)>>>,
        opened_paths: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl MockLauncherHost {
        fn take_emitted_events(&self) -> Vec<(String, Value)> {
            self.emitted_events.lock().unwrap().clone()
        }

        fn take_opened_paths(&self) -> Vec<PathBuf> {
            self.opened_paths.lock().unwrap().clone()
        }
    }

    impl LauncherHost for MockLauncherHost {
        fn emit(&self, event: &str, payload: Value) -> AppResult<()> {
            self.emitted_events
                .lock()
                .unwrap()
                .push((event.to_string(), payload));
            Ok(())
        }

        fn get_webview_window(&self, _label: &str) -> Option<Box<dyn LauncherWindow>> {
            Some(Box::new(MockLauncherWindow))
        }

        fn app_data_dir(&self) -> AppResult<PathBuf> {
            Ok(std::env::temp_dir())
        }

        fn package_info(&self) -> AppPackageInfo {
            AppPackageInfo {
                name: "rtool".to_string(),
                version: "test".to_string(),
            }
        }

        fn resolved_locale(&self) -> Option<String> {
            Some("zh-CN".to_string())
        }

        fn apply_clipboard_window_mode(
            &self,
            _compact: bool,
            _source: &str,
        ) -> AppResult<ClipboardWindowModeAppliedDto> {
            Ok(ClipboardWindowModeAppliedDto {
                compact: false,
                applied_width_logical: 0.0,
                applied_height_logical: 0.0,
                scale_factor: 1.0,
            })
        }

        fn open_path(&self, path: &Path) -> AppResult<()> {
            self.opened_paths.lock().unwrap().push(path.to_path_buf());
            Ok(())
        }
    }

    fn create_temp_file() -> PathBuf {
        let path = std::env::temp_dir().join(format!("rtool-launcher-{}.txt", Uuid::new_v4()));
        std::fs::write(&path, "test").unwrap();
        path
    }

    #[test]
    fn open_builtin_tool_action_should_navigate_to_tool_route() {
        let host = MockLauncherHost::default();

        let result = execute_launcher_action(
            &host,
            &LauncherActionDto::OpenBuiltinTool {
                tool_id: "regex".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result, "route:/tools/regex");
        assert_eq!(
            host.take_emitted_events(),
            vec![(
                "rtool://main/navigate".to_string(),
                json!({ "route": "/tools/regex" }),
            )]
        );
    }

    #[test]
    fn open_file_action_should_delegate_to_host_open_path() {
        let host = MockLauncherHost::default();
        let path = create_temp_file();

        let result = execute_launcher_action(
            &host,
            &LauncherActionDto::OpenFile {
                path: path.to_string_lossy().to_string(),
            },
        )
        .unwrap();

        assert_eq!(result, format!("path:{}", path.to_string_lossy()));
        assert_eq!(host.take_opened_paths(), vec![path.clone()]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn open_path_action_should_fail_when_path_missing() {
        let host = MockLauncherHost::default();
        let path = std::env::temp_dir().join(format!("rtool-launcher-missing-{}", Uuid::new_v4()));

        let error = execute_launcher_action(
            &host,
            &LauncherActionDto::OpenDirectory {
                path: path.to_string_lossy().to_string(),
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "launcher_path_not_found");
        assert!(host.take_opened_paths().is_empty());
    }
}

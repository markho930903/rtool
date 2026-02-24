use app_core::{AppError, AppResult};
use app_launcher_app::host::{AppPackageInfo, LauncherHost, LauncherWindow};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub struct TauriLauncherHost {
    app: AppHandle,
}

impl TauriLauncherHost {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

struct TauriLauncherWindow {
    window: WebviewWindow,
}

impl TauriLauncherWindow {
    fn new(window: WebviewWindow) -> Self {
        Self { window }
    }
}

impl LauncherWindow for TauriLauncherWindow {
    fn show(&self) -> AppResult<()> {
        self.window.show().map_err(|error| {
            AppError::new("launcher_window_show_failed", "打开窗口失败")
                .with_context("detail", error.to_string())
        })
    }

    fn set_focus(&self) -> AppResult<()> {
        self.window.set_focus().map_err(|error| {
            AppError::new("launcher_window_focus_failed", "打开窗口失败")
                .with_context("detail", error.to_string())
        })
    }
}

impl LauncherHost for TauriLauncherHost {
    fn emit(&self, event: &str, payload: serde_json::Value) -> AppResult<()> {
        self.app.emit(event, payload).map_err(|error| {
            AppError::new("launcher_event_emit_failed", "发送事件失败")
                .with_context("event", event)
                .with_context("detail", error.to_string())
        })
    }

    fn get_webview_window(&self, label: &str) -> Option<Box<dyn LauncherWindow>> {
        self.app
            .get_webview_window(label)
            .map(|window| Box::new(TauriLauncherWindow::new(window)) as Box<dyn LauncherWindow>)
    }

    fn app_data_dir(&self) -> AppResult<std::path::PathBuf> {
        self.app.path().app_data_dir().map_err(|error| {
            AppError::new("launcher_app_data_dir_unavailable", "获取应用目录失败")
                .with_context("detail", error.to_string())
        })
    }

    fn package_info(&self) -> AppPackageInfo {
        let package_info = self.app.package_info();
        AppPackageInfo {
            name: package_info.name.to_string(),
            version: package_info.version.to_string(),
        }
    }

    fn apply_clipboard_window_mode(
        &self,
        compact: bool,
        source: &str,
    ) -> AppResult<app_core::models::ClipboardWindowModeAppliedDto> {
        crate::platform::native_ui::clipboard_window::apply_clipboard_window_mode(
            &self.app, compact, source,
        )
    }
}

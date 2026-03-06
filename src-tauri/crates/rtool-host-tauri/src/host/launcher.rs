use rtool_contracts::{AppError, AppResult};
use rtool_platform::launcher::{AppPackageInfo, LauncherHost, LauncherWindow};
use std::path::Path;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

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

    fn set_position(&self, x: f64, y: f64) -> AppResult<()> {
        self.window
            .set_position(LogicalPosition::new(x, y))
            .map_err(|error| {
                AppError::new("launcher_window_position_failed", "设置窗口位置失败")
                    .with_context("detail", error.to_string())
            })
    }

    fn set_size(&self, width: f64, height: f64) -> AppResult<()> {
        self.window
            .set_size(LogicalSize::new(width, height))
            .map_err(|error| {
                AppError::new("launcher_window_size_failed", "设置窗口尺寸失败")
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
        crate::platform::native_ui::window_factory::ensure_webview_window(&self.app, label)
            .ok()
            .map(|window| Box::new(TauriLauncherWindow::new(window)) as Box<dyn LauncherWindow>)
    }

    fn open_path(&self, path: &Path) -> AppResult<()> {
        self.app
            .opener()
            .open_path(path.to_string_lossy().to_string(), None::<&str>)
            .map_err(|error| {
                AppError::new("launcher_path_open_failed", "打开失败")
                    .with_context("path", path.to_string_lossy().to_string())
                    .with_context("detail", error.to_string())
            })
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

    fn resolved_locale(&self) -> Option<String> {
        self.app
            .try_state::<crate::app::state::AppState>()
            .map(|state| state.resolved_locale())
    }

    fn apply_clipboard_window_mode(
        &self,
        compact: bool,
        source: &str,
    ) -> AppResult<rtool_contracts::models::ClipboardWindowModeAppliedDto> {
        crate::platform::native_ui::clipboard_window::apply_clipboard_window_mode(
            &self.app, compact, source,
        )
    }
}

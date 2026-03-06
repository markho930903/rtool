use rtool_contracts::AppResult;
use rtool_contracts::models::ClipboardWindowModeAppliedDto;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPackageInfo {
    pub name: String,
    pub version: String,
}

pub trait LauncherWindow: Send + Sync {
    fn show(&self) -> AppResult<()>;
    fn set_focus(&self) -> AppResult<()>;
    fn set_position(&self, x: f64, y: f64) -> AppResult<()>;
    fn set_size(&self, width: f64, height: f64) -> AppResult<()>;
}

pub trait LauncherHost: Send + Sync {
    fn emit(&self, event: &str, payload: serde_json::Value) -> AppResult<()>;
    fn get_webview_window(&self, label: &str) -> Option<Box<dyn LauncherWindow>>;
    fn open_path(&self, path: &Path) -> AppResult<()>;
    fn app_data_dir(&self) -> AppResult<PathBuf>;
    fn package_info(&self) -> AppPackageInfo;
    fn resolved_locale(&self) -> Option<String>;
    fn apply_clipboard_window_mode(
        &self,
        compact: bool,
        source: &str,
    ) -> AppResult<ClipboardWindowModeAppliedDto>;
}

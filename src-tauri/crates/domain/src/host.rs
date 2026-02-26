use foundation::AppResult;
use foundation::models::ClipboardWindowModeAppliedDto;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppPackageInfo {
    pub name: String,
    pub version: String,
}

pub trait LauncherWindow: Send + Sync {
    fn show(&self) -> AppResult<()>;
    fn set_focus(&self) -> AppResult<()>;
}

pub trait LauncherHost: Send + Sync {
    fn emit(&self, event: &str, payload: Value) -> AppResult<()>;
    fn get_webview_window(&self, label: &str) -> Option<Box<dyn LauncherWindow>>;
    fn app_data_dir(&self) -> AppResult<PathBuf>;
    fn package_info(&self) -> AppPackageInfo;
    fn resolved_locale(&self) -> Option<String>;
    fn apply_clipboard_window_mode(
        &self,
        compact: bool,
        source: &str,
    ) -> AppResult<ClipboardWindowModeAppliedDto>;
}

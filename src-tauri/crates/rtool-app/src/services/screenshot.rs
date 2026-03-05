use rtool_capture::ScreenshotCommitImage;
use rtool_contracts::AppResult;
use rtool_contracts::models::{
    ScreenshotCommitInputDto, ScreenshotSessionDto, SettingsScreenshotDto,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, Default)]
pub struct ScreenshotApplicationService;

impl ScreenshotApplicationService {
    pub const PIN_MAX_INSTANCES_MIN: u32 = rtool_capture::SCREENSHOT_PIN_MAX_INSTANCES_MIN;
    pub const PIN_MAX_INSTANCES_MAX: u32 = rtool_capture::SCREENSHOT_PIN_MAX_INSTANCES_MAX;

    pub fn start_session(
        self,
        requested_display_id: Option<String>,
    ) -> AppResult<ScreenshotSessionDto> {
        rtool_capture::start_session(requested_display_id)
    }

    pub fn cancel_session(self, session_id: &str) -> AppResult<bool> {
        rtool_capture::cancel_session(session_id)
    }

    pub fn sweep_expired_sessions_now(self) -> usize {
        rtool_capture::sweep_expired_sessions_now()
    }

    pub fn save_png_file_for_session(
        self,
        app_data_dir: &Path,
        session_id: &str,
        png: &[u8],
    ) -> AppResult<String> {
        rtool_capture::save_png_file_for_session(app_data_dir, session_id, png)
    }

    pub fn cleanup_saved_archive(
        self,
        app_data_dir: &Path,
        max_items: u32,
        max_total_size_mb: u32,
    ) {
        rtool_capture::cleanup_saved_archive(app_data_dir, max_items, max_total_size_mb);
    }

    pub fn commit_selection(
        self,
        input: ScreenshotCommitInputDto,
        app_data_dir: &Path,
        settings: &SettingsScreenshotDto,
    ) -> AppResult<ScreenshotCommitImage> {
        rtool_capture::commit_selection(input, app_data_dir, settings)
    }
}

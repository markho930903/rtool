use rtool_capture::helpers::{build_image_clipboard_item, parse_file_paths_from_text};
use rtool_capture::service::{
    ClipboardSaveResult, ClipboardService, ClipboardSettingsUpdateResult,
};
use rtool_contracts::models::{ClipboardFilterDto, ClipboardItemDto, SettingsClipboardDto};
use rtool_contracts::{AppError, AppResult};
use rtool_data::db::{self, DbConn};

#[derive(Clone)]
pub struct ClipboardApplicationService {
    db_conn: DbConn,
    service: ClipboardService,
}

impl ClipboardApplicationService {
    pub fn new(db_conn: DbConn, service: ClipboardService) -> Self {
        Self { db_conn, service }
    }

    pub fn ensure_disk_space_for_new_item(&self) -> AppResult<()> {
        self.service.ensure_disk_space_for_new_item()
    }

    pub fn parse_file_paths_from_plain_text(plain_text: &str) -> AppResult<Vec<String>> {
        parse_file_paths_from_text(plain_text).ok_or_else(|| {
            AppError::new(
                "clipboard_file_payload_invalid",
                "文件条目路径数据无效或目标文件不存在",
            )
        })
    }

    pub async fn list(&self, filter: ClipboardFilterDto) -> AppResult<Vec<ClipboardItemDto>> {
        self.service.list(filter).await
    }

    pub async fn pin(&self, id: String, pinned: bool) -> AppResult<ClipboardItemDto> {
        self.service.pin(id, pinned).await
    }

    pub async fn delete(&self, id: String) -> AppResult<()> {
        self.service.delete(id).await
    }

    pub async fn clear_all(&self) -> AppResult<()> {
        self.service.clear_all().await
    }

    pub async fn save_text(
        &self,
        text: String,
        source_app: Option<String>,
    ) -> AppResult<ClipboardSaveResult> {
        self.service.save_text(text, source_app).await
    }

    pub async fn save_watcher_image(
        &self,
        width: usize,
        height: usize,
        signature: &str,
        preview_path: Option<String>,
        source_app: Option<String>,
    ) -> AppResult<ClipboardSaveResult> {
        let item =
            build_image_clipboard_item(width, height, signature, preview_path, None, source_app);
        self.service.save_item(item).await
    }

    pub async fn touch_item(&self, id: String) -> AppResult<ClipboardItemDto> {
        self.service.touch_item(id).await
    }

    pub async fn get_item_or_not_found(&self, query_id: String) -> AppResult<ClipboardItemDto> {
        let item = db::get_clipboard_item(&self.db_conn, query_id.as_str()).await?;
        item.ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))
    }

    pub async fn apply_settings(
        &self,
        settings: &SettingsClipboardDto,
    ) -> AppResult<ClipboardSettingsUpdateResult> {
        self.service.apply_settings(settings).await
    }
}

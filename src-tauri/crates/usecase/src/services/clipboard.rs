use domain::service::{
    ClipboardSaveResult, ClipboardService, ClipboardSettingsUpdateResult,
};
use foundation::models::{ClipboardFilterDto, ClipboardItemDto, UserClipboardSettingsDto};
use foundation::{AppError, AppResult};
use foundation::db::{self, DbConn};

#[derive(Clone)]
pub struct ClipboardApplicationService {
    db_conn: DbConn,
    service: ClipboardService,
}

impl ClipboardApplicationService {
    pub fn new(db_conn: DbConn, service: ClipboardService) -> Self {
        Self { db_conn, service }
    }

    pub fn domain_service(&self) -> &ClipboardService {
        &self.service
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

    pub async fn touch_item(&self, id: String) -> AppResult<ClipboardItemDto> {
        self.service.touch_item(id).await
    }

    pub async fn get_item_or_not_found(&self, query_id: String) -> AppResult<ClipboardItemDto> {
        let item = db::get_clipboard_item(&self.db_conn, query_id.as_str()).await?;
        item.ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))
    }

    pub async fn apply_user_settings(
        &self,
        settings: &UserClipboardSettingsDto,
    ) -> AppResult<ClipboardSettingsUpdateResult> {
        self.service.apply_user_settings(settings).await
    }
}

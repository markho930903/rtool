use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::models::{ClipboardFilterDto, ClipboardItemDto, ClipboardSettingsDto};
use crate::core::{AppError, AppResult};
use crate::infrastructure::clipboard::build_clipboard_item;
use crate::infrastructure::db::{self, DbPool};

pub const CLIPBOARD_MAX_ITEMS_DEFAULT: u32 = 1000;
pub const CLIPBOARD_MAX_ITEMS_MIN: u32 = 100;
pub const CLIPBOARD_MAX_ITEMS_MAX: u32 = 10_000;

fn validate_max_items(max_items: u32) -> AppResult<u32> {
    if (CLIPBOARD_MAX_ITEMS_MIN..=CLIPBOARD_MAX_ITEMS_MAX).contains(&max_items) {
        return Ok(max_items);
    }

    Err(AppError::new(
        "clipboard_max_items_out_of_range",
        format!(
            "剪贴板条目上限必须在 {} 到 {} 之间",
            CLIPBOARD_MAX_ITEMS_MIN, CLIPBOARD_MAX_ITEMS_MAX
        ),
    ))
}

fn remove_preview_file(path: &str) {
    if path.trim().is_empty() {
        return;
    }

    if let Err(error) = std::fs::remove_file(path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(
                event = "clipboard_preview_delete_failed",
                preview_path = path,
                error = error.to_string()
            );
        }
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default()
}

#[derive(Clone)]
pub struct ClipboardService {
    db_pool: DbPool,
    max_items: Arc<RwLock<u32>>,
}

#[derive(Debug, Clone)]
pub struct ClipboardSaveResult {
    pub item: ClipboardItemDto,
    pub removed_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ClipboardSettingsUpdateResult {
    pub settings: ClipboardSettingsDto,
    pub removed_ids: Vec<String>,
}

impl ClipboardService {
    pub fn new(db_pool: DbPool) -> AppResult<Self> {
        let stored = db::get_clipboard_max_items(&db_pool)?.unwrap_or(CLIPBOARD_MAX_ITEMS_DEFAULT);
        let normalized = stored.clamp(CLIPBOARD_MAX_ITEMS_MIN, CLIPBOARD_MAX_ITEMS_MAX);
        db::set_clipboard_max_items(&db_pool, normalized)?;

        let service = Self {
            db_pool,
            max_items: Arc::new(RwLock::new(normalized)),
        };
        let _ = service.enforce_capacity()?;
        Ok(service)
    }

    fn current_max_items(&self) -> u32 {
        self.max_items
            .read()
            .map(|value| *value)
            .unwrap_or(CLIPBOARD_MAX_ITEMS_DEFAULT)
    }

    fn set_cached_max_items(&self, value: u32) -> AppResult<()> {
        let mut guard = self
            .max_items
            .write()
            .map_err(|_| AppError::new("clipboard_settings_lock_failed", "更新剪贴板设置失败"))?;
        *guard = value;
        Ok(())
    }

    fn enforce_capacity(&self) -> AppResult<Vec<String>> {
        let removed_items = db::prune_clipboard_items(&self.db_pool, self.current_max_items())?;
        let mut removed_ids = Vec::with_capacity(removed_items.len());
        for removed in removed_items {
            removed_ids.push(removed.id);
            if let Some(preview_path) = removed.preview_path {
                remove_preview_file(&preview_path);
            }
        }
        Ok(removed_ids)
    }

    pub fn save_text(
        &self,
        text: String,
        source_app: Option<String>,
    ) -> AppResult<ClipboardSaveResult> {
        let item = build_clipboard_item(text, source_app);
        let stored = db::insert_clipboard_item(&self.db_pool, &item)?;
        let removed_ids = self.enforce_capacity()?;
        Ok(ClipboardSaveResult {
            item: stored,
            removed_ids,
        })
    }

    pub fn save_item(&self, item: ClipboardItemDto) -> AppResult<ClipboardSaveResult> {
        let stored = db::insert_clipboard_item(&self.db_pool, &item)?;
        let removed_ids = self.enforce_capacity()?;
        Ok(ClipboardSaveResult {
            item: stored,
            removed_ids,
        })
    }

    pub fn list(&self, filter: ClipboardFilterDto) -> AppResult<Vec<ClipboardItemDto>> {
        db::list_clipboard_items(&self.db_pool, &filter)
    }

    pub fn pin(&self, id: String, pinned: bool) -> AppResult<ClipboardItemDto> {
        db::pin_clipboard_item(&self.db_pool, &id, pinned)?;
        db::get_clipboard_item(&self.db_pool, &id)?
            .ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))
    }

    pub fn touch_item(&self, id: String) -> AppResult<ClipboardItemDto> {
        let created_at = now_millis();
        db::touch_clipboard_item(&self.db_pool, &id, created_at)?
            .ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))
    }

    pub fn delete(&self, id: String) -> AppResult<()> {
        if let Some(preview_path) = db::delete_clipboard_item(&self.db_pool, &id)? {
            remove_preview_file(&preview_path);
        }
        Ok(())
    }

    pub fn clear_all(&self) -> AppResult<()> {
        let removed_paths = db::clear_all_clipboard_items(&self.db_pool)?;
        for preview_path in removed_paths {
            remove_preview_file(&preview_path);
        }
        Ok(())
    }

    pub fn get_settings(&self) -> ClipboardSettingsDto {
        ClipboardSettingsDto {
            max_items: self.current_max_items(),
        }
    }

    pub fn update_settings(&self, max_items: u32) -> AppResult<ClipboardSettingsUpdateResult> {
        let max_items = validate_max_items(max_items)?;
        db::set_clipboard_max_items(&self.db_pool, max_items)?;
        self.set_cached_max_items(max_items)?;
        let removed_ids = self.enforce_capacity()?;
        Ok(ClipboardSettingsUpdateResult {
            settings: ClipboardSettingsDto { max_items },
            removed_ids,
        })
    }
}

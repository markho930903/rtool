use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::models::{ClipboardFilterDto, ClipboardItemDto, ClipboardSettingsDto};
use crate::core::{AppError, AppResult};
use crate::infrastructure::clipboard::build_clipboard_item;
use crate::infrastructure::db::{self, DbPool};
use sysinfo::Disks;

pub const CLIPBOARD_MAX_ITEMS_DEFAULT: u32 = 1000;
pub const CLIPBOARD_MAX_ITEMS_MIN: u32 = 100;
pub const CLIPBOARD_MAX_ITEMS_MAX: u32 = 10_000;
pub const CLIPBOARD_SIZE_CLEANUP_ENABLED_DEFAULT: bool = true;
pub const CLIPBOARD_MAX_TOTAL_SIZE_MB_DEFAULT: u32 = 500;
pub const CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN: u32 = 100;
pub const CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX: u32 = 10_240;
pub const CLIPBOARD_MIN_FREE_DISK_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
struct ClipboardRuntimeSettings {
    max_items: u32,
    size_cleanup_enabled: bool,
    max_total_size_mb: u32,
}

impl Default for ClipboardRuntimeSettings {
    fn default() -> Self {
        Self {
            max_items: CLIPBOARD_MAX_ITEMS_DEFAULT,
            size_cleanup_enabled: CLIPBOARD_SIZE_CLEANUP_ENABLED_DEFAULT,
            max_total_size_mb: CLIPBOARD_MAX_TOTAL_SIZE_MB_DEFAULT,
        }
    }
}

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

fn validate_max_total_size_mb(max_total_size_mb: u32) -> AppResult<u32> {
    if (CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN..=CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX)
        .contains(&max_total_size_mb)
    {
        return Ok(max_total_size_mb);
    }

    Err(AppError::new(
        "clipboard_max_total_size_out_of_range",
        format!(
            "剪贴板体积上限必须在 {} 到 {} MB 之间",
            CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN, CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX
        ),
    ))
}

fn resolve_available_space_bytes(path: &Path) -> Option<u64> {
    let disks = Disks::new_with_refreshed_list();
    let mut best_match: Option<(usize, u64)> = None;

    for disk in &disks {
        let mount_point = disk.mount_point();
        if !path.starts_with(mount_point) {
            continue;
        }

        let mount_depth = mount_point.components().count();
        match best_match {
            Some((depth, _)) if depth >= mount_depth => {}
            _ => {
                best_match = Some((mount_depth, disk.available_space()));
            }
        }
    }

    best_match.map(|(_, available)| available)
}

fn ensure_available_space(
    available_space_bytes: Option<u64>,
    min_required_bytes: u64,
) -> AppResult<()> {
    let Some(available) = available_space_bytes else {
        return Ok(());
    };

    if available >= min_required_bytes {
        return Ok(());
    }

    let min_required_mb = min_required_bytes / (1024 * 1024);
    let available_mb = available / (1024 * 1024);
    Err(AppError::new(
        "clipboard_disk_space_low",
        format!("磁盘可用空间不足，至少需要保留 {min_required_mb} MB"),
    )
    .with_detail(format!(
        "available_mb={available_mb}, required_mb={min_required_mb}"
    )))
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
    db_path: PathBuf,
    settings: Arc<RwLock<ClipboardRuntimeSettings>>,
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
    pub fn new(db_pool: DbPool, db_path: PathBuf) -> AppResult<Self> {
        let stored = db::get_clipboard_max_items(&db_pool)?.unwrap_or(CLIPBOARD_MAX_ITEMS_DEFAULT);
        let max_items = stored.clamp(CLIPBOARD_MAX_ITEMS_MIN, CLIPBOARD_MAX_ITEMS_MAX);
        let size_cleanup_enabled = db::get_clipboard_size_cleanup_enabled(&db_pool)?
            .unwrap_or(CLIPBOARD_SIZE_CLEANUP_ENABLED_DEFAULT);
        let stored_max_total_size_mb = db::get_clipboard_max_total_size_mb(&db_pool)?
            .unwrap_or(CLIPBOARD_MAX_TOTAL_SIZE_MB_DEFAULT);
        let max_total_size_mb = stored_max_total_size_mb.clamp(
            CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN,
            CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX,
        );

        db::set_clipboard_max_items(&db_pool, max_items)?;
        db::set_clipboard_size_cleanup_enabled(&db_pool, size_cleanup_enabled)?;
        db::set_clipboard_max_total_size_mb(&db_pool, max_total_size_mb)?;

        let service = Self {
            db_pool,
            db_path,
            settings: Arc::new(RwLock::new(ClipboardRuntimeSettings {
                max_items,
                size_cleanup_enabled,
                max_total_size_mb,
            })),
        };
        let _ = service.enforce_capacity()?;
        Ok(service)
    }

    fn current_settings(&self) -> ClipboardRuntimeSettings {
        self.settings
            .read()
            .map(|value| value.clone())
            .unwrap_or_else(|_| ClipboardRuntimeSettings::default())
    }

    fn set_cached_settings(&self, value: ClipboardRuntimeSettings) -> AppResult<()> {
        let mut guard = self
            .settings
            .write()
            .map_err(|_| AppError::new("clipboard_settings_lock_failed", "更新剪贴板设置失败"))?;
        *guard = value;
        Ok(())
    }

    pub fn ensure_disk_space_for_new_item(&self) -> AppResult<()> {
        let available = resolve_available_space_bytes(self.db_path.as_path());
        ensure_available_space(available, CLIPBOARD_MIN_FREE_DISK_BYTES)
    }

    fn enforce_capacity(&self) -> AppResult<Vec<String>> {
        let settings = self.current_settings();
        let size_limit = if settings.size_cleanup_enabled {
            Some(u64::from(settings.max_total_size_mb).saturating_mul(1024 * 1024))
        } else {
            None
        };
        let removed_items =
            db::prune_clipboard_items(&self.db_pool, settings.max_items, size_limit)?;
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
        self.ensure_disk_space_for_new_item()?;
        let item = build_clipboard_item(text, source_app);
        let stored = db::insert_clipboard_item(&self.db_pool, &item)?;
        let removed_ids = self.enforce_capacity()?;
        Ok(ClipboardSaveResult {
            item: stored,
            removed_ids,
        })
    }

    pub fn save_item(&self, item: ClipboardItemDto) -> AppResult<ClipboardSaveResult> {
        self.ensure_disk_space_for_new_item()?;
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
        let settings = self.current_settings();
        ClipboardSettingsDto {
            max_items: settings.max_items,
            size_cleanup_enabled: settings.size_cleanup_enabled,
            max_total_size_mb: settings.max_total_size_mb,
        }
    }

    pub fn update_settings(
        &self,
        max_items: u32,
        size_cleanup_enabled: Option<bool>,
        max_total_size_mb: Option<u32>,
    ) -> AppResult<ClipboardSettingsUpdateResult> {
        let current = self.current_settings();
        let max_items = validate_max_items(max_items)?;
        let size_cleanup_enabled = size_cleanup_enabled.unwrap_or(current.size_cleanup_enabled);
        let max_total_size_mb =
            validate_max_total_size_mb(max_total_size_mb.unwrap_or(current.max_total_size_mb))?;

        db::set_clipboard_max_items(&self.db_pool, max_items)?;
        db::set_clipboard_size_cleanup_enabled(&self.db_pool, size_cleanup_enabled)?;
        db::set_clipboard_max_total_size_mb(&self.db_pool, max_total_size_mb)?;
        self.set_cached_settings(ClipboardRuntimeSettings {
            max_items,
            size_cleanup_enabled,
            max_total_size_mb,
        })?;
        let removed_ids = self.enforce_capacity()?;
        Ok(ClipboardSettingsUpdateResult {
            settings: ClipboardSettingsDto {
                max_items,
                size_cleanup_enabled,
                max_total_size_mb,
            },
            removed_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db;

    fn unique_temp_db_path(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_millis();
        std::env::temp_dir().join(format!("rtool-{prefix}-{}-{now}.db", std::process::id()))
    }

    #[test]
    fn should_load_default_clipboard_settings() {
        let db_path = unique_temp_db_path("clipboard-settings-default");
        db::init_db(db_path.as_path()).expect("init db");
        let db_pool = db::new_db_pool(db_path.as_path()).expect("new db pool");
        let service =
            ClipboardService::new(db_pool, db_path.clone()).expect("new clipboard service");

        let settings = service.get_settings();
        assert_eq!(settings.max_items, CLIPBOARD_MAX_ITEMS_DEFAULT);
        assert!(settings.size_cleanup_enabled);
        assert_eq!(
            settings.max_total_size_mb,
            CLIPBOARD_MAX_TOTAL_SIZE_MB_DEFAULT
        );

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn should_allow_when_disk_space_metric_is_missing() {
        let result = ensure_available_space(None, CLIPBOARD_MIN_FREE_DISK_BYTES);
        assert!(result.is_ok());
    }

    #[test]
    fn should_allow_when_disk_space_is_enough() {
        let result = ensure_available_space(
            Some(CLIPBOARD_MIN_FREE_DISK_BYTES),
            CLIPBOARD_MIN_FREE_DISK_BYTES,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn should_reject_when_disk_space_is_low() {
        let result = ensure_available_space(
            Some(CLIPBOARD_MIN_FREE_DISK_BYTES - 1),
            CLIPBOARD_MIN_FREE_DISK_BYTES,
        );
        assert!(result.is_err());
        assert_eq!(
            result.expect_err("expected low disk error").code,
            "clipboard_disk_space_low"
        );
    }
}

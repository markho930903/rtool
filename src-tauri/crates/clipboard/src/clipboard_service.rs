use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use kernel::clipboard::build_clipboard_item;
use protocol::models::{
    ClipboardFilterDto, ClipboardItemDto, ClipboardSettingsDto, UserClipboardSettingsDto,
};
use protocol::{AppError, AppResult, ResultExt};
use rtool_db::db::{self, DbConn};
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

impl ClipboardRuntimeSettings {
    fn from_user_settings(value: &UserClipboardSettingsDto) -> Self {
        Self {
            max_items: value
                .max_items
                .clamp(CLIPBOARD_MAX_ITEMS_MIN, CLIPBOARD_MAX_ITEMS_MAX),
            size_cleanup_enabled: value.size_cleanup_enabled,
            max_total_size_mb: value.max_total_size_mb.clamp(
                CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN,
                CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX,
            ),
        }
    }
}

fn validate_max_items(max_items: u32) -> AppResult<u32> {
    (|| -> anyhow::Result<u32> {
        anyhow::ensure!(
            (CLIPBOARD_MAX_ITEMS_MIN..=CLIPBOARD_MAX_ITEMS_MAX).contains(&max_items),
            "max_items={max_items}, expected=[{}, {}]",
            CLIPBOARD_MAX_ITEMS_MIN,
            CLIPBOARD_MAX_ITEMS_MAX
        );
        Ok(max_items)
    })()
    .with_code(
        "clipboard_max_items_out_of_range",
        format!(
            "剪贴板条目上限必须在 {} 到 {} 之间",
            CLIPBOARD_MAX_ITEMS_MIN, CLIPBOARD_MAX_ITEMS_MAX
        ),
    )
}

fn validate_max_total_size_mb(max_total_size_mb: u32) -> AppResult<u32> {
    (|| -> anyhow::Result<u32> {
        anyhow::ensure!(
            (CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN..=CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX)
                .contains(&max_total_size_mb),
            "max_total_size_mb={max_total_size_mb}, expected=[{}, {}]",
            CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN,
            CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX
        );
        Ok(max_total_size_mb)
    })()
    .with_code(
        "clipboard_max_total_size_out_of_range",
        format!(
            "剪贴板体积上限必须在 {} 到 {} MB 之间",
            CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN, CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX
        ),
    )
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

    let min_required_mb = min_required_bytes / (1024 * 1024);
    let available_mb = available / (1024 * 1024);
    (|| -> anyhow::Result<()> {
        anyhow::ensure!(
            available >= min_required_bytes,
            "available_mb={available_mb}, required_mb={min_required_mb}"
        );
        Ok(())
    })()
    .with_code(
        "clipboard_disk_space_low",
        format!("磁盘可用空间不足，至少需要保留 {min_required_mb} MB"),
    )
    .map_err(|error| {
        error
            .with_context("availableMb", available_mb.to_string())
            .with_context("requiredMb", min_required_mb.to_string())
    })
}

fn remove_preview_file(path: &str) {
    if path.trim().is_empty() {
        return;
    }

    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(
            event = "clipboard_preview_delete_failed",
            preview_path = path,
            error = error.to_string()
        );
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
    db_conn: DbConn,
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
    pub async fn new(
        db_conn: DbConn,
        db_path: PathBuf,
        initial_settings: UserClipboardSettingsDto,
    ) -> AppResult<Self> {
        let runtime_settings = ClipboardRuntimeSettings::from_user_settings(&initial_settings);

        let service = Self {
            db_conn,
            db_path,
            settings: Arc::new(RwLock::new(runtime_settings)),
        };
        let _ = service.enforce_capacity().await?;
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

    async fn enforce_capacity(&self) -> AppResult<Vec<String>> {
        let settings = self.current_settings();
        let size_limit = if settings.size_cleanup_enabled {
            Some(u64::from(settings.max_total_size_mb).saturating_mul(1024 * 1024))
        } else {
            None
        };
        let removed_items =
            db::prune_clipboard_items(&self.db_conn, settings.max_items, size_limit).await?;
        let mut removed_ids = Vec::with_capacity(removed_items.len());
        for removed in removed_items {
            removed_ids.push(removed.id);
            if let Some(preview_path) = removed.preview_path {
                remove_preview_file(&preview_path);
            }
        }
        Ok(removed_ids)
    }

    pub async fn save_text(
        &self,
        text: String,
        source_app: Option<String>,
    ) -> AppResult<ClipboardSaveResult> {
        self.ensure_disk_space_for_new_item()?;
        let item = build_clipboard_item(text, source_app);
        let stored = db::insert_clipboard_item(&self.db_conn, &item).await?;
        let removed_ids = self.enforce_capacity().await?;
        Ok(ClipboardSaveResult {
            item: stored,
            removed_ids,
        })
    }

    pub async fn save_item(&self, item: ClipboardItemDto) -> AppResult<ClipboardSaveResult> {
        self.ensure_disk_space_for_new_item()?;
        let stored = db::insert_clipboard_item(&self.db_conn, &item).await?;
        let removed_ids = self.enforce_capacity().await?;
        Ok(ClipboardSaveResult {
            item: stored,
            removed_ids,
        })
    }

    pub async fn list(&self, filter: ClipboardFilterDto) -> AppResult<Vec<ClipboardItemDto>> {
        db::list_clipboard_items(&self.db_conn, &filter)
            .await
            .map_err(AppError::from)
    }

    pub async fn pin(&self, id: String, pinned: bool) -> AppResult<ClipboardItemDto> {
        db::pin_clipboard_item(&self.db_conn, &id, pinned).await?;
        db::get_clipboard_item(&self.db_conn, &id)
            .await?
            .ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))
    }

    pub async fn touch_item(&self, id: String) -> AppResult<ClipboardItemDto> {
        let created_at = now_millis();
        db::touch_clipboard_item(&self.db_conn, &id, created_at)
            .await?
            .ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))
    }

    pub async fn delete(&self, id: String) -> AppResult<()> {
        if let Some(preview_path) = db::delete_clipboard_item(&self.db_conn, &id).await? {
            remove_preview_file(&preview_path);
        }
        Ok(())
    }

    pub async fn clear_all(&self) -> AppResult<()> {
        let removed_paths = db::clear_all_clipboard_items(&self.db_conn).await?;
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

    pub async fn update_settings(
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

        self.set_cached_settings(ClipboardRuntimeSettings {
            max_items,
            size_cleanup_enabled,
            max_total_size_mb,
        })?;
        let removed_ids = self.enforce_capacity().await?;
        Ok(ClipboardSettingsUpdateResult {
            settings: ClipboardSettingsDto {
                max_items,
                size_cleanup_enabled,
                max_total_size_mb,
            },
            removed_ids,
        })
    }

    pub async fn apply_user_settings(
        &self,
        settings: &UserClipboardSettingsDto,
    ) -> AppResult<ClipboardSettingsUpdateResult> {
        let normalized = ClipboardRuntimeSettings::from_user_settings(settings);
        let current = self.current_settings();
        if current.max_items == normalized.max_items
            && current.size_cleanup_enabled == normalized.size_cleanup_enabled
            && current.max_total_size_mb == normalized.max_total_size_mb
        {
            return Ok(ClipboardSettingsUpdateResult {
                settings: ClipboardSettingsDto {
                    max_items: current.max_items,
                    size_cleanup_enabled: current.size_cleanup_enabled,
                    max_total_size_mb: current.max_total_size_mb,
                },
                removed_ids: Vec::new(),
            });
        }

        self.set_cached_settings(normalized.clone())?;
        let removed_ids = self.enforce_capacity().await?;
        Ok(ClipboardSettingsUpdateResult {
            settings: ClipboardSettingsDto {
                max_items: normalized.max_items,
                size_cleanup_enabled: normalized.size_cleanup_enabled,
                max_total_size_mb: normalized.max_total_size_mb,
            },
            removed_ids,
        })
    }
}


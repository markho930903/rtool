use super::{command_end_error, command_end_ok, command_start, normalize_request_id};
use crate::app::state::AppState;
use crate::core::models::{
    ClipboardFilterDto, ClipboardItemDto, ClipboardSettingsDto, ClipboardSyncPayload,
    ClipboardWindowModeAppliedDto,
};
use crate::core::{AppError, AppResult};
use crate::infrastructure::db;
use arboard::{Clipboard as ArboardClipboard, ImageData};
use base64::Engine as _;
use image::ImageReader;
use std::borrow::Cow;
use tauri::{AppHandle, Emitter, State};

const CLIPBOARD_SYNC_EVENT: &str = "rtool://clipboard/sync";

fn default_filter() -> ClipboardFilterDto {
    ClipboardFilterDto {
        query: None,
        item_type: None,
        only_pinned: Some(false),
        limit: Some(100),
    }
}

fn decode_data_url_image_bytes(data_url: &str) -> AppResult<Vec<u8>> {
    let encoded = data_url
        .split_once(",")
        .map(|(_, value)| value)
        .unwrap_or(data_url)
        .trim();

    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| {
            AppError::new("image_data_url_decode_failed", "解析图片数据失败")
                .with_detail(error.to_string())
        })
}

fn parse_file_paths_from_plain_text(plain_text: &str) -> AppResult<Vec<String>> {
    crate::infrastructure::clipboard::parse_file_paths_from_text(plain_text).ok_or_else(|| {
        AppError::new(
            "clipboard_file_payload_invalid",
            "文件条目路径数据无效或目标文件不存在",
        )
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn to_clipboard_files_uris(file_paths: &[String]) -> Vec<String> {
    file_paths
        .iter()
        .map(|path| format!("file://{path}"))
        .collect()
}

#[cfg(target_os = "windows")]
fn to_clipboard_files_uris(file_paths: &[String]) -> Vec<String> {
    file_paths.to_vec()
}

fn emit_clipboard_sync(app: &AppHandle, payload: ClipboardSyncPayload) {
    if let Err(error) = app.emit(CLIPBOARD_SYNC_EVENT, payload) {
        tracing::warn!(
            event = "clipboard_event_emit_failed",
            event_name = CLIPBOARD_SYNC_EVENT,
            error = error.to_string()
        );
    }
}

#[tauri::command]
pub fn clipboard_list(
    state: State<'_, AppState>,
    filter: Option<ClipboardFilterDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<ClipboardItemDto>, AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("clipboard_list", &request_id, window_label.as_deref());
    let result = state
        .clipboard_service
        .list(filter.unwrap_or_else(default_filter));
    match &result {
        Ok(_) => command_end_ok("clipboard_list", &request_id, started_at),
        Err(error) => command_end_error("clipboard_list", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub fn clipboard_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<()> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("clipboard_pin", &request_id, window_label.as_deref());
    let result = (|| -> AppResult<()> {
        let updated = state.clipboard_service.pin(id, pinned)?;
        emit_clipboard_sync(
            &app,
            ClipboardSyncPayload {
                upsert: vec![updated],
                removed_ids: Vec::new(),
                clear_all: false,
                reason: Some("pin".to_string()),
            },
        );
        Ok(())
    })();
    match &result {
        Ok(_) => command_end_ok("clipboard_pin", &request_id, started_at),
        Err(error) => command_end_error("clipboard_pin", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub fn clipboard_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<()> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("clipboard_delete", &request_id, window_label.as_deref());
    let result = (|| -> AppResult<()> {
        state.clipboard_service.delete(id.clone())?;
        emit_clipboard_sync(
            &app,
            ClipboardSyncPayload {
                upsert: Vec::new(),
                removed_ids: vec![id],
                clear_all: false,
                reason: Some("delete".to_string()),
            },
        );
        Ok(())
    })();
    match &result {
        Ok(_) => command_end_ok("clipboard_delete", &request_id, started_at),
        Err(error) => command_end_error("clipboard_delete", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub fn clipboard_clear_all(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<()> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("clipboard_clear_all", &request_id, window_label.as_deref());
    let result = (|| -> AppResult<()> {
        state.clipboard_service.clear_all()?;
        emit_clipboard_sync(
            &app,
            ClipboardSyncPayload {
                upsert: Vec::new(),
                removed_ids: Vec::new(),
                clear_all: true,
                reason: Some("clear_all".to_string()),
            },
        );
        Ok(())
    })();
    match &result {
        Ok(_) => command_end_ok("clipboard_clear_all", &request_id, started_at),
        Err(error) => command_end_error("clipboard_clear_all", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub fn clipboard_save_text(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<ClipboardItemDto> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("clipboard_save_text", &request_id, window_label.as_deref());
    let result = (|| -> AppResult<ClipboardItemDto> {
        let saved = state.clipboard_service.save_text(text, None)?;
        emit_clipboard_sync(
            &app,
            ClipboardSyncPayload {
                upsert: vec![saved.item.clone()],
                removed_ids: saved.removed_ids,
                clear_all: false,
                reason: Some("save_text".to_string()),
            },
        );
        Ok(saved.item)
    })();
    match &result {
        Ok(_) => command_end_ok("clipboard_save_text", &request_id, started_at),
        Err(error) => command_end_error("clipboard_save_text", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub fn clipboard_get_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ClipboardSettingsDto, AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "clipboard_get_settings",
        &request_id,
        window_label.as_deref(),
    );
    let result = Ok(state.clipboard_service.get_settings());
    command_end_ok("clipboard_get_settings", &request_id, started_at);
    result
}

#[tauri::command]
pub fn clipboard_update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    max_items: u32,
    size_cleanup_enabled: Option<bool>,
    max_total_size_mb: Option<u32>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ClipboardSettingsDto, AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "clipboard_update_settings",
        &request_id,
        window_label.as_deref(),
    );
    let result = (|| -> Result<ClipboardSettingsDto, AppError> {
        let updated = state.clipboard_service.update_settings(
            max_items,
            size_cleanup_enabled,
            max_total_size_mb,
        )?;
        if !updated.removed_ids.is_empty() {
            emit_clipboard_sync(
                &app,
                ClipboardSyncPayload {
                    upsert: Vec::new(),
                    removed_ids: updated.removed_ids.clone(),
                    clear_all: false,
                    reason: Some("update_settings_prune".to_string()),
                },
            );
        }
        Ok(updated.settings)
    })();
    match &result {
        Ok(_) => command_end_ok("clipboard_update_settings", &request_id, started_at),
        Err(error) => {
            command_end_error("clipboard_update_settings", &request_id, started_at, error)
        }
    }
    result
}

#[tauri::command]
pub fn clipboard_window_set_mode(
    state: State<'_, AppState>,
    compact: bool,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<()> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "clipboard_window_set_mode",
        &request_id,
        window_label.as_deref(),
    );
    let result = (|| -> AppResult<()> {
        state.set_clipboard_window_compact(compact);
        Ok(())
    })();
    match &result {
        Ok(_) => command_end_ok("clipboard_window_set_mode", &request_id, started_at),
        Err(error) => {
            command_end_error("clipboard_window_set_mode", &request_id, started_at, error)
        }
    }
    result
}

#[tauri::command]
pub fn clipboard_window_apply_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    compact: bool,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ClipboardWindowModeAppliedDto, AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "clipboard_window_apply_mode",
        &request_id,
        window_label.as_deref(),
    );
    let result = (|| -> Result<ClipboardWindowModeAppliedDto, AppError> {
        let applied =
            crate::apply_clipboard_window_mode(&app, compact, "command").map_err(|detail| {
                AppError::new("clipboard_window_resize_failed", "设置剪贴板窗口尺寸失败")
                    .with_detail(detail)
            })?;
        state.set_clipboard_window_compact(compact);
        Ok(applied)
    })();
    match &result {
        Ok(_) => command_end_ok("clipboard_window_apply_mode", &request_id, started_at),
        Err(error) => command_end_error(
            "clipboard_window_apply_mode",
            &request_id,
            started_at,
            error,
        ),
    }
    result
}

#[tauri::command]
pub fn clipboard_copy_back(
    app: AppHandle,
    state: State<'_, AppState>,
    clipboard_plugin: State<'_, tauri_plugin_clipboard::Clipboard>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<()> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("clipboard_copy_back", &request_id, window_label.as_deref());

    let result = (|| -> AppResult<()> {
        let item = db::get_clipboard_item(&state.db_pool, &id)?
            .ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))?;

        if item.item_type == "file" {
            let file_paths = parse_file_paths_from_plain_text(&item.plain_text)?;
            let files_uris = to_clipboard_files_uris(&file_paths);
            clipboard_plugin
                .write_files_uris(files_uris)
                .map_err(|error| {
                    AppError::new("clipboard_set_files_failed", "写入文件到剪贴板失败")
                        .with_detail(error)
                })?;
        } else {
            let mut clipboard = ArboardClipboard::new()?;
            clipboard.set_text(item.plain_text)?;
        }

        let touched = state.clipboard_service.touch_item(id)?;
        emit_clipboard_sync(
            &app,
            ClipboardSyncPayload {
                upsert: vec![touched],
                removed_ids: Vec::new(),
                clear_all: false,
                reason: Some("copy_back".to_string()),
            },
        );
        Ok(())
    })();

    match &result {
        Ok(_) => command_end_ok("clipboard_copy_back", &request_id, started_at),
        Err(error) => command_end_error("clipboard_copy_back", &request_id, started_at, error),
    }
    result
}

#[tauri::command]
pub fn clipboard_copy_file_paths(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<()> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "clipboard_copy_file_paths",
        &request_id,
        window_label.as_deref(),
    );

    let result = (|| -> AppResult<()> {
        let item = db::get_clipboard_item(&state.db_pool, &id)?
            .ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))?;
        if item.item_type != "file" {
            return Err(AppError::new("clipboard_not_file", "当前条目不是文件类型"));
        }

        let mut clipboard = ArboardClipboard::new()?;
        clipboard.set_text(item.plain_text)?;

        let touched = state.clipboard_service.touch_item(id)?;
        emit_clipboard_sync(
            &app,
            ClipboardSyncPayload {
                upsert: vec![touched],
                removed_ids: Vec::new(),
                clear_all: false,
                reason: Some("copy_file_paths".to_string()),
            },
        );
        Ok(())
    })();

    match &result {
        Ok(_) => command_end_ok("clipboard_copy_file_paths", &request_id, started_at),
        Err(error) => {
            command_end_error("clipboard_copy_file_paths", &request_id, started_at, error)
        }
    }
    result
}

#[tauri::command]
pub fn clipboard_copy_image_back(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> AppResult<()> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "clipboard_copy_image_back",
        &request_id,
        window_label.as_deref(),
    );

    let result = (|| -> AppResult<()> {
        let item = db::get_clipboard_item(&state.db_pool, &id)?
            .ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))?;

        if item.item_type != "image" {
            return Err(AppError::new("clipboard_not_image", "当前条目不是图片类型"));
        }

        let image_from_path = item.preview_path.as_ref().and_then(|preview_path| {
            let reader = ImageReader::open(preview_path).ok()?;
            reader.decode().ok()
        });

        let image = if let Some(image) = image_from_path {
            image
        } else if let Some(preview_data_url) = &item.preview_data_url {
            let decoded_bytes = decode_data_url_image_bytes(preview_data_url)?;
            image::load_from_memory(&decoded_bytes).map_err(|error| {
                AppError::new("image_decode_failed", "解码图片失败").with_detail(error.to_string())
            })?
        } else {
            return Err(AppError::new("image_preview_missing", "图片预览数据不存在"));
        };

        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        let image_data = ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(rgba.into_raw()),
        };

        let mut clipboard = ArboardClipboard::new()?;
        clipboard.set_image(image_data).map_err(|error| {
            AppError::new("clipboard_set_image_failed", "写入图片到剪贴板失败")
                .with_detail(error.to_string())
        })?;

        let touched = state.clipboard_service.touch_item(id)?;
        emit_clipboard_sync(
            &app,
            ClipboardSyncPayload {
                upsert: vec![touched],
                removed_ids: Vec::new(),
                clear_all: false,
                reason: Some("copy_image_back".to_string()),
            },
        );

        Ok(())
    })();

    match &result {
        Ok(_) => command_end_ok("clipboard_copy_image_back", &request_id, started_at),
        Err(error) => {
            command_end_error("clipboard_copy_image_back", &request_id, started_at, error)
        }
    }
    result
}

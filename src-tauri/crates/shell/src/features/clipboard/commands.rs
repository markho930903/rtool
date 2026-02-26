use super::{run_command_async, run_command_sync};
use crate::app::state::AppState;
use crate::features::clipboard::events::emit_clipboard_sync;
#[cfg(test)]
use crate::features::clipboard::system_clipboard::{
    build_macos_copy_files_script, ensure_expected_and_actual_file_paths,
    normalize_path_for_compare,
};
use crate::features::clipboard::system_clipboard::{
    copy_files_to_clipboard_with_verify, decode_data_url_image_bytes,
    parse_file_paths_from_plain_text,
};
use anyhow::Context;
use usecase::services::ClipboardApplicationService;
use foundation::models::{
    ClipboardFilterDto, ClipboardItemDto, ClipboardSyncPayload, ClipboardWindowModeAppliedDto,
};
use foundation::{AppError, AppResult, InvokeError, ResultExt};
use foundation::runtime::blocking::run_blocking;
use arboard::{Clipboard as ArboardClipboard, ImageData};
use image::ImageReader;
use std::borrow::Cow;
use tauri::{AppHandle, State};

fn default_filter() -> ClipboardFilterDto {
    ClipboardFilterDto {
        query: None,
        item_type: None,
        only_pinned: Some(false),
        limit: Some(100),
    }
}

fn map_arboard_error(error: arboard::Error) -> AppError {
    AppError::new("clipboard_error", "剪贴板操作失败").with_source(error)
}

async fn fetch_clipboard_item_or_not_found(
    service: ClipboardApplicationService,
    query_id: String,
) -> AppResult<ClipboardItemDto> {
    service.get_item_or_not_found(query_id).await
}

async fn touch_clipboard_item(
    service: ClipboardApplicationService,
    item_id: String,
) -> AppResult<ClipboardItemDto> {
    service.touch_item(item_id).await
}

fn emit_clipboard_touch_sync(app: &AppHandle, touched: ClipboardItemDto, reason: &str) {
    emit_clipboard_sync(
        app,
        ClipboardSyncPayload {
            upsert: vec![touched],
            removed_ids: Vec::new(),
            clear_all: false,
            reason: Some(reason.to_string()),
        },
    );
}

#[tauri::command]
pub async fn clipboard_list(
    state: State<'_, AppState>,
    filter: Option<ClipboardFilterDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<ClipboardItemDto>, InvokeError> {
    let service = state.app_services.clipboard.clone();
    let filter = filter.unwrap_or_else(default_filter);
    run_command_async(
        "clipboard_list",
        request_id,
        window_label,
        move || async move { service.list(filter).await },
    )
    .await
}

#[tauri::command]
pub async fn clipboard_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_async(
        "clipboard_pin",
        request_id,
        window_label,
        move || async move {
            let service = state.app_services.clipboard.clone();
            let updated = service.pin(id, pinned).await?;
            emit_clipboard_sync(
                &app,
                ClipboardSyncPayload {
                    upsert: vec![updated],
                    removed_ids: Vec::new(),
                    clear_all: false,
                    reason: Some("pin".to_string()),
                },
            );
            Ok::<(), AppError>(())
        },
    )
    .await
}

#[tauri::command]
pub async fn clipboard_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_async(
        "clipboard_delete",
        request_id,
        window_label,
        move || async move {
            let service = state.app_services.clipboard.clone();
            let removed_id = id.clone();
            service.delete(id).await?;
            emit_clipboard_sync(
                &app,
                ClipboardSyncPayload {
                    upsert: Vec::new(),
                    removed_ids: vec![removed_id],
                    clear_all: false,
                    reason: Some("delete".to_string()),
                },
            );
            Ok::<(), AppError>(())
        },
    )
    .await
}

#[tauri::command]
pub async fn clipboard_clear_all(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_async(
        "clipboard_clear_all",
        request_id,
        window_label,
        move || async move {
            let service = state.app_services.clipboard.clone();
            service.clear_all().await?;
            emit_clipboard_sync(
                &app,
                ClipboardSyncPayload {
                    upsert: Vec::new(),
                    removed_ids: Vec::new(),
                    clear_all: true,
                    reason: Some("clear_all".to_string()),
                },
            );
            Ok::<(), AppError>(())
        },
    )
    .await
}

#[tauri::command]
pub async fn clipboard_save_text(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ClipboardItemDto, InvokeError> {
    run_command_async(
        "clipboard_save_text",
        request_id,
        window_label,
        move || async move {
            let service = state.app_services.clipboard.clone();
            let saved = service.save_text(text, None).await?;
            emit_clipboard_sync(
                &app,
                ClipboardSyncPayload {
                    upsert: vec![saved.item.clone()],
                    removed_ids: saved.removed_ids,
                    clear_all: false,
                    reason: Some("save_text".to_string()),
                },
            );
            Ok::<ClipboardItemDto, AppError>(saved.item)
        },
    )
    .await
}

#[tauri::command]
pub fn clipboard_window_set_mode(
    state: State<'_, AppState>,
    compact: bool,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_sync(
        "clipboard_window_set_mode",
        request_id,
        window_label,
        move || {
            state.set_clipboard_window_compact(compact);
            Ok::<_, InvokeError>(())
        },
    )
}

#[tauri::command]
pub fn clipboard_window_apply_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    compact: bool,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ClipboardWindowModeAppliedDto, InvokeError> {
    run_command_sync(
        "clipboard_window_apply_mode",
        request_id,
        window_label,
        move || {
            let applied = crate::apply_clipboard_window_mode(&app, compact, "command")?;
            state.set_clipboard_window_compact(compact);
            Ok::<ClipboardWindowModeAppliedDto, AppError>(applied)
        },
    )
}

#[tauri::command]
pub async fn clipboard_copy_back(
    app: AppHandle,
    state: State<'_, AppState>,
    clipboard_plugin: State<'_, tauri_plugin_clipboard::Clipboard>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let clipboard_service = state.app_services.clipboard.clone();
    run_command_async(
        "clipboard_copy_back",
        request_id,
        window_label,
        move || async move {
            let item =
                fetch_clipboard_item_or_not_found(clipboard_service.clone(), id.clone()).await?;
            if item.item_type == "file" {
                let file_paths = parse_file_paths_from_plain_text(&item.plain_text)?;
                copy_files_to_clipboard_with_verify(clipboard_plugin.inner(), &file_paths)?;
            } else {
                let mut clipboard = ArboardClipboard::new().map_err(map_arboard_error)?;
                clipboard
                    .set_text(item.plain_text)
                    .map_err(map_arboard_error)?;
            }

            let touched = touch_clipboard_item(clipboard_service, id.clone()).await?;
            emit_clipboard_touch_sync(&app, touched, "copy_back");
            Ok::<(), AppError>(())
        },
    )
    .await
}

#[cfg(test)]
#[path = "../../../tests/commands/clipboard_tests.rs"]
mod tests;

#[tauri::command]
pub async fn clipboard_copy_file_paths(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let clipboard_service = state.app_services.clipboard.clone();
    run_command_async(
        "clipboard_copy_file_paths",
        request_id,
        window_label,
        move || async move {
            let item =
                fetch_clipboard_item_or_not_found(clipboard_service.clone(), id.clone()).await?;
            if item.item_type != "file" {
                return Err(AppError::new("clipboard_not_file", "当前条目不是文件类型"));
            }

            let mut clipboard = ArboardClipboard::new().map_err(map_arboard_error)?;
            clipboard
                .set_text(item.plain_text)
                .map_err(map_arboard_error)?;

            let touched = touch_clipboard_item(clipboard_service, id.clone()).await?;
            emit_clipboard_touch_sync(&app, touched, "copy_file_paths");
            Ok(())
        },
    )
    .await
}

#[tauri::command]
pub async fn clipboard_copy_image_back(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let clipboard_service = state.app_services.clipboard.clone();
    run_command_async(
        "clipboard_copy_image_back",
        request_id,
        window_label,
        move || async move {
            let item =
                fetch_clipboard_item_or_not_found(clipboard_service.clone(), id.clone()).await?;
            if item.item_type != "image" {
                return Err(AppError::new("clipboard_not_image", "当前条目不是图片类型"));
            }

            let preview_path = item.preview_path.clone();
            let preview_data_url = item.preview_data_url.clone();
            let (width, height, bytes) =
                run_blocking("clipboard_copy_image_back_decode", move || {
                    let image_from_path = preview_path.as_ref().and_then(|path| {
                        let reader = ImageReader::open(path).ok()?;
                        reader.decode().ok()
                    });

                    let image = if let Some(image) = image_from_path {
                        image
                    } else if let Some(data_url) = preview_data_url.as_deref() {
                        let decoded_bytes = decode_data_url_image_bytes(data_url)?;
                        image::load_from_memory(&decoded_bytes)
                            .with_context(|| {
                                format!("解码图片失败: data_url_len={}", data_url.len())
                            })
                            .with_code("image_decode_failed", "解码图片失败")
                            .with_ctx("dataUrlLength", data_url.len().to_string())?
                    } else {
                        return Err(AppError::new("image_preview_missing", "图片预览数据不存在"));
                    };

                    let rgba = image.to_rgba8();
                    let (width, height) = rgba.dimensions();
                    Ok((width, height, rgba.into_raw()))
                })
                .await?;

            let image_data = ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(bytes),
            };

            let mut clipboard = ArboardClipboard::new().map_err(map_arboard_error)?;
            clipboard
                .set_image(image_data)
                .with_context(|| format!("写入图片到剪贴板失败: id={id}"))
                .with_code("clipboard_set_image_failed", "写入图片到剪贴板失败")
                .with_ctx("itemId", id.clone())?;

            let touched = touch_clipboard_item(clipboard_service, id.clone()).await?;
            emit_clipboard_touch_sync(&app, touched, "copy_image_back");

            Ok(())
        },
    )
    .await
}

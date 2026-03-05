use crate::app::state::AppState;
use crate::features::clipboard::events::emit_clipboard_sync;
use crate::features::clipboard::system_clipboard::{
    copy_files_to_clipboard_with_verify, decode_data_url_image_bytes,
    parse_file_paths_from_plain_text,
};
use crate::shared::command_response::CommandPayloadContext;
use crate::shared::command_runtime::{run_blocking, run_command_async, run_command_sync};
use crate::shared::request_context::InvokeMeta;
use anyhow::Context;
use arboard::{Clipboard as ArboardClipboard, ImageData};
use image::ImageReader;
use rtool_app::services::ClipboardApplicationService;
use rtool_contracts::models::{
    ClipboardFilterDto, ClipboardImageExportResultDto, ClipboardItemDto, ClipboardSyncPayload,
    ClipboardWindowModeAppliedDto,
};
use rtool_contracts::{AppError, AppResult, InvokeError, ResultExt};
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::io::Cursor;
use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

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

fn decode_clipboard_image(
    preview_path: Option<String>,
    preview_data_url: Option<String>,
) -> AppResult<image::DynamicImage> {
    let image_from_path = preview_path.as_ref().and_then(|path| {
        let reader = ImageReader::open(path).ok()?;
        reader.decode().ok()
    });

    if let Some(image) = image_from_path {
        return Ok(image);
    }

    if let Some(data_url) = preview_data_url.as_deref() {
        let decoded_bytes = decode_data_url_image_bytes(data_url)?;
        return image::load_from_memory(&decoded_bytes)
            .with_context(|| format!("解码图片失败: data_url_len={}", data_url.len()))
            .with_code("image_decode_failed", "解码图片失败")
            .with_ctx("dataUrlLength", data_url.len().to_string());
    }

    Err(AppError::new("image_preview_missing", "图片预览数据不存在"))
}

fn encode_clipboard_image_as_png_bytes(image: image::DynamicImage) -> AppResult<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .with_context(|| "编码图片失败: format=png")
        .with_code("image_encode_failed", "编码图片失败")?;
    Ok(buffer.into_inner())
}

fn append_png_extension_if_missing(mut path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        return path;
    }
    path.set_extension("png");
    path
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

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ClipboardListPayload {
    filter: Option<ClipboardFilterDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipboardPinPayload {
    id: String,
    pinned: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipboardIdPayload {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipboardSaveTextPayload {
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipboardWindowModePayload {
    compact: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub(crate) enum ClipboardRequest {
    List(ClipboardListPayload),
    Pin(ClipboardPinPayload),
    Delete(ClipboardIdPayload),
    ClearAll,
    SaveText(ClipboardSaveTextPayload),
    WindowSetMode(ClipboardWindowModePayload),
    WindowApplyMode(ClipboardWindowModePayload),
    CopyBack(ClipboardIdPayload),
    CopyFilePaths(ClipboardIdPayload),
    CopyImageBack(ClipboardIdPayload),
    ExportImage(ClipboardIdPayload),
}

const CLIPBOARD_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "clipboard",
    "剪贴板命令参数无效",
    "剪贴板命令返回序列化失败",
    "未知剪贴板命令",
);

async fn clipboard_list(
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

async fn clipboard_pin(
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

async fn clipboard_delete(
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

async fn clipboard_clear_all(
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

async fn clipboard_save_text(
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

fn clipboard_window_set_mode(
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

fn clipboard_window_apply_mode(
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
            let applied =
                crate::platform::native_ui::clipboard_window::apply_clipboard_window_mode(
                    &app, compact, "command",
                )?;
            state.set_clipboard_window_compact(compact);
            Ok::<ClipboardWindowModeAppliedDto, AppError>(applied)
        },
    )
}

async fn clipboard_copy_back(
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

async fn clipboard_copy_file_paths(
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

async fn clipboard_copy_image_back(
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
                    let image = decode_clipboard_image(preview_path, preview_data_url)?;
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

async fn clipboard_export_image(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ClipboardImageExportResultDto, InvokeError> {
    let clipboard_service = state.app_services.clipboard.clone();
    run_command_async(
        "clipboard_export_image",
        request_id,
        window_label,
        move || async move {
            let item =
                fetch_clipboard_item_or_not_found(clipboard_service.clone(), id.clone()).await?;
            if item.item_type != "image" {
                return Err(AppError::new("clipboard_not_image", "当前条目不是图片类型"));
            }

            let selected_path = app
                .dialog()
                .file()
                .set_title("保存图片")
                .set_file_name(format!("{}.png", item.id))
                .add_filter("PNG Image", &["png"])
                .blocking_save_file();

            let Some(file_path) = selected_path else {
                return Ok(ClipboardImageExportResultDto {
                    saved: false,
                    path: None,
                });
            };

            let output_path = append_png_extension_if_missing(
                file_path
                    .into_path()
                    .with_code("clipboard_export_image_path_invalid", "保存路径无效")?,
            );
            let output_path_for_write = output_path.clone();
            let output_path_text = output_path.to_string_lossy().to_string();
            let preview_path = item.preview_path.clone();
            let preview_data_url = item.preview_data_url.clone();
            let item_id = item.id.clone();

            run_blocking("clipboard_export_image_write", move || {
                let image = decode_clipboard_image(preview_path, preview_data_url)?;
                let png_bytes = encode_clipboard_image_as_png_bytes(image)?;
                std::fs::write(&output_path_for_write, png_bytes)
                    .with_context(|| {
                        format!("写入图片文件失败: path={}", output_path_for_write.display())
                    })
                    .with_code("clipboard_export_image_write_failed", "保存图片失败")
                    .with_ctx("itemId", item_id)
                    .with_ctx("outputPath", output_path_for_write.display().to_string())?;
                Ok::<(), AppError>(())
            })
            .await?;

            Ok(ClipboardImageExportResultDto {
                saved: true,
                path: Some(output_path_text),
            })
        },
    )
    .await
}

pub(crate) async fn handle_clipboard(
    app: AppHandle,
    state: State<'_, AppState>,
    clipboard_plugin: State<'_, tauri_plugin_clipboard::Clipboard>,
    request: ClipboardRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    let (request_id, window_label) = meta.unwrap_or_default().split();

    match request {
        ClipboardRequest::List(payload) => CLIPBOARD_COMMAND_CONTEXT.serialize(
            "list",
            clipboard_list(state, payload.filter, request_id, window_label).await?,
        ),
        ClipboardRequest::Pin(payload) => {
            clipboard_pin(
                app,
                state,
                payload.id,
                payload.pinned,
                request_id,
                window_label,
            )
            .await?;
            Ok(Value::Null)
        }
        ClipboardRequest::Delete(payload) => {
            clipboard_delete(app, state, payload.id, request_id, window_label).await?;
            Ok(Value::Null)
        }
        ClipboardRequest::ClearAll => {
            clipboard_clear_all(app, state, request_id, window_label).await?;
            Ok(Value::Null)
        }
        ClipboardRequest::SaveText(payload) => CLIPBOARD_COMMAND_CONTEXT.serialize(
            "save_text",
            clipboard_save_text(app, state, payload.text, request_id, window_label).await?,
        ),
        ClipboardRequest::WindowSetMode(payload) => {
            clipboard_window_set_mode(state, payload.compact, request_id, window_label)?;
            Ok(Value::Null)
        }
        ClipboardRequest::WindowApplyMode(payload) => CLIPBOARD_COMMAND_CONTEXT.serialize(
            "window_apply_mode",
            clipboard_window_apply_mode(app, state, payload.compact, request_id, window_label)?,
        ),
        ClipboardRequest::CopyBack(payload) => {
            clipboard_copy_back(
                app,
                state,
                clipboard_plugin,
                payload.id,
                request_id,
                window_label,
            )
            .await?;
            Ok(Value::Null)
        }
        ClipboardRequest::CopyFilePaths(payload) => {
            clipboard_copy_file_paths(app, state, payload.id, request_id, window_label).await?;
            Ok(Value::Null)
        }
        ClipboardRequest::CopyImageBack(payload) => {
            clipboard_copy_image_back(app, state, payload.id, request_id, window_label).await?;
            Ok(Value::Null)
        }
        ClipboardRequest::ExportImage(payload) => CLIPBOARD_COMMAND_CONTEXT.serialize(
            "export_image",
            clipboard_export_image(app, state, payload.id, request_id, window_label).await?,
        ),
    }
}

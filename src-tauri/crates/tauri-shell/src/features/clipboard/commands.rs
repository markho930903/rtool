use super::{run_blocking_command, run_command_async, run_command_sync};
use app_clipboard::service::ClipboardService;
use crate::app::state::AppState;
use app_core::models::{
    ClipboardFilterDto, ClipboardItemDto, ClipboardSettingsDto, ClipboardSyncPayload,
    ClipboardWindowModeAppliedDto,
};
use app_core::{AppError, AppResult, InvokeError, ResultExt};
use crate::features::clipboard::events::emit_clipboard_sync;
use app_infra::db;
use app_infra::runtime::blocking::run_blocking;
use anyhow::Context;
use arboard::{Clipboard as ArboardClipboard, ImageData};
use base64::Engine as _;
use image::ImageReader;
use std::borrow::Cow;
use std::collections::BTreeSet;
use tauri::{AppHandle, State};

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
        .with_context(|| format!("解析图片数据失败: encoded_len={}", encoded.len()))
        .with_code("image_data_url_decode_failed", "解析图片数据失败")
        .with_ctx("encodedLength", encoded.len().to_string())
}

fn parse_file_paths_from_plain_text(plain_text: &str) -> AppResult<Vec<String>> {
    app_infra::clipboard::parse_file_paths_from_text(plain_text).ok_or_else(|| {
        AppError::new(
            "clipboard_file_payload_invalid",
            "文件条目路径数据无效或目标文件不存在",
        )
    })
}

#[cfg(all(not(target_os = "macos"), target_os = "linux"))]
fn to_clipboard_files_uris(file_paths: &[String]) -> Vec<String> {
    file_paths
        .iter()
        .map(|path| format!("file://{path}"))
        .collect()
}

#[cfg(all(not(target_os = "macos"), target_os = "windows"))]
fn to_clipboard_files_uris(file_paths: &[String]) -> Vec<String> {
    file_paths.to_vec()
}

fn normalize_path_for_compare(path: &str) -> String {
    let trimmed = path.trim().trim_matches('"').trim_matches('\'');
    let without_uri = if let Some(value) = trimmed.strip_prefix("file://") {
        #[cfg(target_os = "windows")]
        {
            value.strip_prefix('/').unwrap_or(value)
        }
        #[cfg(not(target_os = "windows"))]
        {
            value
        }
    } else {
        trimmed
    };

    #[cfg(target_os = "windows")]
    {
        without_uri.replace('/', "\\").to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        without_uri.to_string()
    }
}

fn normalize_file_paths_for_compare(file_paths: &[String]) -> BTreeSet<String> {
    file_paths
        .iter()
        .map(|path| normalize_path_for_compare(path))
        .collect()
}

fn file_paths_digest(file_paths: &[String]) -> String {
    let mut normalized: Vec<String> = normalize_file_paths_for_compare(file_paths)
        .into_iter()
        .collect();
    normalized.sort();
    let hash = blake3::hash(normalized.join("\n").as_bytes())
        .to_hex()
        .to_string();
    hash.chars().take(16).collect()
}

fn ensure_expected_and_actual_file_paths(
    expected_file_paths: &[String],
    actual_file_paths: &[String],
) -> AppResult<()> {
    let expected = normalize_file_paths_for_compare(expected_file_paths);
    let actual = normalize_file_paths_for_compare(actual_file_paths);

    if expected == actual {
        return Ok(());
    }

    Err(AppError::new(
        "clipboard_set_files_verify_failed",
        "文件复制失败，写入剪贴板的文件与预期不一致",
    ))
}

#[cfg(any(target_os = "macos", test))]
fn escape_applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(any(target_os = "macos", test))]
fn build_macos_copy_files_script(file_paths: &[String]) -> String {
    let file_list = file_paths
        .iter()
        .map(|path| format!("POSIX file \"{}\"", escape_applescript_string(path)))
        .collect::<Vec<String>>()
        .join(", ");
    format!("set the clipboard to {{{file_list}}}")
}

#[cfg(target_os = "macos")]
fn write_files_to_system_clipboard(
    _clipboard_plugin: &tauri_plugin_clipboard::Clipboard,
    file_paths: &[String],
) -> AppResult<()> {
    let script = build_macos_copy_files_script(file_paths);
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .with_context(|| format!("执行系统剪贴板脚本失败: file_count={}", file_paths.len()))
        .with_code(
            "clipboard_set_files_unsupported_target",
            "文件复制失败，当前环境不支持文件粘贴",
        )
        .with_ctx("fileCount", file_paths.len().to_string())?;

    if output.status.success() {
        return Ok(());
    }

    Err(
        AppError::new("clipboard_set_files_failed", "写入文件到剪贴板失败")
            .with_context("exitCode", format!("{:?}", output.status.code())),
    )
}

#[cfg(not(target_os = "macos"))]
fn write_files_to_system_clipboard(
    clipboard_plugin: &tauri_plugin_clipboard::Clipboard,
    file_paths: &[String],
) -> AppResult<()> {
    let files_uris = to_clipboard_files_uris(file_paths);
    clipboard_plugin
        .write_files_uris(files_uris)
        .map_err(|error| {
            AppError::new("clipboard_set_files_failed", "写入文件到剪贴板失败").with_source(error)
        })
}

fn verify_files_written_to_clipboard(
    clipboard_plugin: &tauri_plugin_clipboard::Clipboard,
    expected_file_paths: &[String],
) -> AppResult<()> {
    let has_files = clipboard_plugin.has_files().map_err(|error| {
        AppError::new(
            "clipboard_set_files_unsupported_target",
            "文件复制失败，当前目标应用不支持文件粘贴",
        )
        .with_cause(error)
    })?;
    if !has_files {
        return Err(AppError::new(
            "clipboard_set_files_unsupported_target",
            "文件复制失败，当前目标应用不支持文件粘贴",
        ));
    }

    let actual_files_uris = clipboard_plugin.read_files_uris().map_err(|error| {
        AppError::new(
            "clipboard_set_files_unsupported_target",
            "文件复制失败，当前目标应用不支持文件粘贴",
        )
        .with_cause(error)
    })?;
    if actual_files_uris.is_empty() {
        return Err(AppError::new(
            "clipboard_set_files_verify_failed",
            "文件复制失败，未检测到文件剪贴板数据",
        ));
    }

    let actual_file_paths = parse_file_paths_from_plain_text(&actual_files_uris.join("\n"))
        .map_err(|_| {
            AppError::new(
                "clipboard_set_files_verify_failed",
                "文件复制失败，目标未识别为文件粘贴",
            )
        })?;
    ensure_expected_and_actual_file_paths(expected_file_paths, &actual_file_paths)
}

fn copy_files_to_clipboard_with_verify(
    clipboard_plugin: &tauri_plugin_clipboard::Clipboard,
    file_paths: &[String],
) -> AppResult<()> {
    let file_count = file_paths.len();
    let file_digest = file_paths_digest(file_paths);
    tracing::info!(
        event = "clipboard_file_copy_write_started",
        file_count,
        file_digest = file_digest.as_str()
    );

    if let Err(error) = write_files_to_system_clipboard(clipboard_plugin, file_paths) {
        tracing::warn!(
            event = "clipboard_file_copy_write_failed",
            file_count,
            file_digest = file_digest.as_str(),
            error_code = error.code.as_str()
        );
        return Err(error);
    }

    if let Err(error) = verify_files_written_to_clipboard(clipboard_plugin, file_paths) {
        tracing::warn!(
            event = "clipboard_file_copy_verify_failed",
            file_count,
            file_digest = file_digest.as_str(),
            error_code = error.code.as_str()
        );
        return Err(error);
    }

    tracing::info!(
        event = "clipboard_file_copy_succeeded",
        file_count,
        file_digest = file_digest.as_str()
    );
    Ok(())
}

async fn fetch_clipboard_item_or_not_found(
    pool: db::DbPool,
    query_id: String,
    query_label: &'static str,
) -> AppResult<ClipboardItemDto> {
    let item = run_blocking(query_label, move || {
        db::get_clipboard_item(&pool, &query_id)
    })
    .await?;
    item.ok_or_else(|| AppError::new("clipboard_not_found", "未找到对应剪贴板记录"))
}

async fn touch_clipboard_item(
    service: ClipboardService,
    item_id: String,
    touch_label: &'static str,
) -> AppResult<ClipboardItemDto> {
    run_blocking(touch_label, move || service.touch_item(item_id)).await
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
    let service = state.clipboard_service.clone();
    let filter = filter.unwrap_or_else(default_filter);
    run_blocking_command(
        "clipboard_list",
        request_id,
        window_label,
        "clipboard_list",
        move || service.list(filter),
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
            let service = state.clipboard_service.clone();
            let updated = run_blocking("clipboard_pin", move || service.pin(id, pinned)).await?;
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
            let service = state.clipboard_service.clone();
            let removed_id = id.clone();
            run_blocking("clipboard_delete", move || service.delete(id)).await?;
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
            let service = state.clipboard_service.clone();
            run_blocking("clipboard_clear_all", move || service.clear_all()).await?;
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
            let service = state.clipboard_service.clone();
            let saved =
                run_blocking("clipboard_save_text", move || service.save_text(text, None)).await?;
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
pub async fn clipboard_get_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ClipboardSettingsDto, InvokeError> {
    let service = state.clipboard_service.clone();
    run_blocking_command(
        "clipboard_get_settings",
        request_id,
        window_label,
        "clipboard_get_settings",
        move || Ok(service.get_settings()),
    )
    .await
}

#[tauri::command]
pub async fn clipboard_update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    max_items: u32,
    size_cleanup_enabled: Option<bool>,
    max_total_size_mb: Option<u32>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ClipboardSettingsDto, InvokeError> {
    run_command_async(
        "clipboard_update_settings",
        request_id,
        window_label,
        move || async move {
            let service = state.clipboard_service.clone();
            let updated = run_blocking("clipboard_update_settings", move || {
                service.update_settings(max_items, size_cleanup_enabled, max_total_size_mb)
            })
            .await?;
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
            Ok::<ClipboardSettingsDto, AppError>(updated.settings)
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
    run_command_async(
        "clipboard_copy_back",
        request_id,
        window_label,
        move || async move {
            let item = fetch_clipboard_item_or_not_found(
                state.db_pool.clone(),
                id.clone(),
                "clipboard_copy_back_query",
            )
            .await?;
            if item.item_type == "file" {
                let file_paths = parse_file_paths_from_plain_text(&item.plain_text)?;
                copy_files_to_clipboard_with_verify(clipboard_plugin.inner(), &file_paths)?;
            } else {
                let mut clipboard = ArboardClipboard::new().map_err(AppError::from)?;
                clipboard
                    .set_text(item.plain_text)
                    .map_err(AppError::from)?;
            }

            let touched = touch_clipboard_item(
                state.clipboard_service.clone(),
                id.clone(),
                "clipboard_copy_back_touch",
            )
            .await?;
            emit_clipboard_touch_sync(&app, touched, "copy_back");
            Ok::<(), AppError>(())
        },
    )
    .await
}

#[cfg(test)]
#[path = "../../../../../tests/commands/clipboard_tests.rs"]
mod tests;

#[tauri::command]
pub async fn clipboard_copy_file_paths(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_async(
        "clipboard_copy_file_paths",
        request_id,
        window_label,
        move || async move {
            let item = fetch_clipboard_item_or_not_found(
                state.db_pool.clone(),
                id.clone(),
                "clipboard_copy_file_paths_query",
            )
            .await?;
            if item.item_type != "file" {
                return Err(AppError::new("clipboard_not_file", "当前条目不是文件类型"));
            }

            let mut clipboard = ArboardClipboard::new().map_err(AppError::from)?;
            clipboard
                .set_text(item.plain_text)
                .map_err(AppError::from)?;

            let touched = touch_clipboard_item(
                state.clipboard_service.clone(),
                id.clone(),
                "clipboard_copy_file_paths_touch",
            )
            .await?;
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
    run_command_async(
        "clipboard_copy_image_back",
        request_id,
        window_label,
        move || async move {
            let item = fetch_clipboard_item_or_not_found(
                state.db_pool.clone(),
                id.clone(),
                "clipboard_copy_image_back_query",
            )
            .await?;
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

            let mut clipboard = ArboardClipboard::new().map_err(AppError::from)?;
            clipboard
                .set_image(image_data)
                .with_context(|| format!("写入图片到剪贴板失败: id={id}"))
                .with_code("clipboard_set_image_failed", "写入图片到剪贴板失败")
                .with_ctx("itemId", id.clone())?;

            let touched = touch_clipboard_item(
                state.clipboard_service.clone(),
                id.clone(),
                "clipboard_copy_image_back_touch",
            )
            .await?;
            emit_clipboard_touch_sync(&app, touched, "copy_image_back");

            Ok(())
        },
    )
    .await
}

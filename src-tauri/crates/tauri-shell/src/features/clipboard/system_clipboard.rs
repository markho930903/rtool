use anyhow::Context;
use app_core::{AppError, AppResult, ResultExt};
use base64::Engine as _;
use std::collections::BTreeSet;

pub fn decode_data_url_image_bytes(data_url: &str) -> AppResult<Vec<u8>> {
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

pub fn parse_file_paths_from_plain_text(plain_text: &str) -> AppResult<Vec<String>> {
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

pub(crate) fn normalize_path_for_compare(path: &str) -> String {
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

pub(crate) fn ensure_expected_and_actual_file_paths(
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
pub(crate) fn build_macos_copy_files_script(file_paths: &[String]) -> String {
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

pub fn copy_files_to_clipboard_with_verify(
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

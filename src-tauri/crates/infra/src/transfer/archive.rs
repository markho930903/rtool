use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

use anyhow::Context;
use app_core::models::TransferFileInputDto;
use app_core::{AppError, AppResult, ResultExt};

#[derive(Debug, Clone)]
pub struct TransferSourceFile {
    pub source_path: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub is_folder_archive: bool,
}

#[derive(Debug, Clone)]
pub struct TransferSourceBundle {
    pub files: Vec<TransferSourceFile>,
    pub temp_paths: Vec<PathBuf>,
}

fn file_name_or_fallback(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn resolve_relative_path(input_relative: Option<&String>, fallback_path: &Path) -> String {
    input_relative
        .map(|value| value.trim().replace('\\', "/"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| file_name_or_fallback(fallback_path))
}

fn push_file(
    output: &mut Vec<TransferSourceFile>,
    path: &Path,
    relative_path: String,
    is_folder_archive: bool,
) -> AppResult<()> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("读取源文件元数据失败: {}", path.display()))
        .with_code("transfer_source_metadata_failed", "准备传输文件失败")
        .with_ctx("path", path.display().to_string())?;
    if !metadata.is_file() {
        return Ok(());
    }

    output.push(TransferSourceFile {
        source_path: path.to_string_lossy().to_string(),
        relative_path,
        size_bytes: metadata.len(),
        is_folder_archive,
    });
    Ok(())
}

fn build_archive_for_folder(path: &Path) -> AppResult<PathBuf> {
    let base_name = file_name_or_fallback(path);
    let temp_root = std::env::temp_dir().join("rtool-transfer-archives");
    std::fs::create_dir_all(temp_root.as_path())
        .with_context(|| format!("创建临时归档目录失败: {}", temp_root.display()))
        .with_code("transfer_archive_dir_create_failed", "准备传输文件失败")
        .with_ctx("tempRoot", temp_root.display().to_string())?;

    let temp_file = temp_root.join(format!("{}-{}.zip", base_name, uuid::Uuid::new_v4()));
    let handle = File::create(temp_file.as_path())
        .with_context(|| format!("创建临时归档文件失败: {}", temp_file.display()))
        .with_code("transfer_archive_create_failed", "准备传输文件失败")
        .with_ctx("tempArchive", temp_file.display().to_string())?;

    let mut zip = zip::ZipWriter::new(handle);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }

        let Ok(relative) = entry_path.strip_prefix(path) else {
            continue;
        };

        let zip_path = format!(
            "{}/{}",
            base_name,
            relative.to_string_lossy().replace('\\', "/")
        );
        zip.start_file(zip_path, options)
            .with_context(|| format!("写入归档条目失败: {}", entry_path.display()))
            .with_code("transfer_archive_write_failed", "准备传输文件失败")
            .with_ctx("sourcePath", entry_path.display().to_string())?;

        let mut input = File::open(entry_path)
            .with_context(|| format!("打开归档源文件失败: {}", entry_path.display()))
            .with_code("transfer_archive_source_open_failed", "准备传输文件失败")
            .with_ctx("sourcePath", entry_path.display().to_string())?;
        let mut buffer = [0u8; 16 * 1024];
        loop {
            let read_count = input
                .read(buffer.as_mut_slice())
                .with_context(|| format!("读取归档源文件失败: {}", entry_path.display()))
                .with_code("transfer_archive_source_read_failed", "准备传输文件失败")
                .with_ctx("sourcePath", entry_path.display().to_string())?;
            if read_count == 0 {
                break;
            }
            zip.write_all(&buffer[..read_count])
                .with_context(|| format!("写入归档输出失败: {}", temp_file.display()))
                .with_code("transfer_archive_write_failed", "准备传输文件失败")
                .with_ctx("tempArchive", temp_file.display().to_string())?;
        }
    }

    zip.finish()
        .with_context(|| format!("完成归档失败: {}", temp_file.display()))
        .with_code("transfer_archive_finish_failed", "准备传输文件失败")
        .with_ctx("tempArchive", temp_file.display().to_string())?;

    Ok(temp_file)
}

pub fn collect_sources(inputs: &[TransferFileInputDto]) -> AppResult<TransferSourceBundle> {
    if inputs.is_empty() {
        return Err(AppError::new(
            "transfer_files_empty",
            "至少需要选择一个文件或目录",
        ));
    }

    let mut files = Vec::new();
    let mut temp_paths = Vec::new();

    for input in inputs {
        let trimmed_path = input.path.trim();
        if trimmed_path.is_empty() {
            continue;
        }

        let path = PathBuf::from(trimmed_path);
        let metadata = std::fs::metadata(path.as_path())
            .with_context(|| format!("读取传输源路径失败: {}", path.display()))
            .with_code("transfer_source_not_found", "准备传输文件失败")
            .with_ctx("path", path.display().to_string())?;

        if metadata.is_file() {
            push_file(
                &mut files,
                path.as_path(),
                resolve_relative_path(input.relative_path.as_ref(), path.as_path()),
                false,
            )?;
            continue;
        }

        if !metadata.is_dir() {
            continue;
        }

        if input.compress_folder.unwrap_or(false) {
            let archive_path = build_archive_for_folder(path.as_path())?;
            let relative_path = format!("{}.zip", file_name_or_fallback(path.as_path()));
            push_file(&mut files, archive_path.as_path(), relative_path, true)?;
            temp_paths.push(archive_path);
            continue;
        }

        let root_name = resolve_relative_path(input.relative_path.as_ref(), path.as_path());
        for entry in WalkDir::new(path.as_path())
            .into_iter()
            .filter_map(Result::ok)
        {
            let entry_path = entry.path();
            if !entry_path.is_file() {
                continue;
            }
            let Ok(relative) = entry_path.strip_prefix(path.as_path()) else {
                continue;
            };
            let relative_path = format!(
                "{}/{}",
                root_name,
                relative.to_string_lossy().replace('\\', "/")
            );
            push_file(&mut files, entry_path, relative_path, false)?;
        }
    }

    if files.is_empty() {
        return Err(AppError::new("transfer_files_empty", "没有可传输的文件"));
    }

    Ok(TransferSourceBundle { files, temp_paths })
}

pub fn cleanup_temp_paths(paths: &[PathBuf]) {
    for path in paths {
        if let Err(error) = std::fs::remove_file(path) {
            if error.kind() == std::io::ErrorKind::NotFound {
                continue;
            }
            tracing::warn!(
                event = "transfer_archive_cleanup_failed",
                path = %path.to_string_lossy(),
                error = error.to_string()
            );
        }
    }
}

#[cfg(test)]
#[path = "../../tests/transfer/archive_tests.rs"]
mod tests;

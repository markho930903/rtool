use std::io::Read;
use std::path::{Path, PathBuf};

use tokio::fs::{OpenOptions, create_dir_all};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use anyhow::Context;
use crate::{AppResult, ResultExt};

pub fn file_hash_hex(path: &Path) -> AppResult<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("打开文件并计算哈希失败: {}", path.display()))
        .with_code("transfer_file_read_failed", "文件传输读写失败")
        .with_ctx("path", path.display().to_string())?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read_count = file
            .read(buffer.as_mut_slice())
            .with_context(|| format!("读取文件并计算哈希失败: {}", path.display()))
            .with_code("transfer_file_read_failed", "文件传输读写失败")
            .with_ctx("path", path.display().to_string())?;
        if read_count == 0 {
            break;
        }
        hasher.update(&buffer[..read_count]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[derive(Debug)]
pub struct ChunkReader {
    path: PathBuf,
    file: tokio::fs::File,
}

impl ChunkReader {
    pub async fn open(path: &Path) -> AppResult<Self> {
        let file = tokio::fs::File::open(path)
            .await
            .with_context(|| format!("打开传输源文件失败: {}", path.display()))
            .with_code("transfer_source_open_failed", "文件传输读写失败")
            .with_ctx("path", path.display().to_string())?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
        })
    }

    pub async fn read_chunk(&mut self, chunk_index: u32, chunk_size: u32) -> AppResult<Vec<u8>> {
        let offset = u64::from(chunk_index) * u64::from(chunk_size);
        self.file
            .seek(std::io::SeekFrom::Start(offset))
            .await
            .with_context(|| format!("定位传输源文件失败: {}", self.path.display()))
            .with_code("transfer_source_seek_failed", "文件传输读写失败")
            .with_ctx("path", self.path.display().to_string())
            .with_ctx("chunkIndex", chunk_index.to_string())
            .with_ctx("chunkSize", chunk_size.to_string())?;

        let mut buffer = vec![0u8; chunk_size as usize];
        let read_count = self
            .file
            .read(buffer.as_mut_slice())
            .await
            .with_context(|| format!("读取传输源文件失败: {}", self.path.display()))
            .with_code("transfer_source_read_failed", "文件传输读写失败")
            .with_ctx("path", self.path.display().to_string())
            .with_ctx("chunkIndex", chunk_index.to_string())
            .with_ctx("chunkSize", chunk_size.to_string())?;
        buffer.truncate(read_count);
        Ok(buffer)
    }
}

#[derive(Debug)]
pub struct ChunkWriter {
    path: PathBuf,
    file: tokio::fs::File,
}

impl ChunkWriter {
    pub async fn open(path: &Path, total_size: Option<u64>) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent)
                .await
                .with_context(|| format!("创建传输目标目录失败: {}", parent.display()))
                .with_code("transfer_target_dir_create_failed", "文件传输读写失败")
                .with_ctx("path", parent.display().to_string())?;
        }

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .read(true)
            .open(path)
            .await
            .with_context(|| format!("打开传输目标文件失败: {}", path.display()))
            .with_code("transfer_target_open_failed", "文件传输读写失败")
            .with_ctx("path", path.display().to_string())?;

        if let Some(size) = total_size {
            file.set_len(size)
                .await
                .with_context(|| format!("预分配目标文件失败: {}", path.display()))
                .with_code("transfer_target_preallocate_failed", "文件传输读写失败")
                .with_ctx("path", path.display().to_string())
                .with_ctx("sizeBytes", size.to_string())?;
        }

        Ok(Self {
            path: path.to_path_buf(),
            file,
        })
    }

    pub async fn write_chunk(
        &mut self,
        chunk_index: u32,
        chunk_size: u32,
        bytes: &[u8],
    ) -> AppResult<()> {
        let offset = u64::from(chunk_index) * u64::from(chunk_size);
        self.file
            .seek(std::io::SeekFrom::Start(offset))
            .await
            .with_context(|| format!("定位传输目标文件失败: {}", self.path.display()))
            .with_code("transfer_target_seek_failed", "文件传输读写失败")
            .with_ctx("path", self.path.display().to_string())
            .with_ctx("chunkIndex", chunk_index.to_string())
            .with_ctx("chunkSize", chunk_size.to_string())?;

        self.file
            .write_all(bytes)
            .await
            .with_context(|| format!("写入传输目标文件失败: {}", self.path.display()))
            .with_code("transfer_target_write_failed", "文件传输读写失败")
            .with_ctx("path", self.path.display().to_string())
            .with_ctx("chunkIndex", chunk_index.to_string())
            .with_ctx("chunkLength", bytes.len().to_string())?;
        Ok(())
    }

    pub async fn flush(&mut self) -> AppResult<()> {
        self.file
            .flush()
            .await
            .with_context(|| format!("刷新传输目标文件失败: {}", self.path.display()))
            .with_code("transfer_target_flush_failed", "文件传输读写失败")
            .with_ctx("path", self.path.display().to_string())?;
        Ok(())
    }
}

#[allow(dead_code)]
pub async fn read_chunk(path: &Path, chunk_index: u32, chunk_size: u32) -> AppResult<Vec<u8>> {
    let mut reader = ChunkReader::open(path).await?;
    reader.read_chunk(chunk_index, chunk_size).await
}

#[allow(dead_code)]
pub async fn write_chunk(
    path: &Path,
    chunk_index: u32,
    chunk_size: u32,
    bytes: &[u8],
) -> AppResult<()> {
    let mut writer = ChunkWriter::open(path, None).await?;
    writer.write_chunk(chunk_index, chunk_size, bytes).await?;
    writer.flush().await?;
    Ok(())
}

pub fn build_part_path(save_dir: &Path, session_id: &str, relative_path: &str) -> PathBuf {
    let clean = relative_path.replace('\\', "/");
    let target = save_dir.join(clean);
    let file_name = target
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let part_file_name = format!("{file_name}.{session_id}.part");
    if let Some(parent) = target.parent() {
        return parent.join(part_file_name);
    }
    save_dir.join(part_file_name)
}

pub fn resolve_target_path(save_dir: &Path, relative_path: &str) -> PathBuf {
    let clean = relative_path.replace('\\', "/");
    save_dir.join(clean)
}

pub fn resolve_conflict_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_string());
    let parent = path
        .parent()
        .map(|value| value.to_path_buf())
        .unwrap_or_default();

    for index in 1..10_000 {
        let name = if let Some(ext) = extension.as_deref() {
            format!("{stem} ({index}).{ext}")
        } else {
            format!("{stem} ({index})")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }

    path.to_path_buf()
}

#[cfg(test)]
#[path = "../../tests/transfer/session_tests.rs"]
mod tests;

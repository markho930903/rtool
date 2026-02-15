use std::io::Read;
use std::path::{Path, PathBuf};

use tokio::fs::{OpenOptions, create_dir_all};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::core::{AppError, AppResult};

fn io_error(code: &str, message: impl Into<String>) -> AppError {
    AppError::new(code, "文件传输读写失败").with_detail(message.into())
}

pub fn file_hash_hex(path: &Path) -> AppResult<String> {
    let mut file = std::fs::File::open(path).map_err(|error| {
        io_error(
            "transfer_file_read_failed",
            format!("{}: {}", path.to_string_lossy(), error),
        )
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read_count = file.read(buffer.as_mut_slice()).map_err(|error| {
            io_error(
                "transfer_file_read_failed",
                format!("{}: {}", path.to_string_lossy(), error),
            )
        })?;
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
        let file = tokio::fs::File::open(path).await.map_err(|error| {
            io_error(
                "transfer_source_open_failed",
                format!("{}: {}", path.to_string_lossy(), error),
            )
        })?;
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
            .map_err(|error| {
                io_error(
                    "transfer_source_seek_failed",
                    format!("{}: {}", self.path.to_string_lossy(), error),
                )
            })?;

        let mut buffer = vec![0u8; chunk_size as usize];
        let read_count = self
            .file
            .read(buffer.as_mut_slice())
            .await
            .map_err(|error| {
                io_error(
                    "transfer_source_read_failed",
                    format!("{}: {}", self.path.to_string_lossy(), error),
                )
            })?;
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
            create_dir_all(parent).await.map_err(|error| {
                io_error(
                    "transfer_target_dir_create_failed",
                    format!("{}: {}", parent.to_string_lossy(), error),
                )
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .read(true)
            .open(path)
            .await
            .map_err(|error| {
                io_error(
                    "transfer_target_open_failed",
                    format!("{}: {}", path.to_string_lossy(), error),
                )
            })?;

        if let Some(size) = total_size {
            file.set_len(size).await.map_err(|error| {
                io_error(
                    "transfer_target_preallocate_failed",
                    format!("{}: {}", path.to_string_lossy(), error),
                )
            })?;
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
            .map_err(|error| {
                io_error(
                    "transfer_target_seek_failed",
                    format!("{}: {}", self.path.to_string_lossy(), error),
                )
            })?;

        self.file.write_all(bytes).await.map_err(|error| {
            io_error(
                "transfer_target_write_failed",
                format!("{}: {}", self.path.to_string_lossy(), error),
            )
        })?;
        Ok(())
    }

    pub async fn flush(&mut self) -> AppResult<()> {
        self.file.flush().await.map_err(|error| {
            io_error(
                "transfer_target_flush_failed",
                format!("{}: {}", self.path.to_string_lossy(), error),
            )
        })?;
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
mod tests {
    use super::*;

    #[test]
    fn file_hash_should_match_blake3() {
        let path =
            std::env::temp_dir().join(format!("rtool-transfer-hash-{}.txt", uuid::Uuid::new_v4()));
        let payload = b"transfer-hash-test";
        std::fs::write(path.as_path(), payload).expect("write temp file");

        let expected = blake3::hash(payload).to_hex().to_string();
        let actual = file_hash_hex(path.as_path()).expect("hash file");
        assert_eq!(expected, actual);

        let _ = std::fs::remove_file(path.as_path());
    }
}

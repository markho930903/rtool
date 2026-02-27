use super::pipeline::{TerminalPersistOptions, TransferHistorySyncReason};
use super::*;

#[derive(Debug)]
pub(super) struct IncomingFileRuntime {
    pub(super) file: TransferFileDto,
    pub(super) bitmap: Vec<u8>,
    pub(super) writer: ChunkWriter,
}

#[derive(Debug)]
pub(super) struct IncomingRuntimeState {
    pub(super) missing_chunks_payload: Vec<MissingChunkFrame>,
    pub(super) runtimes: Vec<IncomingFileRuntime>,
    pub(super) file_id_to_idx: HashMap<String, usize>,
}

pub(super) fn should_flush_ack_buffer(
    ack_buffer_len: usize,
    ack_batch_size: u32,
    elapsed: Duration,
    flush_interval: Duration,
) -> bool {
    ack_buffer_len > 0 && (ack_buffer_len >= ack_batch_size as usize || elapsed >= flush_interval)
}

pub(super) fn should_flush_incoming_checkpoint(elapsed: Duration, interval: Duration) -> bool {
    elapsed >= interval
}

pub(super) async fn poll_incoming_frame_raw<R>(
    reader: &mut R,
    session_key: &[u8; 32],
    codec: FrameCodec,
) -> AppResult<Option<TransferFrame>>
where
    R: tokio::io::AsyncRead + Unpin,
{
    match tokio::time::timeout(
        Duration::from_millis(40),
        read_frame_from(reader, Some(session_key), Some(codec)),
    )
    .await
    {
        Ok(Ok(frame)) => Ok(Some(frame)),
        Ok(Err(error)) => Err(error),
        Err(_) => Ok(None),
    }
}

fn resolve_incoming_done_terminal(
    remote_ok: bool,
    remote_error: Option<String>,
) -> (TransferStatus, Option<String>, Option<String>) {
    if remote_ok {
        (TransferStatus::Success, None, None)
    } else {
        (
            TransferStatus::Failed,
            Some("remote_failed".to_string()),
            remote_error,
        )
    }
}

impl TransferService {
    pub(super) async fn build_incoming_runtime_state(
        &self,
        session: &mut TransferSessionDto,
        manifest_files: Vec<ManifestFileFrame>,
        save_dir_path: &Path,
    ) -> AppResult<IncomingRuntimeState> {
        let mut missing_chunks_payload = Vec::new();
        let mut runtimes = Vec::<IncomingFileRuntime>::new();
        let mut file_id_to_idx = HashMap::<String, usize>::new();

        for manifest_file in manifest_files {
            let mut bitmap = self
                .get_file_bitmap_async(session.id.as_str(), manifest_file.file_id.as_str())
                .await
                .unwrap_or_default()
                .unwrap_or_else(|| empty_bitmap(manifest_file.chunk_count));
            if bitmap.is_empty() {
                bitmap = empty_bitmap(manifest_file.chunk_count);
            }

            let target_path =
                resolve_target_path(save_dir_path, manifest_file.relative_path.as_str());
            let part_path = build_part_path(
                save_dir_path,
                session.id.as_str(),
                manifest_file.relative_path.as_str(),
            );
            let missing = missing_chunks(bitmap.as_slice(), manifest_file.chunk_count);
            missing_chunks_payload.push(MissingChunkFrame {
                file_id: manifest_file.file_id.clone(),
                missing_chunk_indexes: missing,
            });

            let file = TransferFileDto {
                id: manifest_file.file_id,
                session_id: session.id.clone(),
                relative_path: manifest_file.relative_path,
                source_path: None,
                target_path: Some(target_path.to_string_lossy().to_string()),
                size_bytes: manifest_file.size_bytes,
                transferred_bytes: completed_bytes(
                    bitmap.as_slice(),
                    manifest_file.chunk_count,
                    manifest_file.chunk_size,
                    manifest_file.size_bytes,
                ),
                chunk_size: manifest_file.chunk_size,
                chunk_count: manifest_file.chunk_count,
                status: TransferStatus::Running,
                blake3: Some(manifest_file.blake3),
                mime_type: manifest_file.mime_type,
                preview_kind: None,
                preview_data: Some(part_path.to_string_lossy().to_string()),
                is_folder_archive: manifest_file.is_folder_archive,
            };
            self.insert_or_update_file_async(&file, bitmap.as_slice())
                .await?;
            file_id_to_idx.insert(file.id.clone(), runtimes.len());
            let writer = ChunkWriter::open(part_path.as_path(), Some(file.size_bytes)).await?;
            runtimes.push(IncomingFileRuntime {
                file: file.clone(),
                bitmap,
                writer,
            });
            session.files.push(file);
        }

        Ok(IncomingRuntimeState {
            missing_chunks_payload,
            runtimes,
            file_id_to_idx,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn process_incoming_chunk(
        session: &mut TransferSessionDto,
        runtimes: &mut [IncomingFileRuntime],
        file_id_to_idx: &HashMap<String, usize>,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        ack_buffer: &mut Vec<AckFrameItem>,
        file_id: String,
        chunk_index: u32,
        hash: String,
        payload: Vec<u8>,
        started_at: i64,
    ) -> AppResult<Option<String>> {
        let Some(file_idx) = file_id_to_idx.get(file_id.as_str()).copied() else {
            return Ok(None);
        };
        let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
            AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
        })?;
        let calculated_hash = blake3::hash(payload.as_slice()).to_hex().to_string();
        if calculated_hash != hash {
            ack_buffer.push(AckFrameItem {
                file_id,
                chunk_index,
                ok: false,
                error: Some("chunk_hash_mismatch".to_string()),
            });
            return Ok(None);
        }
        runtime
            .writer
            .write_chunk(chunk_index, runtime.file.chunk_size, payload.as_slice())
            .await?;
        if !crate::transfer::resume::is_chunk_done(runtime.bitmap.as_slice(), chunk_index) {
            mark_chunk_done(runtime.bitmap.as_mut_slice(), chunk_index)?;
            let previous = runtime.file.transferred_bytes;
            runtime.file.transferred_bytes = completed_bytes(
                runtime.bitmap.as_slice(),
                runtime.file.chunk_count,
                runtime.file.chunk_size,
                runtime.file.size_bytes,
            );
            runtime.file.status = TransferStatus::Running;
            if runtime.file.transferred_bytes > previous {
                session.transferred_bytes = session
                    .transferred_bytes
                    .saturating_add(runtime.file.transferred_bytes - previous);
            }
            session.files[file_idx] = runtime.file.clone();
            dirty_files.insert(
                runtime.file.id.clone(),
                TransferFilePersistItem {
                    file: runtime.file.clone(),
                    completed_bitmap: runtime.bitmap.clone(),
                },
            );
        }
        ack_buffer.push(AckFrameItem {
            file_id: runtime.file.id.clone(),
            chunk_index,
            ok: true,
            error: None,
        });
        session.avg_speed_bps = calculate_speed(session.transferred_bytes, started_at);
        Ok(Some(runtime.file.id.clone()))
    }

    pub(super) async fn apply_incoming_file_done(
        &self,
        session: &mut TransferSessionDto,
        runtimes: &mut [IncomingFileRuntime],
        file_id_to_idx: &HashMap<String, usize>,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        file_id: &str,
        blake3: &str,
    ) -> AppResult<bool> {
        let Some(file_idx) = file_id_to_idx.get(file_id).copied() else {
            return Ok(false);
        };
        let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
            AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
        })?;
        runtime.writer.flush().await?;
        let part_path = PathBuf::from(runtime.file.preview_data.clone().unwrap_or_default());
        let part_path_for_hash = part_path.clone();
        let source_hash = run_blocking("transfer_verify_file_hash", move || {
            file_hash_hex(part_path_for_hash.as_path())
        })
        .await?;
        if source_hash != blake3 {
            runtime.file.status = TransferStatus::Failed;
            let bitmap = empty_bitmap(runtime.file.chunk_count);
            self.insert_or_update_file_async(&runtime.file, bitmap.as_slice())
                .await?;
            return Err(AppError::new("transfer_file_hash_mismatch", "文件校验失败")
                .with_context("fileId", runtime.file.id.clone()));
        }

        let target = PathBuf::from(runtime.file.target_path.clone().unwrap_or_default());
        let final_path = resolve_conflict_path(target.as_path());
        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("创建目标目录失败: {}", parent.display()))
                .with_code("transfer_target_dir_create_failed", "创建目标目录失败")
                .with_ctx("fileId", runtime.file.id.clone())
                .with_ctx("targetDir", parent.display().to_string())?;
        }
        tokio::fs::rename(part_path.as_path(), final_path.as_path())
            .await
            .with_context(|| {
                format!(
                    "落盘文件失败: {} -> {}",
                    part_path.display(),
                    final_path.display()
                )
            })
            .with_code("transfer_target_rename_failed", "落盘文件失败")
            .with_ctx("fileId", runtime.file.id.clone())
            .with_ctx("partPath", part_path.display().to_string())
            .with_ctx("targetPath", final_path.display().to_string())?;

        runtime.file.target_path = Some(final_path.to_string_lossy().to_string());
        runtime.file.preview_data = runtime.file.target_path.clone();
        runtime.file.transferred_bytes = runtime.file.size_bytes;
        runtime.file.status = TransferStatus::Success;
        session.files[file_idx] = runtime.file.clone();
        dirty_files.insert(
            runtime.file.id.clone(),
            TransferFilePersistItem {
                file: runtime.file.clone(),
                completed_bitmap: runtime.bitmap.clone(),
            },
        );
        Ok(true)
    }

    pub(super) async fn finalize_incoming_canceled(
        &self,
        session: &mut TransferSessionDto,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        codec: FrameCodec,
    ) -> AppResult<()> {
        self.flush_progress_checkpoint(session, dirty_files, true)
            .await?;
        Self::mark_session_terminal_state(session, TransferStatus::Canceled, None, None);
        self.persist_terminal_session_state(
            session,
            TerminalPersistOptions::for_codec_with_reason(
                session.avg_speed_bps,
                codec,
                Some(TransferHistorySyncReason::IncomingCanceled),
            ),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn finalize_incoming_session_done<W>(
        &self,
        writer: &mut W,
        session: &mut TransferSessionDto,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        ack_buffer: &mut Vec<AckFrameItem>,
        ack_batch_enabled: bool,
        session_key: &[u8; 32],
        codec: FrameCodec,
        remote_ok: bool,
        remote_error: Option<String>,
    ) -> AppResult<()>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        self.flush_ack_buffer(
            writer,
            session.id.as_str(),
            ack_buffer,
            ack_batch_enabled,
            session_key,
            codec,
        )
        .await?;
        self.flush_dirty_files(dirty_files, false).await?;

        session.transferred_bytes = session
            .files
            .iter()
            .map(|value| value.transferred_bytes)
            .sum();
        let (status, error_code, error_message) =
            resolve_incoming_done_terminal(remote_ok, remote_error);
        Self::mark_session_terminal_state(session, status, error_code, error_message);
        self.persist_terminal_session_state(
            session,
            TerminalPersistOptions::for_terminal_done(
                session.avg_speed_bps,
                codec,
                None,
                None,
                TransferHistorySyncReason::IncomingDone,
            ),
        )
        .await
    }

    async fn finalize_incoming_read_failed(
        &self,
        session: &mut TransferSessionDto,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        codec: FrameCodec,
        error: &AppError,
    ) -> AppResult<()> {
        self.flush_progress_checkpoint(session, dirty_files, true)
            .await?;
        Self::mark_session_terminal_state(
            session,
            TransferStatus::Failed,
            Some(error.code.clone()),
            Some(error.message.clone()),
        );
        self.persist_terminal_session_state(
            session,
            TerminalPersistOptions::for_codec_with_reason(
                session.avg_speed_bps,
                codec,
                Some(TransferHistorySyncReason::IncomingReadFailed),
            ),
        )
        .await
    }

    async fn finalize_incoming_peer_error(
        &self,
        session: &mut TransferSessionDto,
        codec: FrameCodec,
        code: String,
        message: String,
    ) -> AppResult<()> {
        Self::mark_session_terminal_state(
            session,
            TransferStatus::Failed,
            Some(code),
            Some(message),
        );
        self.persist_terminal_session_state(session, TerminalPersistOptions::for_codec(0, codec))
            .await
    }

    pub(super) async fn poll_incoming_frame<R>(
        &self,
        reader: &mut R,
        session: &mut TransferSessionDto,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        session_key: &[u8; 32],
        codec: FrameCodec,
    ) -> AppResult<Option<TransferFrame>>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        match poll_incoming_frame_raw(reader, session_key, codec).await {
            Ok(frame) => Ok(frame),
            Err(error) => {
                self.finalize_incoming_read_failed(session, dirty_files, codec, &error)
                    .await?;
                Err(error)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn handle_incoming_frame<W>(
        &self,
        frame: TransferFrame,
        writer: &mut W,
        session: &mut TransferSessionDto,
        runtimes: &mut [IncomingFileRuntime],
        file_id_to_idx: &HashMap<String, usize>,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        ack_buffer: &mut Vec<AckFrameItem>,
        ack_batch_enabled: bool,
        session_key: &[u8; 32],
        codec: FrameCodec,
        started_at: i64,
    ) -> AppResult<bool>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        match frame {
            TransferFrame::ChunkBinary {
                session_id: incoming_session_id,
                file_id,
                chunk_index,
                hash,
                data,
                ..
            } => {
                if incoming_session_id != session.id {
                    return Ok(false);
                }
                self.handle_incoming_chunk_payload(
                    session,
                    runtimes,
                    file_id_to_idx,
                    dirty_files,
                    ack_buffer,
                    file_id,
                    chunk_index,
                    hash,
                    data,
                    started_at,
                    codec,
                )
                .await?;
                Ok(false)
            }
            TransferFrame::FileDone {
                session_id: incoming_session_id,
                file_id,
                blake3,
            } => {
                if incoming_session_id != session.id {
                    return Ok(false);
                }
                let _ = self
                    .apply_incoming_file_done(
                        session,
                        runtimes,
                        file_id_to_idx,
                        dirty_files,
                        file_id.as_str(),
                        blake3.as_str(),
                    )
                    .await?;
                Ok(false)
            }
            TransferFrame::SessionDone {
                session_id: incoming_session_id,
                ok,
                error,
            } => {
                if incoming_session_id != session.id {
                    return Ok(false);
                }
                self.finalize_incoming_session_done(
                    writer,
                    session,
                    dirty_files,
                    ack_buffer,
                    ack_batch_enabled,
                    session_key,
                    codec,
                    ok,
                    error,
                )
                .await?;
                Ok(true)
            }
            TransferFrame::Ping { .. } => Ok(false),
            TransferFrame::Error { code, message } => {
                self.finalize_incoming_peer_error(session, codec, code, message)
                    .await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_incoming_chunk_payload(
        &self,
        session: &mut TransferSessionDto,
        runtimes: &mut [IncomingFileRuntime],
        file_id_to_idx: &HashMap<String, usize>,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        ack_buffer: &mut Vec<AckFrameItem>,
        file_id: String,
        chunk_index: u32,
        hash: String,
        payload: Vec<u8>,
        started_at: i64,
        codec: FrameCodec,
    ) -> AppResult<()> {
        let Some(active_file_id) = Self::process_incoming_chunk(
            session,
            runtimes,
            file_id_to_idx,
            dirty_files,
            ack_buffer,
            file_id,
            chunk_index,
            hash,
            payload,
            started_at,
        )
        .await?
        else {
            return Ok(());
        };

        let eta = estimate_eta(
            session.total_bytes,
            session.transferred_bytes,
            session.avg_speed_bps,
        );
        self.maybe_emit_snapshot_with_codec(
            session,
            Some(active_file_id),
            session.avg_speed_bps,
            eta,
            false,
            codec,
            None,
            None,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_flush_ack_buffer_should_return_false_for_empty_buffer() {
        assert!(!should_flush_ack_buffer(
            0,
            32,
            Duration::from_millis(100),
            Duration::from_millis(50),
        ));
    }

    #[test]
    fn should_flush_ack_buffer_should_return_true_when_batch_threshold_reached() {
        assert!(should_flush_ack_buffer(
            32,
            32,
            Duration::from_millis(1),
            Duration::from_millis(50),
        ));
    }

    #[test]
    fn should_flush_ack_buffer_should_return_true_when_flush_interval_elapsed() {
        assert!(should_flush_ack_buffer(
            1,
            32,
            Duration::from_millis(51),
            Duration::from_millis(50),
        ));
    }

    #[test]
    fn should_flush_incoming_checkpoint_should_return_false_before_interval() {
        assert!(!should_flush_incoming_checkpoint(
            Duration::from_millis(99),
            Duration::from_millis(100),
        ));
    }

    #[test]
    fn should_flush_incoming_checkpoint_should_return_true_at_interval() {
        assert!(should_flush_incoming_checkpoint(
            Duration::from_millis(100),
            Duration::from_millis(100),
        ));
    }

    #[test]
    fn resolve_incoming_done_terminal_should_map_success() {
        let (status, error_code, error_message) =
            resolve_incoming_done_terminal(true, Some("ignored".to_string()));
        assert_eq!(status, TransferStatus::Success);
        assert_eq!(error_code, None);
        assert_eq!(error_message, None);
    }

    #[test]
    fn resolve_incoming_done_terminal_should_map_remote_failed() {
        let (status, error_code, error_message) =
            resolve_incoming_done_terminal(false, Some("peer failed".to_string()));
        assert_eq!(status, TransferStatus::Failed);
        assert_eq!(error_code, Some("remote_failed".to_string()));
        assert_eq!(error_message, Some("peer failed".to_string()));
    }

    #[tokio::test]
    async fn poll_incoming_frame_raw_should_return_none_on_timeout() {
        let (mut reader, _writer) = tokio::io::duplex(4096);
        let session_key = [7u8; 32];

        let frame = poll_incoming_frame_raw(&mut reader, &session_key, FrameCodec::Bin)
            .await
            .expect("timeout should not fail");
        assert!(frame.is_none());
    }

    #[tokio::test]
    async fn poll_incoming_frame_raw_should_parse_incoming_frame() {
        let (mut reader, mut writer) = tokio::io::duplex(4096);
        let session_key = [2u8; 32];
        write_frame_to(
            &mut writer,
            &TransferFrame::Ping { ts: 42 },
            Some(&session_key),
            FrameCodec::Bin,
        )
        .await
        .expect("write ping frame");

        let frame = poll_incoming_frame_raw(&mut reader, &session_key, FrameCodec::Bin)
            .await
            .expect("read ping frame");
        assert!(matches!(frame, Some(TransferFrame::Ping { ts: 42 })));
    }

    #[tokio::test]
    async fn poll_incoming_frame_raw_should_map_connection_closed_error() {
        let (mut reader, writer) = tokio::io::duplex(4096);
        drop(writer);
        let session_key = [3u8; 32];

        let result = poll_incoming_frame_raw(&mut reader, &session_key, FrameCodec::Bin).await;
        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_connection_closed");
    }
}

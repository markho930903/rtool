use super::pipeline::{TerminalPersistOptions, TransferHistorySyncReason};
use super::*;

#[derive(Debug)]
pub(super) struct OutgoingFileRuntime {
    pub(super) file: TransferFileDto,
    pub(super) bitmap: Vec<u8>,
    pub(super) reader: ChunkReader,
    pub(super) remaining_chunks: u32,
    pub(super) file_done_sent: bool,
}

#[derive(Debug)]
pub(super) struct InflightChunk {
    pub(super) file_idx: usize,
    pub(super) chunk_index: u32,
    pub(super) sent_at: Instant,
    pub(super) retries: u8,
}

#[derive(Debug)]
pub(super) struct OutgoingRuntimeState {
    pub(super) runtimes: Vec<OutgoingFileRuntime>,
    pub(super) file_id_to_idx: HashMap<String, usize>,
    pub(super) fair_queue: VecDeque<(usize, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OutgoingSnapshotMetrics {
    pub(super) eta_seconds: Option<u64>,
    pub(super) inflight_chunks: u32,
    pub(super) retransmit_chunks: u32,
}

#[derive(Debug)]
pub(super) struct OutgoingLoopState {
    pub(super) session: TransferSessionDto,
    pub(super) runtimes: Vec<OutgoingFileRuntime>,
    pub(super) file_id_to_idx: HashMap<String, usize>,
    pub(super) fair_queue: VecDeque<(usize, u32)>,
    pub(super) inflight: HashMap<(usize, u32), InflightChunk>,
    pub(super) retry_counts: HashMap<(usize, u32), u8>,
    pub(super) retransmit_chunks: u32,
    pub(super) dirty_files: HashMap<String, TransferFilePersistItem>,
    pub(super) start_at: i64,
}

impl OutgoingLoopState {
    pub(super) fn from_parts(
        session: TransferSessionDto,
        runtime_state: OutgoingRuntimeState,
        start_at: i64,
    ) -> Self {
        Self {
            session,
            runtimes: runtime_state.runtimes,
            file_id_to_idx: runtime_state.file_id_to_idx,
            fair_queue: runtime_state.fair_queue,
            inflight: HashMap::new(),
            retry_counts: HashMap::new(),
            retransmit_chunks: 0,
            dirty_files: HashMap::new(),
            start_at,
        }
    }

    pub(super) fn has_pending_work(&self) -> bool {
        !self.fair_queue.is_empty() || !self.inflight.is_empty()
    }

    pub(super) fn snapshot(&self) -> OutgoingSnapshotMetrics {
        build_outgoing_snapshot(&self.session, self.inflight.len(), self.retransmit_chunks)
    }
}

pub(super) fn should_flush_outgoing_checkpoint(elapsed: Duration, interval: Duration) -> bool {
    elapsed >= interval
}

pub(super) fn build_outgoing_snapshot(
    session: &TransferSessionDto,
    inflight_len: usize,
    retransmit_chunks: u32,
) -> OutgoingSnapshotMetrics {
    OutgoingSnapshotMetrics {
        eta_seconds: estimate_eta(
            session.total_bytes,
            session.transferred_bytes,
            session.avg_speed_bps,
        ),
        inflight_chunks: inflight_len as u32,
        retransmit_chunks,
    }
}

impl TransferService {
    fn seed_fair_queue(per_file_missing: &mut [VecDeque<u32>]) -> VecDeque<(usize, u32)> {
        let mut fair_queue = VecDeque::<(usize, u32)>::new();
        loop {
            let mut progressed = false;
            for (idx, queue) in per_file_missing.iter_mut().enumerate() {
                if let Some(chunk_index) = queue.pop_front() {
                    fair_queue.push_back((idx, chunk_index));
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }
        fair_queue
    }

    pub(super) async fn build_outgoing_runtime_state(
        &self,
        session: &mut TransferSessionDto,
        missing_by_file: &HashMap<String, Vec<u32>>,
    ) -> AppResult<OutgoingRuntimeState> {
        let mut runtimes = Vec::<OutgoingFileRuntime>::new();
        let mut file_id_to_idx = HashMap::<String, usize>::new();
        let mut per_file_missing = Vec::<VecDeque<u32>>::new();

        for (index, file) in session.files.iter_mut().enumerate() {
            let bitmap = self
                .get_file_bitmap_async(session.id.as_str(), file.id.as_str())
                .await
                .unwrap_or_default()
                .unwrap_or_else(|| empty_bitmap(file.chunk_count));
            let source_path = PathBuf::from(file.source_path.clone().unwrap_or_default());
            let mut missing = missing_by_file
                .get(file.id.as_str())
                .cloned()
                .unwrap_or_else(|| missing_chunks(bitmap.as_slice(), file.chunk_count));
            missing.sort_unstable();
            file.status = TransferStatus::Running;
            file.transferred_bytes = completed_bytes(
                bitmap.as_slice(),
                file.chunk_count,
                file.chunk_size,
                file.size_bytes,
            );
            file_id_to_idx.insert(file.id.clone(), index);
            per_file_missing.push(VecDeque::from(missing.clone()));
            runtimes.push(OutgoingFileRuntime {
                file: file.clone(),
                bitmap,
                reader: ChunkReader::open(source_path.as_path()).await?,
                remaining_chunks: missing.len() as u32,
                file_done_sent: false,
            });
        }

        session.transferred_bytes = session
            .files
            .iter()
            .map(|item| item.transferred_bytes)
            .sum();

        Ok(OutgoingRuntimeState {
            runtimes,
            file_id_to_idx,
            fair_queue: Self::seed_fair_queue(per_file_missing.as_mut_slice()),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn fill_inflight_window<W>(
        writer: &mut W,
        session: &TransferSessionDto,
        session_key: &[u8; 32],
        codec: FrameCodec,
        max_inflight_chunks: usize,
        fair_queue: &mut VecDeque<(usize, u32)>,
        inflight: &mut HashMap<(usize, u32), InflightChunk>,
        retry_counts: &HashMap<(usize, u32), u8>,
        runtimes: &mut [OutgoingFileRuntime],
    ) -> AppResult<()>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        while inflight.len() < max_inflight_chunks {
            let Some((file_idx, chunk_index)) = fair_queue.pop_front() else {
                break;
            };
            if inflight.contains_key(&(file_idx, chunk_index)) {
                continue;
            }
            let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
            })?;
            if runtime.file_done_sent || runtime.remaining_chunks == 0 {
                continue;
            }
            if app_infra::transfer::resume::is_chunk_done(runtime.bitmap.as_slice(), chunk_index) {
                continue;
            }

            let bytes = runtime
                .reader
                .read_chunk(chunk_index, runtime.file.chunk_size)
                .await?;
            let hash = blake3::hash(bytes.as_slice()).to_hex().to_string();
            let frame = TransferFrame::ChunkBinary {
                session_id: session.id.clone(),
                file_id: runtime.file.id.clone(),
                chunk_index,
                total_chunks: runtime.file.chunk_count,
                hash,
                data: bytes,
            };
            write_frame_to(writer, &frame, Some(session_key), codec).await?;
            inflight.insert(
                (file_idx, chunk_index),
                InflightChunk {
                    file_idx,
                    chunk_index,
                    sent_at: Instant::now(),
                    retries: retry_counts
                        .get(&(file_idx, chunk_index))
                        .copied()
                        .unwrap_or_default(),
                },
            );
        }
        Ok(())
    }

    pub(super) async fn process_outgoing_iteration<R, W>(
        writer: &mut W,
        reader: &mut R,
        state: &mut OutgoingLoopState,
        peer_address: &str,
        session_key: &[u8; 32],
        codec: FrameCodec,
        max_inflight_chunks: usize,
    ) -> AppResult<()>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        Self::fill_inflight_window(
            writer,
            &state.session,
            session_key,
            codec,
            max_inflight_chunks,
            &mut state.fair_queue,
            &mut state.inflight,
            &state.retry_counts,
            state.runtimes.as_mut_slice(),
        )
        .await?;

        let ack_items = Self::poll_outgoing_ack_items(
            reader,
            state.session.id.as_str(),
            peer_address,
            session_key,
            codec,
        )
        .await?;
        if !ack_items.is_empty() {
            Self::apply_ack_items(
                &mut state.session,
                writer,
                session_key,
                codec,
                peer_address,
                state.start_at,
                ack_items,
                &state.file_id_to_idx,
                state.runtimes.as_mut_slice(),
                &mut state.inflight,
                &mut state.retry_counts,
                &mut state.retransmit_chunks,
                &mut state.fair_queue,
                &mut state.dirty_files,
            )
            .await?;
        }

        let timeout_chunks = Self::collect_timeout_chunks(&state.inflight);
        Self::requeue_timeout_chunks(
            &state.session,
            peer_address,
            timeout_chunks,
            &mut state.inflight,
            &mut state.retry_counts,
            &mut state.retransmit_chunks,
            &mut state.fair_queue,
        )?;

        Ok(())
    }

    pub(super) async fn maybe_flush_outgoing_checkpoint(
        &self,
        state: &mut OutgoingLoopState,
        last_db_flush: &mut Instant,
        db_flush_interval: Duration,
    ) -> AppResult<()> {
        if !should_flush_outgoing_checkpoint(last_db_flush.elapsed(), db_flush_interval) {
            return Ok(());
        }
        self.flush_progress_checkpoint(&state.session, &mut state.dirty_files, true)
            .await?;
        *last_db_flush = Instant::now();
        Ok(())
    }

    pub(super) async fn finalize_outgoing_canceled(
        &self,
        state: &mut OutgoingLoopState,
        codec: FrameCodec,
    ) -> AppResult<()> {
        self.flush_progress_checkpoint(&state.session, &mut state.dirty_files, true)
            .await?;
        Self::mark_session_terminal_state(&mut state.session, TransferStatus::Canceled, None, None);
        self.persist_terminal_session_state(
            &state.session,
            TerminalPersistOptions::for_codec_with_reason(
                state.session.avg_speed_bps,
                codec,
                Some(TransferHistorySyncReason::OutgoingCanceled),
            )
            .with_inflight(Some(state.inflight.len() as u32))
            .with_retransmit(Some(state.retransmit_chunks)),
        )
        .await
    }

    pub(super) async fn finalize_outgoing_success<W>(
        &self,
        writer: &mut W,
        state: &mut OutgoingLoopState,
        session_key: &[u8; 32],
        codec: FrameCodec,
    ) -> AppResult<()>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        self.flush_dirty_files(&mut state.dirty_files, false)
            .await?;

        write_frame_to(
            writer,
            &TransferFrame::SessionDone {
                session_id: state.session.id.clone(),
                ok: true,
                error: None,
            },
            Some(session_key),
            codec,
        )
        .await?;

        state.session.transferred_bytes = state.session.total_bytes;
        state.session.avg_speed_bps =
            calculate_speed(state.session.transferred_bytes, state.start_at);
        Self::mark_session_terminal_state(&mut state.session, TransferStatus::Success, None, None);
        self.persist_terminal_session_state(
            &state.session,
            TerminalPersistOptions::for_terminal_done(
                state.session.avg_speed_bps,
                codec,
                Some(0),
                Some(state.retransmit_chunks),
                TransferHistorySyncReason::SessionDone,
            ),
        )
        .await
    }

    pub(super) fn collect_ack_items_from_frame(
        frame: TransferFrame,
        session_id: &str,
        peer_address: &str,
    ) -> AppResult<Vec<AckFrameItem>> {
        let mut ack_items = Vec::<AckFrameItem>::new();
        match frame {
            TransferFrame::Ack {
                session_id: ack_session_id,
                file_id,
                chunk_index,
                ok,
                error,
            } if ack_session_id == session_id => {
                ack_items.push(AckFrameItem {
                    file_id,
                    chunk_index,
                    ok,
                    error,
                });
            }
            TransferFrame::AckBatch {
                session_id: ack_session_id,
                items,
            } if ack_session_id == session_id => {
                ack_items.extend(items);
            }
            TransferFrame::Error { code, message } => {
                return Err(AppError::new(code, "目标设备返回错误")
                    .with_cause(message)
                    .with_context("sessionId", session_id.to_string())
                    .with_context("peerAddress", peer_address.to_string()));
            }
            TransferFrame::Ping { .. } => {}
            _ => {}
        }
        Ok(ack_items)
    }

    pub(super) async fn poll_outgoing_ack_items<R>(
        reader: &mut R,
        session_id: &str,
        peer_address: &str,
        session_key: &[u8; 32],
        codec: FrameCodec,
    ) -> AppResult<Vec<AckFrameItem>>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        match tokio::time::timeout(
            Duration::from_millis(40),
            read_frame_from(reader, Some(session_key), Some(codec)),
        )
        .await
        {
            Ok(Ok(frame)) => Self::collect_ack_items_from_frame(frame, session_id, peer_address),
            Ok(Err(error)) => Err(error),
            Err(_) => Ok(Vec::new()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn apply_ack_items<W>(
        session: &mut TransferSessionDto,
        writer: &mut W,
        session_key: &[u8; 32],
        codec: FrameCodec,
        peer_address: &str,
        start_at: i64,
        ack_items: Vec<AckFrameItem>,
        file_id_to_idx: &HashMap<String, usize>,
        runtimes: &mut [OutgoingFileRuntime],
        inflight: &mut HashMap<(usize, u32), InflightChunk>,
        retry_counts: &mut HashMap<(usize, u32), u8>,
        retransmit_chunks: &mut u32,
        fair_queue: &mut VecDeque<(usize, u32)>,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
    ) -> AppResult<()>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        for ack in ack_items {
            let Some(file_idx) = file_id_to_idx.get(ack.file_id.as_str()).copied() else {
                continue;
            };
            let key = (file_idx, ack.chunk_index);
            let Some(inflight_chunk) = inflight.remove(&key) else {
                continue;
            };
            if ack.ok {
                retry_counts.remove(&key);
                let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                    AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                })?;
                if !app_infra::transfer::resume::is_chunk_done(
                    runtime.bitmap.as_slice(),
                    ack.chunk_index,
                ) {
                    mark_chunk_done(runtime.bitmap.as_mut_slice(), ack.chunk_index)?;
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
                    if runtime.remaining_chunks > 0 {
                        runtime.remaining_chunks -= 1;
                    }

                    session.avg_speed_bps = calculate_speed(session.transferred_bytes, start_at);
                    session.files[file_idx] = runtime.file.clone();
                    dirty_files.insert(
                        runtime.file.id.clone(),
                        TransferFilePersistItem {
                            file: runtime.file.clone(),
                            completed_bitmap: runtime.bitmap.clone(),
                        },
                    );
                }

                if runtime.remaining_chunks == 0 && !runtime.file_done_sent {
                    runtime.file.status = TransferStatus::Success;
                    runtime.file.transferred_bytes = runtime.file.size_bytes;
                    session.files[file_idx] = runtime.file.clone();
                    dirty_files.insert(
                        runtime.file.id.clone(),
                        TransferFilePersistItem {
                            file: runtime.file.clone(),
                            completed_bitmap: runtime.bitmap.clone(),
                        },
                    );
                    write_frame_to(
                        writer,
                        &TransferFrame::FileDone {
                            session_id: session.id.clone(),
                            file_id: runtime.file.id.clone(),
                            blake3: runtime.file.blake3.clone().unwrap_or_default(),
                        },
                        Some(session_key),
                        codec,
                    )
                    .await?;
                    runtime.file_done_sent = true;
                }
            } else {
                let retry = inflight_chunk.retries.saturating_add(1);
                if retry > MAX_CHUNK_RETRY {
                    return Err(AppError::new(
                        "transfer_chunk_retry_exhausted",
                        "分块重试次数已耗尽",
                    )
                    .with_context("sessionId", session.id.clone())
                    .with_context("fileId", ack.file_id.clone())
                    .with_context("fileIdx", inflight_chunk.file_idx.to_string())
                    .with_context("chunkIndex", inflight_chunk.chunk_index.to_string())
                    .with_context("peerAddress", peer_address.to_string()));
                }
                *retransmit_chunks = retransmit_chunks.saturating_add(1);
                tracing::warn!(
                    event = "transfer_chunk_requeue_failed_ack",
                    session_id = session.id,
                    file_id = ack.file_id,
                    chunk_index = ack.chunk_index,
                    retry
                );
                retry_counts.insert(key, retry);
                fair_queue.push_front(key);
            }
        }
        Ok(())
    }

    pub(super) fn collect_timeout_chunks(
        inflight: &HashMap<(usize, u32), InflightChunk>,
    ) -> Vec<(usize, u32)> {
        let mut timeout_chunks = Vec::new();
        for (key, value) in inflight {
            if value.sent_at.elapsed() >= Duration::from_millis(CHUNK_ACK_TIMEOUT_MS) {
                timeout_chunks.push(*key);
            }
        }
        timeout_chunks
    }

    pub(super) fn requeue_timeout_chunks(
        session: &TransferSessionDto,
        peer_address: &str,
        timeout_chunks: Vec<(usize, u32)>,
        inflight: &mut HashMap<(usize, u32), InflightChunk>,
        retry_counts: &mut HashMap<(usize, u32), u8>,
        retransmit_chunks: &mut u32,
        fair_queue: &mut VecDeque<(usize, u32)>,
    ) -> AppResult<()> {
        for key in timeout_chunks {
            if let Some(old) = inflight.remove(&key) {
                let retry = old.retries.saturating_add(1);
                if retry > MAX_CHUNK_RETRY {
                    return Err(AppError::new(
                        "transfer_chunk_ack_timeout",
                        "分块确认超时且超过重试上限",
                    )
                    .with_context("sessionId", session.id.clone())
                    .with_context("fileIdx", key.0.to_string())
                    .with_context("chunkIndex", key.1.to_string())
                    .with_context("peerAddress", peer_address.to_string()));
                }
                *retransmit_chunks = retransmit_chunks.saturating_add(1);
                tracing::warn!(
                    event = "transfer_chunk_requeue_timeout",
                    session_id = session.id,
                    file_idx = key.0,
                    chunk_index = key.1,
                    retry
                );
                retry_counts.insert(key, retry);
                fair_queue.push_front(key);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_running_session() -> TransferSessionDto {
        TransferSessionDto {
            id: "session-1".to_string(),
            direction: TransferDirection::Send,
            peer_device_id: "peer-1".to_string(),
            peer_name: "Peer".to_string(),
            status: TransferStatus::Running,
            total_bytes: 100,
            transferred_bytes: 40,
            avg_speed_bps: 10,
            save_dir: "/tmp".to_string(),
            created_at: 0,
            started_at: Some(0),
            finished_at: None,
            error_code: None,
            error_message: None,
            cleanup_after_at: None,
            files: Vec::new(),
        }
    }

    #[test]
    fn should_flush_outgoing_checkpoint_should_return_false_before_interval() {
        assert!(!should_flush_outgoing_checkpoint(
            Duration::from_millis(99),
            Duration::from_millis(100),
        ));
    }

    #[test]
    fn should_flush_outgoing_checkpoint_should_return_true_at_interval() {
        assert!(should_flush_outgoing_checkpoint(
            Duration::from_millis(100),
            Duration::from_millis(100),
        ));
    }

    #[test]
    fn should_flush_outgoing_checkpoint_should_return_true_after_interval() {
        assert!(should_flush_outgoing_checkpoint(
            Duration::from_millis(101),
            Duration::from_millis(100),
        ));
    }

    #[test]
    fn build_outgoing_snapshot_should_include_eta_and_counters() {
        let session = sample_running_session();
        let snapshot = build_outgoing_snapshot(&session, 3, 7);
        assert_eq!(
            snapshot,
            OutgoingSnapshotMetrics {
                eta_seconds: Some(6),
                inflight_chunks: 3,
                retransmit_chunks: 7,
            }
        );
    }

    #[test]
    fn build_outgoing_snapshot_should_hide_eta_when_speed_is_zero() {
        let mut session = sample_running_session();
        session.avg_speed_bps = 0;
        let snapshot = build_outgoing_snapshot(&session, 1, 0);
        assert_eq!(snapshot.eta_seconds, None);
    }

    #[test]
    fn outgoing_loop_state_has_pending_work_should_reflect_queue_or_inflight() {
        let mut state = OutgoingLoopState::from_parts(
            sample_running_session(),
            OutgoingRuntimeState {
                runtimes: Vec::new(),
                file_id_to_idx: HashMap::new(),
                fair_queue: VecDeque::new(),
            },
            0,
        );
        assert!(!state.has_pending_work());

        state.fair_queue.push_back((0, 1));
        assert!(state.has_pending_work());

        state.fair_queue.clear();
        state.inflight.insert(
            (0, 1),
            InflightChunk {
                file_idx: 0,
                chunk_index: 1,
                sent_at: Instant::now(),
                retries: 0,
            },
        );
        assert!(state.has_pending_work());
    }

    #[test]
    fn outgoing_loop_state_snapshot_should_use_session_and_runtime_counters() {
        let mut state = OutgoingLoopState::from_parts(
            sample_running_session(),
            OutgoingRuntimeState {
                runtimes: Vec::new(),
                file_id_to_idx: HashMap::new(),
                fair_queue: VecDeque::new(),
            },
            0,
        );
        state.inflight.insert(
            (0, 1),
            InflightChunk {
                file_idx: 0,
                chunk_index: 1,
                sent_at: Instant::now(),
                retries: 0,
            },
        );
        state.retransmit_chunks = 9;

        let snapshot = state.snapshot();
        assert_eq!(snapshot.inflight_chunks, 1);
        assert_eq!(snapshot.retransmit_chunks, 9);
        assert_eq!(snapshot.eta_seconds, Some(6));
    }

    #[tokio::test]
    async fn poll_outgoing_ack_items_should_return_empty_on_timeout() {
        let (mut reader, _writer) = tokio::io::duplex(4096);
        let session_key = [7u8; 32];

        let result = TransferService::poll_outgoing_ack_items(
            &mut reader,
            "session-1",
            "127.0.0.1:9527",
            &session_key,
            FrameCodec::Bin,
        )
        .await
        .expect("timeout should not fail");

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn poll_outgoing_ack_items_should_parse_single_ack_frame() {
        let (mut reader, mut writer) = tokio::io::duplex(4096);
        let session_key = [9u8; 32];
        write_frame_to(
            &mut writer,
            &TransferFrame::Ack {
                session_id: "session-1".to_string(),
                file_id: "file-1".to_string(),
                chunk_index: 3,
                ok: true,
                error: None,
            },
            Some(&session_key),
            FrameCodec::Bin,
        )
        .await
        .expect("write ack frame");

        let result = TransferService::poll_outgoing_ack_items(
            &mut reader,
            "session-1",
            "127.0.0.1:9527",
            &session_key,
            FrameCodec::Bin,
        )
        .await
        .expect("poll ack frame");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_id, "file-1");
        assert_eq!(result[0].chunk_index, 3);
        assert!(result[0].ok);
    }

    #[tokio::test]
    async fn poll_outgoing_ack_items_should_ignore_mismatched_session_ack_frame() {
        let (mut reader, mut writer) = tokio::io::duplex(4096);
        let session_key = [5u8; 32];
        write_frame_to(
            &mut writer,
            &TransferFrame::Ack {
                session_id: "session-other".to_string(),
                file_id: "file-1".to_string(),
                chunk_index: 4,
                ok: true,
                error: None,
            },
            Some(&session_key),
            FrameCodec::Bin,
        )
        .await
        .expect("write mismatched ack frame");

        let result = TransferService::poll_outgoing_ack_items(
            &mut reader,
            "session-1",
            "127.0.0.1:9527",
            &session_key,
            FrameCodec::Bin,
        )
        .await
        .expect("poll mismatched ack frame");

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn poll_outgoing_ack_items_should_map_peer_error_frame() {
        let (mut reader, mut writer) = tokio::io::duplex(4096);
        let session_key = [3u8; 32];
        write_frame_to(
            &mut writer,
            &TransferFrame::Error {
                code: "transfer_peer_failed".to_string(),
                message: "peer failed".to_string(),
            },
            Some(&session_key),
            FrameCodec::Bin,
        )
        .await
        .expect("write error frame");

        let result = TransferService::poll_outgoing_ack_items(
            &mut reader,
            "session-1",
            "127.0.0.1:9527",
            &session_key,
            FrameCodec::Bin,
        )
        .await;

        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_peer_failed");
        assert_eq!(error.message, "目标设备返回错误");
    }

    #[test]
    fn collect_ack_items_from_frame_should_collect_single_ack() {
        let ack_items = TransferService::collect_ack_items_from_frame(
            TransferFrame::Ack {
                session_id: "session-1".to_string(),
                file_id: "file-1".to_string(),
                chunk_index: 2,
                ok: true,
                error: None,
            },
            "session-1",
            "127.0.0.1:9527",
        )
        .expect("ack frame should parse");

        assert_eq!(ack_items.len(), 1);
        assert_eq!(ack_items[0].file_id, "file-1");
        assert_eq!(ack_items[0].chunk_index, 2);
        assert!(ack_items[0].ok);
    }

    #[test]
    fn collect_ack_items_from_frame_should_collect_batch_ack() {
        let ack_items = TransferService::collect_ack_items_from_frame(
            TransferFrame::AckBatch {
                session_id: "session-1".to_string(),
                items: vec![
                    AckFrameItem {
                        file_id: "file-1".to_string(),
                        chunk_index: 1,
                        ok: true,
                        error: None,
                    },
                    AckFrameItem {
                        file_id: "file-2".to_string(),
                        chunk_index: 3,
                        ok: false,
                        error: Some("chunk_hash_mismatch".to_string()),
                    },
                ],
            },
            "session-1",
            "127.0.0.1:9527",
        )
        .expect("ack batch frame should parse");

        assert_eq!(ack_items.len(), 2);
        assert_eq!(ack_items[1].file_id, "file-2");
        assert!(!ack_items[1].ok);
    }

    #[test]
    fn collect_ack_items_from_frame_should_ignore_mismatched_session_ack() {
        let ack_items = TransferService::collect_ack_items_from_frame(
            TransferFrame::Ack {
                session_id: "session-other".to_string(),
                file_id: "file-1".to_string(),
                chunk_index: 2,
                ok: true,
                error: None,
            },
            "session-1",
            "127.0.0.1:9527",
        )
        .expect("mismatched session ack should be ignored");

        assert!(ack_items.is_empty());
    }

    #[test]
    fn collect_ack_items_from_frame_should_map_peer_error_frame() {
        let result = TransferService::collect_ack_items_from_frame(
            TransferFrame::Error {
                code: "transfer_peer_failed".to_string(),
                message: "peer failed".to_string(),
            },
            "session-1",
            "127.0.0.1:9527",
        );

        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_peer_failed");
        assert_eq!(error.message, "目标设备返回错误");
    }
}

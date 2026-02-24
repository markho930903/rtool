use super::*;

struct IncomingSessionControlGuard {
    service: TransferService,
    session_id: String,
}

impl IncomingSessionControlGuard {
    fn new(service: &TransferService, session_id: String) -> Self {
        service.register_session_control(session_id.as_str());
        Self {
            service: service.clone(),
            session_id,
        }
    }
}

impl Drop for IncomingSessionControlGuard {
    fn drop(&mut self) {
        self.service
            .unregister_session_control(self.session_id.as_str());
    }
}

impl TransferService {
    pub(super) async fn handle_incoming(&self, mut stream: TcpStream) -> AppResult<()> {
        let settings = self.get_settings();
        let handshake = self
            .perform_incoming_handshake(&mut stream, &settings)
            .await?;
        let codec = handshake.codec;
        let ack_batch_enabled = handshake.ack_batch_enabled;
        let session_key = handshake.session_key;
        let (mut reader, mut writer) = stream.into_split();

        let manifest = Self::read_incoming_manifest_stage(&mut reader, &session_key, codec).await?;

        let cleanup_after_at = now_millis() + i64::from(settings.auto_cleanup_days) * 86_400_000;
        let total_bytes = manifest
            .files
            .iter()
            .map(|value| value.size_bytes)
            .sum::<u64>();

        let mut session = TransferSessionDto {
            id: manifest.session_id.clone(),
            direction: TransferDirection::from_remote_manifest(manifest.direction.as_str())?,
            peer_device_id: handshake.peer_device_id,
            peer_name: handshake.peer_name,
            status: TransferStatus::Running,
            total_bytes,
            transferred_bytes: 0,
            avg_speed_bps: 0,
            save_dir: manifest.save_dir.clone(),
            created_at: now_millis(),
            started_at: Some(now_millis()),
            finished_at: None,
            error_code: None,
            error_message: None,
            cleanup_after_at: Some(cleanup_after_at),
            files: Vec::new(),
        };
        self.blocking_upsert_session_progress(session.clone())
            .await?;
        let _session_control_guard = IncomingSessionControlGuard::new(self, session.id.clone());

        let save_dir_path = PathBuf::from(settings.default_download_dir);
        let runtime_state = self
            .build_incoming_runtime_state(&mut session, manifest.files, save_dir_path.as_path())
            .await?;
        let mut runtimes = runtime_state.runtimes;
        let file_id_to_idx = runtime_state.file_id_to_idx;
        let missing_chunks_payload = runtime_state.missing_chunks_payload;

        write_frame_to(
            &mut writer,
            &TransferFrame::ManifestAck {
                session_id: session.id.clone(),
                missing_chunks: missing_chunks_payload,
            },
            Some(&session_key),
            codec,
        )
        .await?;

        let started_at = now_millis();
        let mut ack_buffer = Vec::<AckFrameItem>::new();
        let mut last_ack_flush = Instant::now();
        let ack_flush_interval =
            Duration::from_millis(settings.ack_flush_interval_ms.max(5) as u64);
        let db_flush_interval =
            Duration::from_millis(settings.db_flush_interval_ms.max(100) as u64);
        let mut last_db_flush = Instant::now();
        let mut dirty_files = HashMap::<String, TransferFilePersistItem>::new();

        loop {
            self.wait_if_paused(session.id.as_str()).await;
            if self.is_session_canceled(session.id.as_str()) {
                self.finalize_incoming_canceled(&mut session, &mut dirty_files, codec)
                    .await?;
                return Err(AppError::new(TRANSFER_SESSION_CANCELED_CODE, "传输已取消")
                    .with_context("sessionId", session.id.clone()));
            }

            let frame = self
                .poll_incoming_frame(
                    &mut reader,
                    &mut session,
                    &mut dirty_files,
                    &session_key,
                    codec,
                )
                .await?;

            if let Some(frame) = frame {
                let should_break = self
                    .handle_incoming_frame(
                        frame,
                        &mut writer,
                        &mut session,
                        runtimes.as_mut_slice(),
                        &file_id_to_idx,
                        &mut dirty_files,
                        &mut ack_buffer,
                        ack_batch_enabled,
                        &session_key,
                        codec,
                        started_at,
                    )
                    .await?;
                if should_break {
                    break;
                }
            }

            if incoming_pipeline::should_flush_ack_buffer(
                ack_buffer.len(),
                settings.ack_batch_size,
                last_ack_flush.elapsed(),
                ack_flush_interval,
            ) {
                self.flush_ack_buffer(
                    &mut writer,
                    session.id.as_str(),
                    &mut ack_buffer,
                    ack_batch_enabled,
                    &session_key,
                    codec,
                )
                .await?;
                last_ack_flush = Instant::now();
            }

            if incoming_pipeline::should_flush_incoming_checkpoint(
                last_db_flush.elapsed(),
                db_flush_interval,
            ) {
                self.flush_progress_checkpoint(&session, &mut dirty_files, true)
                    .await?;
                last_db_flush = Instant::now();
            }
        }

        Ok(())
    }
}

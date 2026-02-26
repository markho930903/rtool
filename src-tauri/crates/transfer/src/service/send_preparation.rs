use super::*;
use app_infra::transfer::archive::TransferSourceBundle;

fn should_keep_pair_code_for_retry(error: Option<&AppError>) -> bool {
    matches!(error, Some(value) if value.code != TRANSFER_SESSION_CANCELED_CODE)
}

pub(super) struct PreparedOutgoingSend {
    pub session: TransferSessionDto,
    pub peer_address: String,
    pub pair_code: String,
    pub temp_paths: Vec<PathBuf>,
}

impl TransferService {
    pub(super) async fn prepare_outgoing_send(
        &self,
        input: TransferSendFilesInputDto,
    ) -> AppResult<PreparedOutgoingSend> {
        let peers = self.list_peers().await?;
        let peer = peers
            .into_iter()
            .find(|value| value.device_id == input.peer_device_id)
            .ok_or_else(|| {
                AppError::new("transfer_peer_not_found", "未找到目标设备")
                    .with_context("peerDeviceId", input.peer_device_id.clone())
            })?;
        Self::ensure_peer_not_blocked(&peer)?;

        let session_id = input
            .session_id
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let settings = self.get_settings();
        let bundle = self.collect_send_sources(input.files).await?;
        let (files, total_bytes) = self
            .build_manifest_files(session_id.as_str(), &settings, bundle.files)
            .await?;

        let cleanup_after_at = now_millis() + i64::from(settings.auto_cleanup_days) * 86_400_000;
        let session = TransferSessionDto {
            id: session_id,
            direction: input.direction.unwrap_or(TransferDirection::Send),
            peer_device_id: peer.device_id,
            peer_name: peer.display_name,
            status: TransferStatus::Queued,
            total_bytes,
            transferred_bytes: 0,
            avg_speed_bps: 0,
            rtt_ms_p50: None,
            rtt_ms_p95: None,
            save_dir: settings.default_download_dir,
            created_at: now_millis(),
            started_at: None,
            finished_at: None,
            error_code: None,
            error_message: None,
            cleanup_after_at: Some(cleanup_after_at),
            files,
        };

        Ok(PreparedOutgoingSend {
            session,
            peer_address: format!("{}:{}", peer.address, peer.listen_port),
            pair_code: input.pair_code,
            temp_paths: bundle.temp_paths,
        })
    }

    pub(super) async fn persist_new_session_with_files(
        &self,
        session: &TransferSessionDto,
    ) -> AppResult<()> {
        insert_session(&self.db_conn, session).await?;
        for file in &session.files {
            let bitmap = empty_bitmap(file.chunk_count);
            insert_or_update_file(&self.db_conn, file, bitmap.as_slice()).await?;
        }
        Ok(())
    }

    pub(super) fn attach_outgoing_session_runtime(&self, session_id: &str, pair_code: &str) {
        self.register_session_control(session_id);
        write_lock(self.session_pair_codes.as_ref(), "session_pair_codes")
            .insert(session_id.to_string(), pair_code.to_string());
    }

    pub(super) fn spawn_outgoing_worker(
        &self,
        session_id: String,
        peer_address: String,
        pair_code: String,
        temp_paths: Vec<PathBuf>,
    ) -> AppResult<()> {
        let service = self.clone();
        self.spawn_task("transfer_outgoing_worker", async move {
            let run_result = service
                .run_outgoing_session(
                    session_id.as_str(),
                    peer_address.as_str(),
                    pair_code.as_str(),
                )
                .await;
            if let Err(error) = run_result.as_ref() {
                if error.code == TRANSFER_SESSION_CANCELED_CODE {
                    tracing::info!(event = "transfer_send_canceled", session_id = session_id,);
                } else {
                    tracing::error!(
                        event = "transfer_send_failed",
                        session_id = session_id,
                        error_code = error.code,
                        error_detail = error.causes.first().map(String::as_str).unwrap_or_default()
                    );
                    let _ = service
                        .update_session_failure(session_id.as_str(), error)
                        .await;
                }
            }

            if !should_keep_pair_code_for_retry(run_result.as_ref().err()) {
                write_lock(service.session_pair_codes.as_ref(), "session_pair_codes")
                    .remove(session_id.as_str());
            }

            let _ = run_blocking("transfer_cleanup_temp_paths", move || {
                cleanup_temp_paths(temp_paths.as_slice());
                Ok(())
            })
            .await;
            service.unregister_session_control(session_id.as_str());
        })?;
        Ok(())
    }

    async fn collect_send_sources(
        &self,
        input_files: Vec<TransferFileInputDto>,
    ) -> AppResult<TransferSourceBundle> {
        run_blocking("transfer_collect_sources", move || {
            collect_sources(input_files.as_slice())
        })
        .await
    }

    async fn build_manifest_files(
        &self,
        session_id: &str,
        settings: &TransferSettingsDto,
        sources: Vec<app_infra::transfer::archive::TransferSourceFile>,
    ) -> AppResult<(Vec<TransferFileDto>, u64)> {
        let session_id_for_manifest = session_id.to_string();
        let settings_for_manifest = settings.clone();
        run_blocking("transfer_prepare_manifest_files", move || {
            let chunk_size = settings_for_manifest.chunk_size_kb.saturating_mul(1024);
            let mut files = Vec::with_capacity(sources.len());
            let mut total_bytes = 0u64;
            for source in sources {
                let source_path = PathBuf::from(source.source_path.as_str());
                let hash = file_hash_hex(source_path.as_path())?;
                let chunk_count = chunk_count(source.size_bytes, chunk_size);
                let (mime_type, preview_kind, preview_data) = build_preview(source_path.as_path());
                let file_dto = TransferFileDto {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: session_id_for_manifest.clone(),
                    relative_path: source.relative_path,
                    source_path: Some(source.source_path),
                    target_path: None,
                    size_bytes: source.size_bytes,
                    transferred_bytes: 0,
                    chunk_size,
                    chunk_count,
                    status: TransferStatus::Queued,
                    blake3: Some(hash),
                    mime_type,
                    preview_kind,
                    preview_data,
                    is_folder_archive: source.is_folder_archive,
                };
                total_bytes = total_bytes.saturating_add(file_dto.size_bytes);
                files.push(file_dto);
            }

            Ok((files, total_bytes))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_pair_code_for_retry_should_keep_for_non_cancel_error() {
        let keep = should_keep_pair_code_for_retry(Some(&AppError::new(
            "transfer_peer_connect_failed",
            "connect failed",
        )));
        assert!(keep);
    }

    #[test]
    fn should_keep_pair_code_for_retry_should_drop_for_cancel_error() {
        let keep = should_keep_pair_code_for_retry(Some(&AppError::new(
            TRANSFER_SESSION_CANCELED_CODE,
            "canceled",
        )));
        assert!(!keep);
    }

    #[test]
    fn should_keep_pair_code_for_retry_should_drop_when_success() {
        let keep = should_keep_pair_code_for_retry(None);
        assert!(!keep);
    }
}

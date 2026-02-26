use super::*;

fn ensure_outgoing_session_active(
    session_id: &str,
    session_status: TransferStatus,
    canceled: bool,
) -> AppResult<()> {
    if canceled || matches!(session_status, TransferStatus::Canceled) {
        return Err(AppError::new(TRANSFER_SESSION_CANCELED_CODE, "传输已取消")
            .with_context("sessionId", session_id.to_string()));
    }
    Ok(())
}

impl TransferService {
    pub(super) async fn run_outgoing_session(
        &self,
        session_id: &str,
        peer_address: &str,
        pair_code: &str,
    ) -> AppResult<()> {
        ensure_outgoing_session_active(
            session_id,
            TransferStatus::Queued,
            self.is_session_canceled(session_id),
        )?;

        let settings = self.get_settings();
        let mut stream = TcpStream::connect(peer_address)
            .await
            .with_context(|| format!("连接目标设备失败: {peer_address}"))
            .with_code("transfer_peer_connect_failed", "连接目标设备失败")
            .with_ctx("peerAddress", peer_address.to_string())
            .with_ctx("sessionId", session_id.to_string())?;

        let handshake = self
            .perform_outgoing_handshake(&mut stream, session_id, peer_address, pair_code, &settings)
            .await?;
        let codec = handshake.codec;
        let session_key = handshake.session_key;

        let mut session = self.ensure_session_exists_async(session_id).await?;
        ensure_outgoing_session_active(
            session_id,
            session.status,
            self.is_session_canceled(session.id.as_str()),
        )?;
        session.status = TransferStatus::Running;
        session.started_at = Some(now_millis());
        self.upsert_session_progress_async(&session).await?;

        let (mut reader, mut writer) = stream.into_split();
        let missing_by_file = Self::exchange_outgoing_manifest(
            &mut writer,
            &mut reader,
            &session,
            &session_key,
            codec,
            peer_address,
        )
        .await?;

        let runtime_state = self
            .build_outgoing_runtime_state(&mut session, &missing_by_file)
            .await?;
        let start_at = session.started_at.unwrap_or_else(now_millis);
        let mut state =
            outgoing_pipeline::OutgoingLoopState::from_parts(session, runtime_state, start_at);

        let mut last_db_flush = Instant::now();
        let db_flush_interval =
            Duration::from_millis(settings.db_flush_interval_ms.max(100) as u64);
        let max_inflight_chunks = settings.max_inflight_chunks.max(1) as usize;

        while state.has_pending_work() {
            self.wait_if_paused(state.session.id.as_str()).await;
            if self.is_session_canceled(state.session.id.as_str()) {
                self.finalize_outgoing_canceled(&mut state, codec).await?;
                return Err(AppError::new(TRANSFER_SESSION_CANCELED_CODE, "传输已取消")
                    .with_context("sessionId", state.session.id.clone()));
            }

            self.process_outgoing_iteration(
                &mut writer,
                &mut reader,
                &mut state,
                peer_address,
                &session_key,
                codec,
                max_inflight_chunks,
            )
            .await?;

            self.maybe_flush_outgoing_checkpoint(&mut state, &mut last_db_flush, db_flush_interval)
                .await?;

            let snapshot = state.snapshot();
            self.maybe_emit_snapshot_with_codec(
                &state.session,
                None,
                state.session.avg_speed_bps,
                snapshot.eta_seconds,
                false,
                codec,
                Some(snapshot.inflight_chunks),
                Some(snapshot.retransmit_chunks),
            );
        }

        self.finalize_outgoing_success(&mut writer, &mut state, &session_key, codec)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_outgoing_session_active_should_reject_canceled_status() {
        let result = ensure_outgoing_session_active("session-1", TransferStatus::Canceled, false);
        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, TRANSFER_SESSION_CANCELED_CODE);
    }

    #[test]
    fn ensure_outgoing_session_active_should_reject_when_cancel_flag_is_set() {
        let result = ensure_outgoing_session_active("session-1", TransferStatus::Running, true);
        assert!(result.is_err());
    }

    #[test]
    fn ensure_outgoing_session_active_should_accept_running_status() {
        let result = ensure_outgoing_session_active("session-1", TransferStatus::Running, false);
        assert!(result.is_ok());
    }
}

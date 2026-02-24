use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferHistorySyncReason {
    SessionDone,
    IncomingDone,
    IncomingCanceled,
    IncomingReadFailed,
    SessionCanceled,
    SessionFailed,
    OutgoingCanceled,
}

impl TransferHistorySyncReason {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::SessionDone => "session_done",
            Self::IncomingDone => "incoming_done",
            Self::IncomingCanceled => "incoming_canceled",
            Self::IncomingReadFailed => "incoming_read_failed",
            Self::SessionCanceled => "session_canceled",
            Self::SessionFailed => "session_failed",
            Self::OutgoingCanceled => "outgoing_canceled",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TerminalPersistOptions {
    pub(super) speed_bps: u64,
    pub(super) eta_seconds: Option<u64>,
    pub(super) codec: Option<FrameCodec>,
    pub(super) inflight_chunks: Option<u32>,
    pub(super) retransmit_chunks: Option<u32>,
    pub(super) history_reason: Option<TransferHistorySyncReason>,
}

impl TerminalPersistOptions {
    pub(super) fn new(speed_bps: u64) -> Self {
        Self {
            speed_bps,
            eta_seconds: None,
            codec: None,
            inflight_chunks: None,
            retransmit_chunks: None,
            history_reason: None,
        }
    }

    pub(super) fn with_eta(mut self, eta_seconds: Option<u64>) -> Self {
        self.eta_seconds = eta_seconds;
        self
    }

    pub(super) fn with_codec(mut self, codec: Option<FrameCodec>) -> Self {
        self.codec = codec;
        self
    }

    pub(super) fn with_inflight(mut self, inflight_chunks: Option<u32>) -> Self {
        self.inflight_chunks = inflight_chunks;
        self
    }

    pub(super) fn with_retransmit(mut self, retransmit_chunks: Option<u32>) -> Self {
        self.retransmit_chunks = retransmit_chunks;
        self
    }

    pub(super) fn with_history_reason(
        mut self,
        history_reason: Option<TransferHistorySyncReason>,
    ) -> Self {
        self.history_reason = history_reason;
        self
    }

    pub(super) fn for_codec(speed_bps: u64, codec: FrameCodec) -> Self {
        Self::new(speed_bps).with_codec(Some(codec))
    }

    pub(super) fn for_codec_with_reason(
        speed_bps: u64,
        codec: FrameCodec,
        history_reason: Option<TransferHistorySyncReason>,
    ) -> Self {
        Self::for_codec(speed_bps, codec).with_history_reason(history_reason)
    }

    pub(super) fn for_terminal_done(
        speed_bps: u64,
        codec: FrameCodec,
        inflight_chunks: Option<u32>,
        retransmit_chunks: Option<u32>,
        history_reason: TransferHistorySyncReason,
    ) -> Self {
        Self::for_codec_with_reason(speed_bps, codec, Some(history_reason))
            .with_eta(Some(0))
            .with_inflight(inflight_chunks)
            .with_retransmit(retransmit_chunks)
    }
}

impl TransferService {
    pub(super) fn local_protocol_capabilities() -> Vec<String> {
        vec![
            CAPABILITY_CODEC_BIN.to_string(),
            CAPABILITY_ACK_BATCH.to_string(),
            CAPABILITY_PIPELINE.to_string(),
        ]
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn maybe_emit_snapshot_with_codec(
        &self,
        session: &TransferSessionDto,
        active_file_id: Option<String>,
        speed_bps: u64,
        eta_seconds: Option<u64>,
        force: bool,
        codec: FrameCodec,
        inflight_chunks: Option<u32>,
        retransmit_chunks: Option<u32>,
    ) {
        self.maybe_emit_session_snapshot(
            session,
            active_file_id,
            speed_bps,
            eta_seconds,
            force,
            Some(PROTOCOL_VERSION),
            Some(codec),
            inflight_chunks,
            retransmit_chunks,
        );
    }

    pub(super) async fn flush_dirty_files(
        &self,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        clear_after_flush: bool,
    ) -> AppResult<bool> {
        if dirty_files.is_empty() {
            return Ok(false);
        }
        let items = dirty_files.values().cloned().collect::<Vec<_>>();
        self.blocking_upsert_files_batch(items).await?;
        if clear_after_flush {
            dirty_files.clear();
        }
        Ok(true)
    }

    pub(super) async fn flush_progress_checkpoint(
        &self,
        session: &TransferSessionDto,
        dirty_files: &mut HashMap<String, TransferFilePersistItem>,
        clear_dirty_after_flush: bool,
    ) -> AppResult<()> {
        self.flush_dirty_files(dirty_files, clear_dirty_after_flush)
            .await?;
        self.blocking_upsert_session_progress(session.clone()).await
    }

    pub(super) async fn flush_ack_buffer<W>(
        &self,
        writer: &mut W,
        session_id: &str,
        ack_buffer: &mut Vec<AckFrameItem>,
        ack_batch_enabled: bool,
        session_key: &[u8; 32],
        codec: FrameCodec,
    ) -> AppResult<()>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        if ack_buffer.is_empty() {
            return Ok(());
        }

        if ack_batch_enabled {
            write_frame_to(
                writer,
                &TransferFrame::AckBatch {
                    session_id: session_id.to_string(),
                    items: std::mem::take(ack_buffer),
                },
                Some(session_key),
                codec,
            )
            .await?;
            return Ok(());
        }

        for item in std::mem::take(ack_buffer) {
            write_frame_to(
                writer,
                &TransferFrame::Ack {
                    session_id: session_id.to_string(),
                    file_id: item.file_id,
                    chunk_index: item.chunk_index,
                    ok: item.ok,
                    error: item.error,
                },
                Some(session_key),
                codec,
            )
            .await?;
        }
        Ok(())
    }

    pub(super) fn mark_session_terminal_state(
        session: &mut TransferSessionDto,
        status: TransferStatus,
        error_code: Option<String>,
        error_message: Option<String>,
    ) {
        session.status = status;
        session.error_code = error_code;
        session.error_message = error_message;
        session.finished_at = Some(now_millis());
    }

    pub(super) async fn persist_terminal_session_state(
        &self,
        session: &TransferSessionDto,
        options: TerminalPersistOptions,
    ) -> AppResult<()> {
        self.blocking_upsert_session_progress(session.clone())
            .await?;
        if let Some(codec) = options.codec {
            self.maybe_emit_snapshot_with_codec(
                session,
                None,
                options.speed_bps,
                options.eta_seconds,
                true,
                codec,
                options.inflight_chunks,
                options.retransmit_chunks,
            );
        } else {
            self.maybe_emit_session_snapshot(
                session,
                None,
                options.speed_bps,
                options.eta_seconds,
                true,
                None,
                None,
                options.inflight_chunks,
                options.retransmit_chunks,
            );
        }
        if let Some(reason) = options.history_reason {
            self.emit_history_sync(reason.as_str());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_history_sync_reason_as_str_should_be_stable() {
        assert_eq!(
            TransferHistorySyncReason::SessionDone.as_str(),
            "session_done"
        );
        assert_eq!(
            TransferHistorySyncReason::IncomingDone.as_str(),
            "incoming_done"
        );
        assert_eq!(
            TransferHistorySyncReason::IncomingCanceled.as_str(),
            "incoming_canceled"
        );
        assert_eq!(
            TransferHistorySyncReason::IncomingReadFailed.as_str(),
            "incoming_read_failed"
        );
        assert_eq!(
            TransferHistorySyncReason::SessionCanceled.as_str(),
            "session_canceled"
        );
        assert_eq!(
            TransferHistorySyncReason::SessionFailed.as_str(),
            "session_failed"
        );
        assert_eq!(
            TransferHistorySyncReason::OutgoingCanceled.as_str(),
            "outgoing_canceled"
        );
    }

    #[test]
    fn terminal_persist_options_builder_should_override_fields() {
        let options = TerminalPersistOptions::new(12)
            .with_eta(Some(34))
            .with_codec(Some(FrameCodec::Bin))
            .with_inflight(Some(2))
            .with_retransmit(Some(3))
            .with_history_reason(Some(TransferHistorySyncReason::SessionDone));

        assert_eq!(options.speed_bps, 12);
        assert_eq!(options.eta_seconds, Some(34));
        assert_eq!(options.codec, Some(FrameCodec::Bin));
        assert_eq!(options.inflight_chunks, Some(2));
        assert_eq!(options.retransmit_chunks, Some(3));
        assert_eq!(
            options
                .history_reason
                .map(TransferHistorySyncReason::as_str),
            Some("session_done")
        );
    }

    #[test]
    fn terminal_persist_options_for_terminal_done_should_apply_done_defaults() {
        let options = TerminalPersistOptions::for_terminal_done(
            55,
            FrameCodec::Bin,
            Some(0),
            Some(8),
            TransferHistorySyncReason::SessionDone,
        );

        assert_eq!(options.speed_bps, 55);
        assert_eq!(options.codec, Some(FrameCodec::Bin));
        assert_eq!(options.eta_seconds, Some(0));
        assert_eq!(options.inflight_chunks, Some(0));
        assert_eq!(options.retransmit_chunks, Some(8));
        assert_eq!(
            options
                .history_reason
                .map(TransferHistorySyncReason::as_str),
            Some("session_done")
        );
    }
}

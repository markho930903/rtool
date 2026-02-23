use super::*;

impl TransferService {
    #[allow(clippy::too_many_arguments)]
    fn emit_session_snapshot(
        &self,
        session: &TransferSessionDto,
        active_file_id: Option<String>,
        speed_bps: u64,
        eta_seconds: Option<u64>,
        protocol_version: Option<u16>,
        codec: Option<FrameCodec>,
        inflight_chunks: Option<u32>,
        retransmit_chunks: Option<u32>,
    ) {
        let payload = TransferProgressSnapshotDto {
            session: session.clone(),
            active_file_id,
            speed_bps,
            eta_seconds,
            protocol_version,
            codec: codec.map(|value| value.as_str().to_string()),
            inflight_chunks,
            retransmit_chunks,
        };
        if let Err(error) = self.event_sink.emit_session_sync(&payload) {
            tracing::warn!(
                event = "transfer_event_emit_failed",
                event_name = "transfer_session_sync",
                error = error.to_string()
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn maybe_emit_session_snapshot(
        &self,
        session: &TransferSessionDto,
        active_file_id: Option<String>,
        speed_bps: u64,
        eta_seconds: Option<u64>,
        force: bool,
        protocol_version: Option<u16>,
        codec: Option<FrameCodec>,
        inflight_chunks: Option<u32>,
        retransmit_chunks: Option<u32>,
    ) {
        let settings = self.get_settings();
        let now = now_millis();
        let should_emit = if force {
            true
        } else {
            let mut guard = write_lock(self.session_last_emit_ms.as_ref(), "session_last_emit_ms");
            let interval = i64::from(settings.event_emit_interval_ms.max(50));
            let last = guard.get(session.id.as_str()).copied().unwrap_or_default();
            if now - last >= interval {
                guard.insert(session.id.clone(), now);
                true
            } else {
                false
            }
        };

        if should_emit {
            self.emit_session_snapshot(
                session,
                active_file_id,
                speed_bps,
                eta_seconds,
                protocol_version,
                codec,
                inflight_chunks,
                retransmit_chunks,
            );
        }
    }

    pub(super) fn emit_history_sync(&self, reason: &str) {
        if let Err(error) = self.event_sink.emit_history_sync(reason) {
            tracing::warn!(
                event = "transfer_event_emit_failed",
                event_name = "transfer_history_sync",
                error = error.to_string()
            );
        }
    }
}

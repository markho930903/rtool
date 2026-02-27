use protocol::AppResult;
use protocol::models::{TransferPeerDto, TransferProgressSnapshotDto};

pub trait TransferEventSink: Send + Sync {
    fn emit_peer_sync(&self, peers: &[TransferPeerDto]) -> AppResult<()>;
    fn emit_session_sync(&self, snapshot: &TransferProgressSnapshotDto) -> AppResult<()>;
    fn emit_history_sync(&self, reason: &str) -> AppResult<()>;
}

pub struct NoopTransferEventSink;

impl TransferEventSink for NoopTransferEventSink {
    fn emit_peer_sync(&self, _peers: &[TransferPeerDto]) -> AppResult<()> {
        Ok(())
    }

    fn emit_session_sync(&self, _snapshot: &TransferProgressSnapshotDto) -> AppResult<()> {
        Ok(())
    }

    fn emit_history_sync(&self, _reason: &str) -> AppResult<()> {
        Ok(())
    }
}

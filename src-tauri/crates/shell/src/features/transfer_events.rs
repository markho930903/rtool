use protocol::models::{TransferPeerDto, TransferProgressSnapshotDto};
use protocol::{AppError, AppResult};
use rtool_transfer::service::TransferEventSink;
use tauri::{AppHandle, Emitter};

const TRANSFER_PEER_SYNC_EVENT: &str = "rtool://transfer/peer_sync";
const TRANSFER_SESSION_SYNC_EVENT: &str = "rtool://transfer/session_sync";
const TRANSFER_HISTORY_SYNC_EVENT: &str = "rtool://transfer/history_sync";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HistorySyncPayload {
    reason: String,
}

#[derive(Clone)]
pub struct TauriTransferEventSink {
    app_handle: AppHandle,
}

impl TauriTransferEventSink {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl TransferEventSink for TauriTransferEventSink {
    fn emit_peer_sync(&self, peers: &[TransferPeerDto]) -> AppResult<()> {
        self.app_handle
            .emit(TRANSFER_PEER_SYNC_EVENT, peers.to_vec())
            .map_err(|error| {
                AppError::new("transfer_event_emit_failed", "推送设备列表失败")
                    .with_context("event", TRANSFER_PEER_SYNC_EVENT)
                    .with_context("detail", error.to_string())
            })
    }

    fn emit_session_sync(&self, snapshot: &TransferProgressSnapshotDto) -> AppResult<()> {
        self.app_handle
            .emit(TRANSFER_SESSION_SYNC_EVENT, snapshot.clone())
            .map_err(|error| {
                AppError::new("transfer_event_emit_failed", "推送传输会话快照失败")
                    .with_context("event", TRANSFER_SESSION_SYNC_EVENT)
                    .with_context("detail", error.to_string())
            })
    }

    fn emit_history_sync(&self, reason: &str) -> AppResult<()> {
        self.app_handle
            .emit(
                TRANSFER_HISTORY_SYNC_EVENT,
                HistorySyncPayload {
                    reason: reason.to_string(),
                },
            )
            .map_err(|error| {
                AppError::new("transfer_event_emit_failed", "推送传输历史刷新事件失败")
                    .with_context("event", TRANSFER_HISTORY_SYNC_EVENT)
                    .with_context("detail", error.to_string())
            })
    }
}

use super::*;

impl TransferService {
    pub(super) async fn ensure_session_exists_async(
        &self,
        session_id: &str,
    ) -> AppResult<TransferSessionDto> {
        ensure_session_exists(&self.db_conn, session_id).await
    }

    pub(super) async fn get_file_bitmap_async(
        &self,
        session_id: &str,
        file_id: &str,
    ) -> AppResult<Option<Vec<u8>>> {
        get_file_bitmap(&self.db_conn, session_id, file_id).await
    }

    pub(super) async fn upsert_files_batch_async(
        &self,
        items: &[TransferFilePersistItem],
    ) -> AppResult<()> {
        upsert_files_batch(&self.db_conn, items).await
    }

    pub(super) async fn upsert_session_progress_async(
        &self,
        session: &TransferSessionDto,
    ) -> AppResult<()> {
        upsert_session_progress(&self.db_conn, session).await
    }

    pub(super) async fn insert_or_update_file_async(
        &self,
        file: &TransferFileDto,
        completed_bitmap: &[u8],
    ) -> AppResult<()> {
        insert_or_update_file(&self.db_conn, file, completed_bitmap).await
    }

    pub(super) async fn validate_pair_code(
        &self,
        peer_device_id: &str,
        pair_code: &str,
    ) -> AppResult<()> {
        let blocked_until = get_peer_by_device_id(&self.db_conn, peer_device_id)
            .await?
            .and_then(|peer| peer.blocked_until);
        if let Some(value) = blocked_until
            && value > now_millis()
        {
            return Err(AppError::new(
                "transfer_peer_temporarily_blocked",
                "配对已被临时阻止，请稍后重试",
            )
            .with_context("peerDeviceId", peer_device_id.to_string())
            .with_context("blockedUntil", value.to_string()));
        }

        let settings = self.get_settings();
        if !settings.pairing_required {
            return Ok(());
        }

        let current = read_lock(self.pair_code.as_ref(), "pair_code").clone();
        let Some(entry) = current else {
            return Err(AppError::new(
                "transfer_pair_code_missing",
                "接收端尚未生成配对码",
            ));
        };

        if now_millis() > entry.expires_at {
            return Err(AppError::new("transfer_pair_code_expired", "配对码已过期"));
        }

        if entry.code != pair_code {
            mark_peer_pair_failure(&self.db_conn, peer_device_id, Some(now_millis() + 60_000))
                .await?;
            return Err(AppError::new("transfer_pair_code_invalid", "配对码错误"));
        }

        Ok(())
    }

    pub(super) fn ensure_peer_not_blocked(peer: &TransferPeerDto) -> AppResult<()> {
        if let Some(value) = peer.blocked_until
            && value > now_millis()
        {
            return Err(AppError::new(
                "transfer_peer_temporarily_blocked",
                "目标设备已被临时阻止，请稍后重试",
            )
            .with_context("peerDeviceId", peer.device_id.clone())
            .with_context("blockedUntil", value.to_string()));
        }
        Ok(())
    }
}

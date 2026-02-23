use super::*;

impl TransferService {
    pub(super) async fn blocking_ensure_session_exists(
        &self,
        session_id: String,
    ) -> AppResult<TransferSessionDto> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_ensure_session_exists", move || {
            ensure_session_exists(&pool, session_id.as_str())
        })
        .await
    }

    pub(super) async fn blocking_get_file_bitmap(
        &self,
        session_id: String,
        file_id: String,
    ) -> AppResult<Option<Vec<u8>>> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_get_file_bitmap", move || {
            get_file_bitmap(&pool, session_id.as_str(), file_id.as_str())
        })
        .await
    }

    pub(super) async fn blocking_upsert_files_batch(
        &self,
        items: Vec<TransferFilePersistItem>,
    ) -> AppResult<()> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_upsert_files_batch", move || {
            upsert_files_batch(&pool, items.as_slice())
        })
        .await
    }

    pub(super) async fn blocking_upsert_session_progress(
        &self,
        session: TransferSessionDto,
    ) -> AppResult<()> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_upsert_session_progress", move || {
            upsert_session_progress(&pool, &session)
        })
        .await
    }

    pub(super) async fn blocking_insert_or_update_file(
        &self,
        file: TransferFileDto,
        completed_bitmap: Vec<u8>,
    ) -> AppResult<()> {
        let pool = self.db_pool.clone();
        run_blocking("transfer_insert_or_update_file", move || {
            insert_or_update_file(&pool, &file, completed_bitmap.as_slice())
        })
        .await
    }

    pub(super) async fn validate_pair_code(
        &self,
        peer_device_id: &str,
        pair_code: &str,
    ) -> AppResult<()> {
        let pool = self.db_pool.clone();
        let peer_device_id_for_query = peer_device_id.to_string();
        let blocked_until = run_blocking("transfer_get_peer_blocked_until", move || {
            Ok(
                get_peer_by_device_id(&pool, peer_device_id_for_query.as_str())?
                    .and_then(|peer| peer.blocked_until),
            )
        })
        .await?;
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
            let pool = self.db_pool.clone();
            let device_id = peer_device_id.to_string();
            run_blocking("transfer_mark_pair_failure", move || {
                mark_peer_pair_failure(&pool, device_id.as_str(), Some(now_millis() + 60_000))
            })
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

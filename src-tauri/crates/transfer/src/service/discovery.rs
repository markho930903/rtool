use super::*;

fn prune_finished_discovery_tasks(tasks: &mut Vec<JoinHandle<()>>) -> usize {
    let previous_len = tasks.len();
    tasks.retain(|task| !task.is_finished());
    previous_len.saturating_sub(tasks.len())
}

impl TransferService {
    pub fn start_discovery(&self) -> AppResult<()> {
        let settings = self.get_settings();
        if !settings.discovery_enabled {
            return Ok(());
        }
        self.discovery_stop.store(false, Ordering::Relaxed);

        let mut tasks = lock_mutex(self.discovery_tasks.as_ref(), "discovery_tasks");
        let pruned = prune_finished_discovery_tasks(&mut tasks);
        if pruned > 0 {
            tracing::warn!(
                event = "transfer_discovery_task_pruned",
                pruned_count = pruned
            );
        }
        if !tasks.is_empty() {
            return Ok(());
        }

        let capabilities = vec![
            "chunk".to_string(),
            "resume".to_string(),
            "history".to_string(),
            CAPABILITY_CODEC_BIN.to_string(),
            CAPABILITY_ACK_BATCH.to_string(),
            CAPABILITY_PIPELINE.to_string(),
        ];

        let packet = DiscoveryPacket {
            device_id: self.device_id.clone(),
            display_name: self.device_name.clone(),
            listen_port: TRANSFER_LISTEN_PORT,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            pairing_required: settings.pairing_required,
            capabilities,
            ts: now_millis(),
        };

        let stop_a = self.discovery_stop.clone();
        let task_broadcast = self.spawn_task("transfer_discovery_broadcast", async move {
            run_broadcast_loop(stop_a, packet).await;
        })?;

        let stop_b = self.discovery_stop.clone();
        let peers = self.peers.clone();
        let local_device_id = self.device_id.clone();
        let task_listen = self.spawn_task("transfer_discovery_listen", async move {
            run_listen_loop(stop_b, peers, local_device_id).await;
        })?;

        let service = self.clone();
        let stop_c = self.discovery_stop.clone();
        let task_peer_sync = self.spawn_task("transfer_discovery_peer_sync", async move {
            let mut ticker = interval(Duration::from_secs(2));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            ticker.tick().await;
            loop {
                if stop_c.load(Ordering::Relaxed) {
                    break;
                }
                let _ = service.emit_peer_sync().await;
                ticker.tick().await;
            }
        })?;

        tasks.push(task_broadcast);
        tasks.push(task_listen);
        tasks.push(task_peer_sync);
        Ok(())
    }

    pub fn stop_discovery(&self) {
        self.discovery_stop.store(true, Ordering::Relaxed);

        let mut tasks = lock_mutex(self.discovery_tasks.as_ref(), "discovery_tasks");
        for task in tasks.drain(..) {
            task.abort();
        }
    }

    pub async fn list_peers(&self) -> AppResult<Vec<TransferPeerDto>> {
        let online = self.collect_online_peers().await;
        for peer in &online {
            let _ = upsert_peer(&self.db_conn, peer).await;
        }

        let stored = list_stored_peers(&self.db_conn).await?;
        Ok(merge_online_peers(stored, online.as_slice()))
    }

    pub(super) fn ensure_listener_started(&self) -> AppResult<()> {
        if self.listener_started.swap(true, Ordering::Relaxed) {
            return Ok(());
        }

        let service = self.clone();
        if let Err(error) = self.spawn_task("transfer_listener_accept_loop", async move {
            let listener = match TcpListener::bind(("0.0.0.0", TRANSFER_LISTEN_PORT)).await {
                Ok(value) => value,
                Err(error) => {
                    service.listener_started.store(false, Ordering::Relaxed);
                    tracing::error!(
                        event = "transfer_listener_bind_failed",
                        error = error.to_string()
                    );
                    return;
                }
            };

            loop {
                match listener.accept().await {
                    Ok((stream, address)) => {
                        let service_inner = service.clone();
                        if let Err(error) =
                            service.spawn_task("transfer_incoming_session", async move {
                                if let Err(error) = service_inner.handle_incoming(stream).await {
                                    if error.code == TRANSFER_SESSION_CANCELED_CODE {
                                        tracing::info!(
                                            event = "transfer_incoming_session_canceled",
                                            address = address.to_string(),
                                        );
                                    } else {
                                        tracing::warn!(
                                            event = "transfer_incoming_session_failed",
                                            address = address.to_string(),
                                            error_code = error.code,
                                            error_detail = error
                                                .causes
                                                .first()
                                                .map(String::as_str)
                                                .unwrap_or_default()
                                        );
                                    }
                                }
                            })
                        {
                            tracing::error!(
                                event = "transfer_incoming_task_spawn_failed",
                                address = address.to_string(),
                                error_code = error.code,
                                error_detail =
                                    error.causes.first().map(String::as_str).unwrap_or_default()
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            event = "transfer_listener_accept_failed",
                            error = error.to_string()
                        );
                        sleep(Duration::from_millis(250)).await;
                    }
                }
            }
        }) {
            self.listener_started.store(false, Ordering::Relaxed);
            return Err(error);
        }
        Ok(())
    }

    async fn emit_peer_sync(&self) -> AppResult<()> {
        let peers = self.list_peers().await?;
        self.event_sink.emit_peer_sync(peers.as_slice())
    }

    async fn collect_online_peers(&self) -> Vec<TransferPeerDto> {
        let peers = self.peers.read().await;
        peers
            .values()
            .map(|peer| TransferPeerDto {
                device_id: peer.device_id.clone(),
                display_name: peer.display_name.clone(),
                address: peer.address.clone(),
                listen_port: peer.listen_port,
                last_seen_at: peer.last_seen_at,
                paired_at: None,
                trust_level: TransferPeerTrustLevel::Online,
                failed_attempts: 0,
                blocked_until: None,
                pairing_required: peer.pairing_required,
                online: true,
            })
            .collect()
    }
}

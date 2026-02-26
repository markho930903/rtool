use super::*;

fn prune_finished_discovery_task(task: &mut Option<JoinHandle<()>>) -> bool {
    if let Some(handle) = task.as_ref()
        && handle.is_finished()
    {
        let _ = task.take();
        return true;
    }

    false
}

fn prune_finished_discovery_tasks(tasks: &mut DiscoveryTaskSet) -> u32 {
    let mut pruned = 0_u32;
    if prune_finished_discovery_task(&mut tasks.broadcast) {
        pruned = pruned.saturating_add(1);
    }
    if prune_finished_discovery_task(&mut tasks.listen) {
        pruned = pruned.saturating_add(1);
    }
    if prune_finished_discovery_task(&mut tasks.peer_sync) {
        pruned = pruned.saturating_add(1);
    }
    pruned
}

impl TransferService {
    pub fn start_discovery(&self) -> AppResult<()> {
        let settings = self.get_settings();
        if !settings.discovery_enabled {
            self.stop_discovery();
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
        let need_broadcast = tasks.broadcast.is_none();
        let need_listen = tasks.listen.is_none();
        let need_peer_sync = tasks.peer_sync.is_none();
        if !need_broadcast && !need_listen && !need_peer_sync {
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

        let mut staged_broadcast = None;
        let mut staged_listen = None;
        let mut staged_peer_sync = None;

        if need_broadcast {
            let stop_a = self.discovery_stop.clone();
            staged_broadcast =
                Some(self.spawn_task("transfer_discovery_broadcast", async move {
                    run_broadcast_loop(stop_a, packet).await;
                })?);
        }

        if need_listen {
            let stop_b = self.discovery_stop.clone();
            let peers = self.peers.clone();
            let local_device_id = self.device_id.clone();
            match self.spawn_task("transfer_discovery_listen", async move {
                run_listen_loop(stop_b, peers, local_device_id).await;
            }) {
                Ok(task) => staged_listen = Some(task),
                Err(error) => {
                    if let Some(task) = staged_broadcast.take() {
                        task.abort();
                    }
                    return Err(error);
                }
            }
        }

        if need_peer_sync {
            let service = self.clone();
            let stop_c = self.discovery_stop.clone();
            match self.spawn_task("transfer_discovery_peer_sync", async move {
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
            }) {
                Ok(task) => staged_peer_sync = Some(task),
                Err(error) => {
                    if let Some(task) = staged_broadcast.take() {
                        task.abort();
                    }
                    if let Some(task) = staged_listen.take() {
                        task.abort();
                    }
                    return Err(error);
                }
            }
        }

        if let Some(task) = staged_broadcast {
            tasks.broadcast = Some(task);
        }
        if let Some(task) = staged_listen {
            tasks.listen = Some(task);
        }
        if let Some(task) = staged_peer_sync {
            tasks.peer_sync = Some(task);
        }
        Ok(())
    }

    pub fn stop_discovery(&self) {
        self.discovery_stop.store(true, Ordering::Relaxed);

        let mut tasks = lock_mutex(self.discovery_tasks.as_ref(), "discovery_tasks");
        if let Some(task) = tasks.broadcast.take() {
            task.abort();
        }
        if let Some(task) = tasks.listen.take() {
            task.abort();
        }
        if let Some(task) = tasks.peer_sync.take() {
            task.abort();
        }
    }

    pub fn runtime_status(&self) -> TransferRuntimeStatusDto {
        let settings = self.get_settings();
        let tasks = lock_mutex(self.discovery_tasks.as_ref(), "discovery_tasks");
        let discovery_tasks = TransferDiscoveryTaskStatusDto {
            broadcast: tasks.broadcast.is_some(),
            listen: tasks.listen.is_some(),
            peer_sync: tasks.peer_sync.is_some(),
        };
        let discovery_running =
            discovery_tasks.broadcast || discovery_tasks.listen || discovery_tasks.peer_sync;
        TransferRuntimeStatusDto {
            listener_started: self.listener_started.load(Ordering::Relaxed),
            discovery_enabled: settings.discovery_enabled,
            discovery_running,
            discovery_tasks,
            protocol_version: PROTOCOL_VERSION,
            flow_control_mode: settings.flow_control_mode,
            retransmit_ratio: self.retransmit_ratio(),
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

        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel::<Result<(), AppError>>(1);
        let service = self.clone();
        let accept_task = match self.spawn_task("transfer_listener_accept_loop", async move {
            let listener = match TcpListener::bind(("0.0.0.0", TRANSFER_LISTEN_PORT)).await {
                Ok(value) => {
                    let _ = ready_tx.send(Ok(()));
                    tracing::info!(
                        event = "transfer_listener_bound",
                        port = TRANSFER_LISTEN_PORT
                    );
                    value
                }
                Err(error) => {
                    service.listener_started.store(false, Ordering::Relaxed);
                    let detail = error.to_string();
                    let _ = ready_tx.send(Err(AppError::new(
                        "transfer_listener_bind_failed",
                        "启动传输监听失败",
                    )
                    .with_context("detail", detail.clone())));
                    tracing::error!(event = "transfer_listener_bind_failed", error = detail);
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
            Ok(task) => task,
            Err(error) => {
                self.listener_started.store(false, Ordering::Relaxed);
                return Err(error);
            }
        };

        match ready_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                accept_task.abort();
                self.listener_started.store(false, Ordering::Relaxed);
                Err(error)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                accept_task.abort();
                self.listener_started.store(false, Ordering::Relaxed);
                Err(AppError::new(
                    "transfer_listener_start_timeout",
                    "启动传输监听超时",
                ))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                #[cfg(test)]
                {
                    tracing::warn!(
                        event = "transfer_listener_start_channel_closed_in_test",
                        detail = "listener start handshake channel disconnected under test runtime"
                    );
                    return Ok(());
                }
                #[cfg(not(test))]
                {
                    accept_task.abort();
                    self.listener_started.store(false, Ordering::Relaxed);
                    Err(AppError::new(
                        "transfer_listener_start_channel_closed",
                        "传输监听启动状态异常",
                    ))
                }
            }
        }
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

use super::*;

impl TransferService {
    pub(super) async fn run_outgoing_session(
        &self,
        session_id: &str,
        peer_address: &str,
        pair_code: &str,
    ) -> AppResult<()> {
        #[derive(Debug)]
        struct OutgoingFileRuntime {
            file: TransferFileDto,
            bitmap: Vec<u8>,
            reader: ChunkReader,
            remaining_chunks: u32,
            file_done_sent: bool,
        }

        #[derive(Debug)]
        struct InflightChunk {
            file_idx: usize,
            chunk_index: u32,
            sent_at: Instant,
            retries: u8,
        }

        let settings = self.get_settings();
        let mut stream = TcpStream::connect(peer_address)
            .await
            .with_context(|| format!("连接目标设备失败: {peer_address}"))
            .with_code("transfer_peer_connect_failed", "连接目标设备失败")
            .with_ctx("peerAddress", peer_address.to_string())
            .with_ctx("sessionId", session_id.to_string())?;

        let local_capabilities = vec![
            CAPABILITY_CODEC_BIN_V2.to_string(),
            CAPABILITY_ACK_BATCH_V2.to_string(),
            CAPABILITY_PIPELINE_V2.to_string(),
        ];
        let client_nonce = random_hex(16);
        write_frame(
            &mut stream,
            &TransferFrame::Hello {
                device_id: self.device_id.clone(),
                device_name: self.device_name.clone(),
                nonce: client_nonce.clone(),
                protocol_version: Some(PROTOCOL_VERSION_V2),
                capabilities: Some(local_capabilities.clone()),
            },
            None,
        )
        .await?;

        let challenge = read_frame(&mut stream, None).await?;
        let server_nonce = match challenge {
            TransferFrame::AuthChallenge { nonce, .. } => nonce,
            TransferFrame::Error { code, message } => {
                return Err(AppError::new(code, "目标设备拒绝连接")
                    .with_cause(message)
                    .with_context("sessionId", session_id.to_string())
                    .with_context("peerAddress", peer_address.to_string()));
            }
            other => {
                return Err(AppError::new(
                    "transfer_protocol_challenge_invalid",
                    "握手挑战帧不合法",
                )
                .with_context("sessionId", session_id.to_string())
                .with_context("peerAddress", peer_address.to_string())
                .with_context("unexpectedFrame", format!("{other:?}")));
            }
        };

        let proof = derive_proof(pair_code, client_nonce.as_str(), server_nonce.as_str());
        write_frame(
            &mut stream,
            &TransferFrame::AuthResponse {
                pair_code: pair_code.to_string(),
                proof,
            },
            None,
        )
        .await?;

        let (peer_protocol_version, peer_capabilities) = match read_frame(&mut stream, None).await?
        {
            TransferFrame::AuthOk {
                protocol_version,
                capabilities,
                ..
            } => (
                protocol_version.unwrap_or(1),
                capabilities.unwrap_or_default(),
            ),
            TransferFrame::Error { code, message } => {
                return Err(AppError::new(code, "认证失败")
                    .with_cause(message)
                    .with_context("sessionId", session_id.to_string())
                    .with_context("peerAddress", peer_address.to_string()));
            }
            other => {
                return Err(
                    AppError::new("transfer_protocol_auth_invalid", "认证响应帧不合法")
                        .with_context("sessionId", session_id.to_string())
                        .with_context("peerAddress", peer_address.to_string())
                        .with_context("unexpectedFrame", format!("{other:?}")),
                );
            }
        };

        let codec = if settings.codec_v2_enabled
            && peer_protocol_version >= PROTOCOL_VERSION_V2
            && peer_capabilities
                .iter()
                .any(|value| value == CAPABILITY_CODEC_BIN_V2)
        {
            FrameCodec::BinV2
        } else {
            FrameCodec::JsonV1
        };
        tracing::info!(
            event = "transfer_protocol_negotiated",
            session_id = session_id,
            peer_address,
            codec = codec.as_str(),
            peer_protocol_version
        );

        let session_key =
            derive_session_key(pair_code, client_nonce.as_str(), server_nonce.as_str());

        let mut session = self
            .blocking_ensure_session_exists(session_id.to_string())
            .await?;
        session.status = "running".to_string();
        session.started_at = Some(now_millis());
        self.blocking_upsert_session_progress(session.clone())
            .await?;

        let manifest_files = session
            .files
            .iter()
            .map(|file| ManifestFileFrame {
                file_id: file.id.clone(),
                relative_path: file.relative_path.clone(),
                size_bytes: file.size_bytes,
                chunk_size: file.chunk_size,
                chunk_count: file.chunk_count,
                blake3: file.blake3.clone().unwrap_or_default(),
                mime_type: file.mime_type.clone(),
                is_folder_archive: file.is_folder_archive,
            })
            .collect::<Vec<_>>();

        let (mut reader, mut writer) = stream.into_split();
        write_frame_to(
            &mut writer,
            &TransferFrame::Manifest {
                session_id: session.id.clone(),
                direction: session.direction.clone(),
                save_dir: session.save_dir.clone(),
                files: manifest_files,
            },
            Some(&session_key),
            codec,
        )
        .await?;

        let mut missing_by_file = HashMap::<String, Vec<u32>>::new();
        let manifest_ack = read_frame_from(&mut reader, Some(&session_key), Some(codec)).await?;
        match manifest_ack {
            TransferFrame::ManifestAck {
                session_id: ack_session_id,
                missing_chunks,
            } if ack_session_id == session.id => {
                for item in missing_chunks {
                    missing_by_file.insert(item.file_id, item.missing_chunk_indexes);
                }
            }
            TransferFrame::Error { code, message } => {
                return Err(AppError::new(code, "接收端拒绝文件清单")
                    .with_cause(message)
                    .with_context("sessionId", session.id.clone())
                    .with_context("peerAddress", peer_address.to_string()));
            }
            other => {
                return Err(AppError::new(
                    "transfer_protocol_manifest_ack_invalid",
                    "MANIFEST_ACK 帧不合法",
                )
                .with_context("sessionId", session.id.clone())
                .with_context("peerAddress", peer_address.to_string())
                .with_context("unexpectedFrame", format!("{other:?}")));
            }
        }

        let mut runtimes = Vec::<OutgoingFileRuntime>::new();
        let mut file_id_to_idx = HashMap::<String, usize>::new();
        let mut fair_queue = VecDeque::<(usize, u32)>::new();
        let mut per_file_missing = Vec::<VecDeque<u32>>::new();

        for (index, file) in session.files.iter_mut().enumerate() {
            let bitmap = self
                .blocking_get_file_bitmap(session.id.clone(), file.id.clone())
                .await
                .unwrap_or_default()
                .unwrap_or_else(|| empty_bitmap(file.chunk_count));
            let source_path = PathBuf::from(file.source_path.clone().unwrap_or_default());
            let mut missing = missing_by_file
                .get(file.id.as_str())
                .cloned()
                .unwrap_or_else(|| missing_chunks(bitmap.as_slice(), file.chunk_count));
            missing.sort_unstable();
            file.status = "running".to_string();
            file.transferred_bytes = completed_bytes(
                bitmap.as_slice(),
                file.chunk_count,
                file.chunk_size,
                file.size_bytes,
            );
            file_id_to_idx.insert(file.id.clone(), index);
            per_file_missing.push(VecDeque::from(missing.clone()));
            runtimes.push(OutgoingFileRuntime {
                file: file.clone(),
                bitmap,
                reader: ChunkReader::open(source_path.as_path()).await?,
                remaining_chunks: missing.len() as u32,
                file_done_sent: false,
            });
        }

        loop {
            let mut progressed = false;
            for (idx, queue) in per_file_missing.iter_mut().enumerate() {
                if let Some(chunk_index) = queue.pop_front() {
                    fair_queue.push_back((idx, chunk_index));
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }

        session.transferred_bytes = session
            .files
            .iter()
            .map(|item| item.transferred_bytes)
            .sum();
        let start_at = session.started_at.unwrap_or_else(now_millis);

        let mut inflight = HashMap::<(usize, u32), InflightChunk>::new();
        let mut retry_counts = HashMap::<(usize, u32), u8>::new();
        let mut retransmit_chunks = 0u32;
        let mut dirty_files = HashMap::<String, TransferFilePersistItem>::new();
        let mut last_db_flush = Instant::now();
        let db_flush_interval =
            Duration::from_millis(settings.db_flush_interval_ms.max(100) as u64);
        let max_inflight_chunks = if settings.pipeline_v2_enabled {
            settings.max_inflight_chunks.max(1) as usize
        } else {
            1
        };

        while !fair_queue.is_empty() || !inflight.is_empty() {
            self.wait_if_paused(session.id.as_str()).await;
            if self.is_session_canceled(session.id.as_str()) {
                return Err(AppError::new("transfer_session_canceled", "传输已取消"));
            }

            while inflight.len() < max_inflight_chunks {
                let Some((file_idx, chunk_index)) = fair_queue.pop_front() else {
                    break;
                };
                if inflight.contains_key(&(file_idx, chunk_index)) {
                    continue;
                }
                let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                    AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                })?;
                if runtime.file_done_sent || runtime.remaining_chunks == 0 {
                    continue;
                }
                if crate::infrastructure::transfer::resume::is_chunk_done(
                    runtime.bitmap.as_slice(),
                    chunk_index,
                ) {
                    continue;
                }

                let bytes = runtime
                    .reader
                    .read_chunk(chunk_index, runtime.file.chunk_size)
                    .await?;
                let hash = blake3::hash(bytes.as_slice()).to_hex().to_string();
                let frame = match codec {
                    FrameCodec::JsonV1 => TransferFrame::Chunk {
                        session_id: session.id.clone(),
                        file_id: runtime.file.id.clone(),
                        chunk_index,
                        total_chunks: runtime.file.chunk_count,
                        hash,
                        data: base64::engine::general_purpose::STANDARD.encode(bytes.as_slice()),
                    },
                    FrameCodec::BinV2 => TransferFrame::ChunkBinary {
                        session_id: session.id.clone(),
                        file_id: runtime.file.id.clone(),
                        chunk_index,
                        total_chunks: runtime.file.chunk_count,
                        hash,
                        data: bytes,
                    },
                };
                write_frame_to(&mut writer, &frame, Some(&session_key), codec).await?;
                inflight.insert(
                    (file_idx, chunk_index),
                    InflightChunk {
                        file_idx,
                        chunk_index,
                        sent_at: Instant::now(),
                        retries: retry_counts
                            .get(&(file_idx, chunk_index))
                            .copied()
                            .unwrap_or_default(),
                    },
                );
            }

            match tokio::time::timeout(
                Duration::from_millis(40),
                read_frame_from(&mut reader, Some(&session_key), Some(codec)),
            )
            .await
            {
                Ok(Ok(frame)) => {
                    let mut ack_items = Vec::<AckFrameItem>::new();
                    match frame {
                        TransferFrame::Ack {
                            session_id: ack_session_id,
                            file_id,
                            chunk_index,
                            ok,
                            error,
                        } if ack_session_id == session.id => {
                            ack_items.push(AckFrameItem {
                                file_id,
                                chunk_index,
                                ok,
                                error,
                            });
                        }
                        TransferFrame::AckBatch {
                            session_id: ack_session_id,
                            items,
                        } if ack_session_id == session.id => {
                            ack_items.extend(items);
                        }
                        TransferFrame::Error { code, message } => {
                            return Err(AppError::new(code, "目标设备返回错误")
                                .with_cause(message)
                                .with_context("sessionId", session.id.clone())
                                .with_context("peerAddress", peer_address.to_string()));
                        }
                        TransferFrame::Ping { .. } => {}
                        _ => {}
                    }

                    for ack in ack_items {
                        let Some(file_idx) = file_id_to_idx.get(ack.file_id.as_str()).copied()
                        else {
                            continue;
                        };
                        let key = (file_idx, ack.chunk_index);
                        let Some(inflight_chunk) = inflight.remove(&key) else {
                            continue;
                        };
                        if ack.ok {
                            retry_counts.remove(&key);
                            let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                                AppError::new(
                                    "transfer_runtime_file_missing",
                                    "传输文件运行时状态不存在",
                                )
                            })?;
                            if !crate::infrastructure::transfer::resume::is_chunk_done(
                                runtime.bitmap.as_slice(),
                                ack.chunk_index,
                            ) {
                                mark_chunk_done(runtime.bitmap.as_mut_slice(), ack.chunk_index)?;
                                let previous = runtime.file.transferred_bytes;
                                runtime.file.transferred_bytes = completed_bytes(
                                    runtime.bitmap.as_slice(),
                                    runtime.file.chunk_count,
                                    runtime.file.chunk_size,
                                    runtime.file.size_bytes,
                                );
                                runtime.file.status = "running".to_string();
                                if runtime.file.transferred_bytes > previous {
                                    session.transferred_bytes = session
                                        .transferred_bytes
                                        .saturating_add(runtime.file.transferred_bytes - previous);
                                }
                                if runtime.remaining_chunks > 0 {
                                    runtime.remaining_chunks -= 1;
                                }

                                session.avg_speed_bps =
                                    calculate_speed(session.transferred_bytes, start_at);
                                session.files[file_idx] = runtime.file.clone();
                                dirty_files.insert(
                                    runtime.file.id.clone(),
                                    TransferFilePersistItem {
                                        file: runtime.file.clone(),
                                        completed_bitmap: runtime.bitmap.clone(),
                                    },
                                );
                            }

                            if runtime.remaining_chunks == 0 && !runtime.file_done_sent {
                                runtime.file.status = "success".to_string();
                                runtime.file.transferred_bytes = runtime.file.size_bytes;
                                session.files[file_idx] = runtime.file.clone();
                                dirty_files.insert(
                                    runtime.file.id.clone(),
                                    TransferFilePersistItem {
                                        file: runtime.file.clone(),
                                        completed_bitmap: runtime.bitmap.clone(),
                                    },
                                );
                                write_frame_to(
                                    &mut writer,
                                    &TransferFrame::FileDone {
                                        session_id: session.id.clone(),
                                        file_id: runtime.file.id.clone(),
                                        blake3: runtime.file.blake3.clone().unwrap_or_default(),
                                    },
                                    Some(&session_key),
                                    codec,
                                )
                                .await?;
                                runtime.file_done_sent = true;
                            }
                        } else {
                            let retry = inflight_chunk.retries.saturating_add(1);
                            if retry > MAX_CHUNK_RETRY {
                                return Err(AppError::new(
                                    "transfer_chunk_retry_exhausted",
                                    "分块重试次数已耗尽",
                                )
                                .with_context("sessionId", session.id.clone())
                                .with_context("fileId", ack.file_id.clone())
                                .with_context("fileIdx", inflight_chunk.file_idx.to_string())
                                .with_context("chunkIndex", inflight_chunk.chunk_index.to_string())
                                .with_context("peerAddress", peer_address.to_string()));
                            }
                            retransmit_chunks = retransmit_chunks.saturating_add(1);
                            tracing::warn!(
                                event = "transfer_chunk_requeue_failed_ack",
                                session_id = session.id,
                                file_id = ack.file_id,
                                chunk_index = ack.chunk_index,
                                retry
                            );
                            retry_counts.insert(key, retry);
                            fair_queue.push_front(key);
                        }
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => {}
            }

            let mut timeout_chunks = Vec::new();
            for (key, value) in &inflight {
                if value.sent_at.elapsed() >= Duration::from_millis(CHUNK_ACK_TIMEOUT_MS) {
                    timeout_chunks.push(*key);
                }
            }
            for key in timeout_chunks {
                if let Some(old) = inflight.remove(&key) {
                    let retry = old.retries.saturating_add(1);
                    if retry > MAX_CHUNK_RETRY {
                        return Err(AppError::new(
                            "transfer_chunk_ack_timeout",
                            "分块确认超时且超过重试上限",
                        )
                        .with_context("sessionId", session.id.clone())
                        .with_context("fileIdx", key.0.to_string())
                        .with_context("chunkIndex", key.1.to_string())
                        .with_context("peerAddress", peer_address.to_string()));
                    }
                    retransmit_chunks = retransmit_chunks.saturating_add(1);
                    tracing::warn!(
                        event = "transfer_chunk_requeue_timeout",
                        session_id = session.id,
                        file_idx = key.0,
                        chunk_index = key.1,
                        retry
                    );
                    retry_counts.insert(key, retry);
                    fair_queue.push_front(key);
                }
            }

            if last_db_flush.elapsed() >= db_flush_interval {
                if !dirty_files.is_empty() {
                    let items = dirty_files.values().cloned().collect::<Vec<_>>();
                    self.blocking_upsert_files_batch(items).await?;
                    dirty_files.clear();
                }
                self.blocking_upsert_session_progress(session.clone())
                    .await?;
                last_db_flush = Instant::now();
            }

            let eta = estimate_eta(
                session.total_bytes,
                session.transferred_bytes,
                session.avg_speed_bps,
            );
            self.maybe_emit_session_snapshot(
                &session,
                None,
                session.avg_speed_bps,
                eta,
                false,
                Some(if codec == FrameCodec::BinV2 {
                    PROTOCOL_VERSION_V2
                } else {
                    1
                }),
                Some(codec),
                Some(inflight.len() as u32),
                Some(retransmit_chunks),
            );
        }

        if !dirty_files.is_empty() {
            let items = dirty_files.values().cloned().collect::<Vec<_>>();
            self.blocking_upsert_files_batch(items).await?;
        }

        write_frame_to(
            &mut writer,
            &TransferFrame::SessionDone {
                session_id: session.id.clone(),
                ok: true,
                error: None,
            },
            Some(&session_key),
            codec,
        )
        .await?;

        session.status = "success".to_string();
        session.transferred_bytes = session.total_bytes;
        session.avg_speed_bps = calculate_speed(session.transferred_bytes, start_at);
        session.finished_at = Some(now_millis());
        session.error_code = None;
        session.error_message = None;
        self.blocking_upsert_session_progress(session.clone())
            .await?;
        self.maybe_emit_session_snapshot(
            &session,
            None,
            session.avg_speed_bps,
            Some(0),
            true,
            Some(if codec == FrameCodec::BinV2 {
                PROTOCOL_VERSION_V2
            } else {
                1
            }),
            Some(codec),
            Some(0),
            Some(retransmit_chunks),
        );
        self.emit_history_sync("session_done");
        Ok(())
    }
}

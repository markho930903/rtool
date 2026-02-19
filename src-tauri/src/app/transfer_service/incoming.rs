use super::*;

impl TransferService {
    pub(super) async fn handle_incoming(&self, mut stream: TcpStream) -> AppResult<()> {
        #[derive(Debug)]
        struct IncomingFileRuntime {
            file: TransferFileDto,
            bitmap: Vec<u8>,
            writer: ChunkWriter,
        }

        let settings = self.get_settings();
        let hello = read_frame(&mut stream, None).await?;
        let (peer_device_id, peer_name, client_nonce, peer_protocol_version, peer_capabilities) =
            match hello {
                TransferFrame::Hello {
                    device_id,
                    device_name,
                    nonce,
                    protocol_version,
                    capabilities,
                } => (
                    device_id,
                    device_name,
                    nonce,
                    protocol_version.unwrap_or(1),
                    capabilities.unwrap_or_default(),
                ),
                _ => {
                    return Err(AppError::new(
                        "transfer_protocol_hello_invalid",
                        "无效的 HELLO 帧",
                    ));
                }
            };

        let local_capabilities = vec![
            CAPABILITY_CODEC_BIN_V2.to_string(),
            CAPABILITY_ACK_BATCH_V2.to_string(),
            CAPABILITY_PIPELINE_V2.to_string(),
        ];
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
        let ack_batch_enabled = settings.pipeline_v2_enabled
            && codec == FrameCodec::BinV2
            && peer_capabilities
                .iter()
                .any(|value| value == CAPABILITY_ACK_BATCH_V2);
        tracing::info!(
            event = "transfer_protocol_negotiated_incoming",
            peer_device_id,
            codec = codec.as_str(),
            peer_protocol_version,
            ack_batch_enabled
        );

        let server_nonce = random_hex(16);
        let expires_at = now_millis() + PAIR_CODE_EXPIRE_MS;
        write_frame(
            &mut stream,
            &TransferFrame::AuthChallenge {
                nonce: server_nonce.clone(),
                expires_at,
            },
            None,
        )
        .await?;

        let auth = read_frame(&mut stream, None).await?;
        let (pair_code, proof) = match auth {
            TransferFrame::AuthResponse { pair_code, proof } => (pair_code, proof),
            _ => {
                return Err(AppError::new(
                    "transfer_protocol_auth_response_invalid",
                    "无效的 AUTH_RESPONSE 帧",
                ));
            }
        };

        self.validate_pair_code(peer_device_id.as_str(), pair_code.as_str())
            .await?;
        let expected = derive_proof(
            pair_code.as_str(),
            client_nonce.as_str(),
            server_nonce.as_str(),
        );
        if proof != expected {
            let pool = self.db_pool.clone();
            let peer_device_id = peer_device_id.clone();
            run_blocking("transfer_mark_pair_failure", move || {
                mark_peer_pair_failure(&pool, peer_device_id.as_str(), Some(now_millis() + 60_000))
            })
            .await?;
            write_frame(
                &mut stream,
                &TransferFrame::Error {
                    code: "transfer_auth_failed".to_string(),
                    message: "配对码校验失败".to_string(),
                },
                None,
            )
            .await?;
            return Err(AppError::new("transfer_auth_failed", "配对码校验失败"));
        }

        let pool = self.db_pool.clone();
        let peer_device_id_for_success = peer_device_id.clone();
        run_blocking("transfer_mark_pair_success", move || {
            mark_peer_pair_success(&pool, peer_device_id_for_success.as_str(), now_millis())
        })
        .await?;
        write_frame(
            &mut stream,
            &TransferFrame::AuthOk {
                peer_device_id: self.device_id.clone(),
                peer_name: self.device_name.clone(),
                protocol_version: Some(PROTOCOL_VERSION_V2),
                capabilities: Some(local_capabilities),
            },
            None,
        )
        .await?;

        let session_key = derive_session_key(
            pair_code.as_str(),
            client_nonce.as_str(),
            server_nonce.as_str(),
        );
        let (mut reader, mut writer) = stream.into_split();

        let manifest = read_frame_from(&mut reader, Some(&session_key), Some(codec)).await?;
        let (session_id, direction, save_dir, files) = match manifest {
            TransferFrame::Manifest {
                session_id,
                direction,
                save_dir,
                files,
            } => (session_id, direction, save_dir, files),
            _ => {
                return Err(AppError::new(
                    "transfer_protocol_manifest_invalid",
                    "无效的 MANIFEST 帧",
                ));
            }
        };

        let cleanup_after_at = now_millis() + i64::from(settings.auto_cleanup_days) * 86_400_000;
        let total_bytes = files.iter().map(|value| value.size_bytes).sum::<u64>();

        let mut session = TransferSessionDto {
            id: session_id.clone(),
            direction: if direction == "receive" {
                "send".to_string()
            } else {
                "receive".to_string()
            },
            peer_device_id: peer_device_id.clone(),
            peer_name: peer_name.clone(),
            status: "running".to_string(),
            total_bytes,
            transferred_bytes: 0,
            avg_speed_bps: 0,
            save_dir: save_dir.clone(),
            created_at: now_millis(),
            started_at: Some(now_millis()),
            finished_at: None,
            error_code: None,
            error_message: None,
            cleanup_after_at: Some(cleanup_after_at),
            files: Vec::new(),
        };
        self.blocking_upsert_session_progress(session.clone())
            .await?;

        let save_dir_path = PathBuf::from(settings.default_download_dir);
        let mut missing_chunks_payload = Vec::new();
        let mut runtimes = Vec::<IncomingFileRuntime>::new();
        let mut file_id_to_idx = HashMap::<String, usize>::new();

        for manifest_file in files {
            let mut bitmap = self
                .blocking_get_file_bitmap(session.id.clone(), manifest_file.file_id.clone())
                .await
                .unwrap_or_default()
                .unwrap_or_else(|| empty_bitmap(manifest_file.chunk_count));
            if bitmap.is_empty() {
                bitmap = empty_bitmap(manifest_file.chunk_count);
            }

            let target_path = resolve_target_path(
                save_dir_path.as_path(),
                manifest_file.relative_path.as_str(),
            );
            let part_path = build_part_path(
                save_dir_path.as_path(),
                session.id.as_str(),
                manifest_file.relative_path.as_str(),
            );
            let missing = missing_chunks(bitmap.as_slice(), manifest_file.chunk_count);
            missing_chunks_payload.push(MissingChunkFrame {
                file_id: manifest_file.file_id.clone(),
                missing_chunk_indexes: missing,
            });

            let file = TransferFileDto {
                id: manifest_file.file_id,
                session_id: session.id.clone(),
                relative_path: manifest_file.relative_path,
                source_path: None,
                target_path: Some(target_path.to_string_lossy().to_string()),
                size_bytes: manifest_file.size_bytes,
                transferred_bytes: completed_bytes(
                    bitmap.as_slice(),
                    manifest_file.chunk_count,
                    manifest_file.chunk_size,
                    manifest_file.size_bytes,
                ),
                chunk_size: manifest_file.chunk_size,
                chunk_count: manifest_file.chunk_count,
                status: "running".to_string(),
                blake3: Some(manifest_file.blake3),
                mime_type: manifest_file.mime_type,
                preview_kind: None,
                preview_data: Some(part_path.to_string_lossy().to_string()),
                is_folder_archive: manifest_file.is_folder_archive,
            };
            self.blocking_insert_or_update_file(file.clone(), bitmap.clone())
                .await?;
            file_id_to_idx.insert(file.id.clone(), runtimes.len());
            let writer = ChunkWriter::open(part_path.as_path(), Some(file.size_bytes)).await?;
            runtimes.push(IncomingFileRuntime {
                file: file.clone(),
                bitmap,
                writer,
            });
            session.files.push(file);
        }

        write_frame_to(
            &mut writer,
            &TransferFrame::ManifestAck {
                session_id: session.id.clone(),
                missing_chunks: missing_chunks_payload,
            },
            Some(&session_key),
            codec,
        )
        .await?;

        let started_at = now_millis();
        let mut ack_buffer = Vec::<AckFrameItem>::new();
        let mut last_ack_flush = Instant::now();
        let ack_flush_interval =
            Duration::from_millis(settings.ack_flush_interval_ms.max(5) as u64);
        let db_flush_interval =
            Duration::from_millis(settings.db_flush_interval_ms.max(100) as u64);
        let mut last_db_flush = Instant::now();
        let mut dirty_files = HashMap::<String, TransferFilePersistItem>::new();

        loop {
            let frame = read_frame_from(&mut reader, Some(&session_key), Some(codec)).await?;
            match frame {
                TransferFrame::Chunk {
                    session_id: incoming_session_id,
                    file_id,
                    chunk_index,
                    hash,
                    data,
                    ..
                } => {
                    if incoming_session_id != session.id {
                        continue;
                    }
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(data.as_bytes())
                        .map_err(|error| {
                            AppError::new("transfer_chunk_decode_failed", "分块解码失败")
                                .with_detail(error.to_string())
                        })?;
                    let Some(file_idx) = file_id_to_idx.get(file_id.as_str()).copied() else {
                        continue;
                    };
                    let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                        AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                    })?;
                    let calculated_hash = blake3::hash(decoded.as_slice()).to_hex().to_string();
                    if calculated_hash != hash {
                        ack_buffer.push(AckFrameItem {
                            file_id,
                            chunk_index,
                            ok: false,
                            error: Some("chunk_hash_mismatch".to_string()),
                        });
                        continue;
                    }
                    runtime
                        .writer
                        .write_chunk(chunk_index, runtime.file.chunk_size, decoded.as_slice())
                        .await?;
                    if !crate::infrastructure::transfer::resume::is_chunk_done(
                        runtime.bitmap.as_slice(),
                        chunk_index,
                    ) {
                        mark_chunk_done(runtime.bitmap.as_mut_slice(), chunk_index)?;
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
                        session.files[file_idx] = runtime.file.clone();
                        dirty_files.insert(
                            runtime.file.id.clone(),
                            TransferFilePersistItem {
                                file: runtime.file.clone(),
                                completed_bitmap: runtime.bitmap.clone(),
                            },
                        );
                    }
                    ack_buffer.push(AckFrameItem {
                        file_id: runtime.file.id.clone(),
                        chunk_index,
                        ok: true,
                        error: None,
                    });
                    session.avg_speed_bps = calculate_speed(session.transferred_bytes, started_at);
                    let eta = estimate_eta(
                        session.total_bytes,
                        session.transferred_bytes,
                        session.avg_speed_bps,
                    );
                    self.maybe_emit_session_snapshot(
                        &session,
                        Some(runtime.file.id.clone()),
                        session.avg_speed_bps,
                        eta,
                        false,
                        Some(if codec == FrameCodec::BinV2 {
                            PROTOCOL_VERSION_V2
                        } else {
                            1
                        }),
                        Some(codec),
                        None,
                        None,
                    );
                }
                TransferFrame::ChunkBinary {
                    session_id: incoming_session_id,
                    file_id,
                    chunk_index,
                    hash,
                    data,
                    ..
                } => {
                    if incoming_session_id != session.id {
                        continue;
                    }
                    let Some(file_idx) = file_id_to_idx.get(file_id.as_str()).copied() else {
                        continue;
                    };
                    let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                        AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                    })?;
                    let calculated_hash = blake3::hash(data.as_slice()).to_hex().to_string();
                    if calculated_hash != hash {
                        ack_buffer.push(AckFrameItem {
                            file_id,
                            chunk_index,
                            ok: false,
                            error: Some("chunk_hash_mismatch".to_string()),
                        });
                        continue;
                    }
                    runtime
                        .writer
                        .write_chunk(chunk_index, runtime.file.chunk_size, data.as_slice())
                        .await?;
                    if !crate::infrastructure::transfer::resume::is_chunk_done(
                        runtime.bitmap.as_slice(),
                        chunk_index,
                    ) {
                        mark_chunk_done(runtime.bitmap.as_mut_slice(), chunk_index)?;
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
                        session.files[file_idx] = runtime.file.clone();
                        dirty_files.insert(
                            runtime.file.id.clone(),
                            TransferFilePersistItem {
                                file: runtime.file.clone(),
                                completed_bitmap: runtime.bitmap.clone(),
                            },
                        );
                    }
                    ack_buffer.push(AckFrameItem {
                        file_id: runtime.file.id.clone(),
                        chunk_index,
                        ok: true,
                        error: None,
                    });
                    session.avg_speed_bps = calculate_speed(session.transferred_bytes, started_at);
                    let eta = estimate_eta(
                        session.total_bytes,
                        session.transferred_bytes,
                        session.avg_speed_bps,
                    );
                    self.maybe_emit_session_snapshot(
                        &session,
                        Some(runtime.file.id.clone()),
                        session.avg_speed_bps,
                        eta,
                        false,
                        Some(if codec == FrameCodec::BinV2 {
                            PROTOCOL_VERSION_V2
                        } else {
                            1
                        }),
                        Some(codec),
                        None,
                        None,
                    );
                }
                TransferFrame::FileDone {
                    session_id: incoming_session_id,
                    file_id,
                    blake3,
                } => {
                    if incoming_session_id != session.id {
                        continue;
                    }
                    let Some(file_idx) = file_id_to_idx.get(file_id.as_str()).copied() else {
                        continue;
                    };
                    let runtime = runtimes.get_mut(file_idx).ok_or_else(|| {
                        AppError::new("transfer_runtime_file_missing", "传输文件运行时状态不存在")
                    })?;
                    runtime.writer.flush().await?;
                    let part_path =
                        PathBuf::from(runtime.file.preview_data.clone().unwrap_or_default());
                    let part_path_for_hash = part_path.clone();
                    let source_hash = run_blocking("transfer_verify_file_hash", move || {
                        file_hash_hex(part_path_for_hash.as_path())
                    })
                    .await?;
                    if source_hash != blake3 {
                        runtime.file.status = "failed".to_string();
                        self.blocking_insert_or_update_file(
                            runtime.file.clone(),
                            empty_bitmap(runtime.file.chunk_count),
                        )
                        .await?;
                        return Err(AppError::new("transfer_file_hash_mismatch", "文件校验失败")
                            .with_detail(format!("file_id={}", runtime.file.id)));
                    }

                    let target =
                        PathBuf::from(runtime.file.target_path.clone().unwrap_or_default());
                    let final_path = resolve_conflict_path(target.as_path());
                    if let Some(parent) = final_path.parent() {
                        tokio::fs::create_dir_all(parent).await.map_err(|error| {
                            AppError::new("transfer_target_dir_create_failed", "创建目标目录失败")
                                .with_detail(error.to_string())
                        })?;
                    }
                    tokio::fs::rename(part_path.as_path(), final_path.as_path())
                        .await
                        .map_err(|error| {
                            AppError::new("transfer_target_rename_failed", "落盘文件失败")
                                .with_detail(error.to_string())
                        })?;

                    runtime.file.target_path = Some(final_path.to_string_lossy().to_string());
                    runtime.file.preview_data = runtime.file.target_path.clone();
                    runtime.file.transferred_bytes = runtime.file.size_bytes;
                    runtime.file.status = "success".to_string();
                    session.files[file_idx] = runtime.file.clone();
                    dirty_files.insert(
                        runtime.file.id.clone(),
                        TransferFilePersistItem {
                            file: runtime.file.clone(),
                            completed_bitmap: runtime.bitmap.clone(),
                        },
                    );
                }
                TransferFrame::SessionDone {
                    session_id: incoming_session_id,
                    ok,
                    error,
                } => {
                    if incoming_session_id != session.id {
                        continue;
                    }
                    if !ack_buffer.is_empty() {
                        if ack_batch_enabled {
                            write_frame_to(
                                &mut writer,
                                &TransferFrame::AckBatch {
                                    session_id: session.id.clone(),
                                    items: std::mem::take(&mut ack_buffer),
                                },
                                Some(&session_key),
                                codec,
                            )
                            .await?;
                        } else {
                            for item in std::mem::take(&mut ack_buffer) {
                                write_frame_to(
                                    &mut writer,
                                    &TransferFrame::Ack {
                                        session_id: session.id.clone(),
                                        file_id: item.file_id,
                                        chunk_index: item.chunk_index,
                                        ok: item.ok,
                                        error: item.error,
                                    },
                                    Some(&session_key),
                                    codec,
                                )
                                .await?;
                            }
                        }
                    }

                    if !dirty_files.is_empty() {
                        let items = dirty_files.values().cloned().collect::<Vec<_>>();
                        self.blocking_upsert_files_batch(items).await?;
                    }

                    session.finished_at = Some(now_millis());
                    session.transferred_bytes = session
                        .files
                        .iter()
                        .map(|value| value.transferred_bytes)
                        .sum();
                    if ok {
                        session.status = "success".to_string();
                        session.error_code = None;
                        session.error_message = None;
                    } else {
                        session.status = "failed".to_string();
                        session.error_code = Some("remote_failed".to_string());
                        session.error_message = error;
                    }
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
                        None,
                        None,
                    );
                    self.emit_history_sync("incoming_done");
                    break;
                }
                TransferFrame::Ping { .. } => {}
                TransferFrame::Error { code, message } => {
                    session.status = "failed".to_string();
                    session.error_code = Some(code);
                    session.error_message = Some(message);
                    session.finished_at = Some(now_millis());
                    self.blocking_upsert_session_progress(session.clone())
                        .await?;
                    self.maybe_emit_session_snapshot(
                        &session,
                        None,
                        0,
                        None,
                        true,
                        Some(if codec == FrameCodec::BinV2 {
                            PROTOCOL_VERSION_V2
                        } else {
                            1
                        }),
                        Some(codec),
                        None,
                        None,
                    );
                    break;
                }
                _ => {}
            }

            if !ack_buffer.is_empty()
                && (ack_buffer.len() >= settings.ack_batch_size as usize
                    || last_ack_flush.elapsed() >= ack_flush_interval)
            {
                if ack_batch_enabled {
                    write_frame_to(
                        &mut writer,
                        &TransferFrame::AckBatch {
                            session_id: session.id.clone(),
                            items: std::mem::take(&mut ack_buffer),
                        },
                        Some(&session_key),
                        codec,
                    )
                    .await?;
                } else {
                    for item in std::mem::take(&mut ack_buffer) {
                        write_frame_to(
                            &mut writer,
                            &TransferFrame::Ack {
                                session_id: session.id.clone(),
                                file_id: item.file_id,
                                chunk_index: item.chunk_index,
                                ok: item.ok,
                                error: item.error,
                            },
                            Some(&session_key),
                            codec,
                        )
                        .await?;
                    }
                }
                last_ack_flush = Instant::now();
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
        }

        Ok(())
    }
}

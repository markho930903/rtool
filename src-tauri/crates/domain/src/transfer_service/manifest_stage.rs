use super::*;

pub(super) struct IncomingManifestContext {
    pub(super) session_id: String,
    pub(super) direction: String,
    pub(super) save_dir: String,
    pub(super) files: Vec<ManifestFileFrame>,
}

fn parse_incoming_manifest_frame(frame: TransferFrame) -> AppResult<IncomingManifestContext> {
    match frame {
        TransferFrame::Manifest {
            session_id,
            direction,
            save_dir,
            files,
        } => Ok(IncomingManifestContext {
            session_id,
            direction,
            save_dir,
            files,
        }),
        _ => Err(AppError::new(
            "transfer_protocol_manifest_invalid",
            "无效的 MANIFEST 帧",
        )),
    }
}

fn parse_outgoing_manifest_ack_frame(
    frame: TransferFrame,
    session_id: &str,
    peer_address: &str,
) -> AppResult<HashMap<String, Vec<u32>>> {
    match frame {
        TransferFrame::ManifestAck {
            session_id: ack_session_id,
            missing_chunks,
        } if ack_session_id == session_id => Ok(missing_chunks
            .into_iter()
            .map(|item| (item.file_id, item.missing_chunk_indexes))
            .collect()),
        TransferFrame::Error { code, message } => Err(AppError::new(code, "接收端拒绝文件清单")
            .with_cause(message)
            .with_context("sessionId", session_id.to_string())
            .with_context("peerAddress", peer_address.to_string())),
        other => Err(AppError::new(
            "transfer_protocol_manifest_ack_invalid",
            "MANIFEST_ACK 帧不合法",
        )
        .with_context("sessionId", session_id.to_string())
        .with_context("peerAddress", peer_address.to_string())
        .with_context("unexpectedFrame", format!("{other:?}"))),
    }
}

impl TransferService {
    fn build_manifest_files_from_session(session: &TransferSessionDto) -> Vec<ManifestFileFrame> {
        session
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
            .collect::<Vec<_>>()
    }

    pub(super) async fn read_incoming_manifest_stage<R: tokio::io::AsyncRead + Unpin>(
        reader: &mut R,
        session_key: &[u8; 32],
        codec: FrameCodec,
    ) -> AppResult<IncomingManifestContext> {
        let manifest = read_frame_from(reader, Some(session_key), Some(codec)).await?;
        parse_incoming_manifest_frame(manifest)
    }

    pub(super) async fn exchange_outgoing_manifest<
        W: tokio::io::AsyncWrite + Unpin,
        R: tokio::io::AsyncRead + Unpin,
    >(
        writer: &mut W,
        reader: &mut R,
        session: &TransferSessionDto,
        session_key: &[u8; 32],
        codec: FrameCodec,
        peer_address: &str,
    ) -> AppResult<HashMap<String, Vec<u32>>> {
        let manifest_files = Self::build_manifest_files_from_session(session);
        write_frame_to(
            writer,
            &TransferFrame::Manifest {
                session_id: session.id.clone(),
                direction: session.direction.as_str().to_string(),
                save_dir: session.save_dir.clone(),
                files: manifest_files,
            },
            Some(session_key),
            codec,
        )
        .await?;

        let manifest_ack = read_frame_from(reader, Some(session_key), Some(codec)).await?;
        parse_outgoing_manifest_ack_frame(manifest_ack, session.id.as_str(), peer_address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_incoming_manifest_frame_should_reject_non_manifest_frame() {
        let result = parse_incoming_manifest_frame(TransferFrame::Ping { ts: 1 });
        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_protocol_manifest_invalid");
    }

    #[test]
    fn parse_outgoing_manifest_ack_frame_should_return_missing_chunks() {
        let missing_by_file = parse_outgoing_manifest_ack_frame(
            TransferFrame::ManifestAck {
                session_id: "session-1".to_string(),
                missing_chunks: vec![MissingChunkFrame {
                    file_id: "file-1".to_string(),
                    missing_chunk_indexes: vec![0, 2, 5],
                }],
            },
            "session-1",
            "127.0.0.1:9000",
        )
        .expect("matching manifest ack should be accepted");

        assert_eq!(missing_by_file.get("file-1"), Some(&vec![0, 2, 5]));
    }

    #[test]
    fn parse_outgoing_manifest_ack_frame_should_reject_mismatched_session_id() {
        let result = parse_outgoing_manifest_ack_frame(
            TransferFrame::ManifestAck {
                session_id: "other-session".to_string(),
                missing_chunks: Vec::new(),
            },
            "session-1",
            "127.0.0.1:9000",
        );

        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_protocol_manifest_ack_invalid");
    }

    #[test]
    fn parse_outgoing_manifest_ack_frame_should_map_peer_error() {
        let result = parse_outgoing_manifest_ack_frame(
            TransferFrame::Error {
                code: "transfer_peer_reject".to_string(),
                message: "reject".to_string(),
            },
            "session-1",
            "127.0.0.1:9000",
        );

        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_peer_reject");
        assert_eq!(error.message, "接收端拒绝文件清单");
        assert!(!error.causes.is_empty());
    }
}

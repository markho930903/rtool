use super::*;

pub(super) struct IncomingHandshakeContext {
    pub(super) peer_device_id: String,
    pub(super) peer_name: String,
    pub(super) codec: FrameCodec,
    pub(super) ack_batch_enabled: bool,
    pub(super) session_key: [u8; 32],
}

pub(super) struct OutgoingHandshakeContext {
    pub(super) codec: FrameCodec,
    pub(super) session_key: [u8; 32],
}

fn parse_incoming_hello_frame(
    frame: TransferFrame,
) -> AppResult<(String, String, String, u16, Vec<String>)> {
    match frame {
        TransferFrame::Hello {
            device_id,
            device_name,
            nonce,
            protocol_version,
            capabilities,
        } => Ok((
            device_id,
            device_name,
            nonce,
            protocol_version.unwrap_or(1),
            capabilities.unwrap_or_default(),
        )),
        _ => Err(AppError::new(
            "transfer_protocol_hello_invalid",
            "无效的 HELLO 帧",
        )),
    }
}

fn parse_incoming_auth_response_frame(frame: TransferFrame) -> AppResult<(String, String)> {
    match frame {
        TransferFrame::AuthResponse { pair_code, proof } => Ok((pair_code, proof)),
        _ => Err(AppError::new(
            "transfer_protocol_auth_response_invalid",
            "无效的 AUTH_RESPONSE 帧",
        )),
    }
}

fn parse_outgoing_challenge_frame(
    frame: TransferFrame,
    session_id: &str,
    peer_address: &str,
) -> AppResult<String> {
    match frame {
        TransferFrame::AuthChallenge { nonce, .. } => Ok(nonce),
        TransferFrame::Error { code, message } => Err(AppError::new(code, "目标设备拒绝连接")
            .with_cause(message)
            .with_context("sessionId", session_id.to_string())
            .with_context("peerAddress", peer_address.to_string())),
        other => Err(
            AppError::new("transfer_protocol_challenge_invalid", "握手挑战帧不合法")
                .with_context("sessionId", session_id.to_string())
                .with_context("peerAddress", peer_address.to_string())
                .with_context("unexpectedFrame", format!("{other:?}")),
        ),
    }
}

fn parse_outgoing_auth_result_frame(
    frame: TransferFrame,
    session_id: &str,
    peer_address: &str,
) -> AppResult<(u16, Vec<String>)> {
    match frame {
        TransferFrame::AuthOk {
            protocol_version,
            capabilities,
            ..
        } => Ok((
            protocol_version.unwrap_or(1),
            capabilities.unwrap_or_default(),
        )),
        TransferFrame::Error { code, message } => Err(AppError::new(code, "认证失败")
            .with_cause(message)
            .with_context("sessionId", session_id.to_string())
            .with_context("peerAddress", peer_address.to_string())),
        other => Err(
            AppError::new("transfer_protocol_auth_invalid", "认证响应帧不合法")
                .with_context("sessionId", session_id.to_string())
                .with_context("peerAddress", peer_address.to_string())
                .with_context("unexpectedFrame", format!("{other:?}")),
        ),
    }
}

fn validate_peer_protocol(
    peer_protocol_version: u16,
    peer_capabilities: &[String],
    context_key: &str,
    context_value: &str,
) -> AppResult<()> {
    if peer_protocol_version != PROTOCOL_VERSION {
        return Err(AppError::new(
            "transfer_protocol_version_mismatch",
            "对端协议版本不匹配",
        )
        .with_context(context_key, context_value.to_string())
        .with_context("peerProtocolVersion", peer_protocol_version.to_string())
        .with_context("localProtocolVersion", PROTOCOL_VERSION.to_string()));
    }

    for capability in [
        CAPABILITY_CODEC_BIN,
        CAPABILITY_ACK_BATCH,
        CAPABILITY_PIPELINE,
    ] {
        if !peer_capabilities.iter().any(|value| value == capability) {
            return Err(AppError::new(
                "transfer_protocol_capability_missing",
                "对端缺少必要协议能力",
            )
            .with_context(context_key, context_value.to_string())
            .with_context("requiredCapability", capability.to_string()));
        }
    }

    Ok(())
}

impl TransferService {
    pub(super) async fn perform_incoming_handshake(
        &self,
        stream: &mut TcpStream,
        _settings: &TransferSettingsDto,
    ) -> AppResult<IncomingHandshakeContext> {
        let hello = read_frame(stream, None).await?;
        let (peer_device_id, peer_name, client_nonce, peer_protocol_version, peer_capabilities) =
            parse_incoming_hello_frame(hello)?;

        validate_peer_protocol(
            peer_protocol_version,
            peer_capabilities.as_slice(),
            "peerDeviceId",
            peer_device_id.as_str(),
        )?;

        let local_capabilities = Self::local_protocol_capabilities();
        let codec = FrameCodec::Bin;
        let ack_batch_enabled = true;
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
            stream,
            &TransferFrame::AuthChallenge {
                nonce: server_nonce.clone(),
                expires_at,
            },
            None,
        )
        .await?;

        let auth = read_frame(stream, None).await?;
        let (pair_code, proof) = parse_incoming_auth_response_frame(auth)?;

        self.validate_pair_code(peer_device_id.as_str(), pair_code.as_str())
            .await?;
        let expected = derive_proof(
            pair_code.as_str(),
            client_nonce.as_str(),
            server_nonce.as_str(),
        );
        if proof != expected {
            let pool = self.db_pool.clone();
            let peer_device_id_for_failure = peer_device_id.clone();
            run_blocking("transfer_mark_pair_failure", move || {
                mark_peer_pair_failure(
                    &pool,
                    peer_device_id_for_failure.as_str(),
                    Some(now_millis() + 60_000),
                )
            })
            .await?;
            write_frame(
                stream,
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
            stream,
            &TransferFrame::AuthOk {
                peer_device_id: self.device_id.clone(),
                peer_name: self.device_name.clone(),
                protocol_version: Some(PROTOCOL_VERSION),
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
        Ok(IncomingHandshakeContext {
            peer_device_id,
            peer_name,
            codec,
            ack_batch_enabled,
            session_key,
        })
    }

    pub(super) async fn perform_outgoing_handshake(
        &self,
        stream: &mut TcpStream,
        session_id: &str,
        peer_address: &str,
        pair_code: &str,
        _settings: &TransferSettingsDto,
    ) -> AppResult<OutgoingHandshakeContext> {
        let local_capabilities = Self::local_protocol_capabilities();
        let client_nonce = random_hex(16);
        write_frame(
            stream,
            &TransferFrame::Hello {
                device_id: self.device_id.clone(),
                device_name: self.device_name.clone(),
                nonce: client_nonce.clone(),
                protocol_version: Some(PROTOCOL_VERSION),
                capabilities: Some(local_capabilities),
            },
            None,
        )
        .await?;

        let challenge = read_frame(stream, None).await?;
        let server_nonce = parse_outgoing_challenge_frame(challenge, session_id, peer_address)?;

        let proof = derive_proof(pair_code, client_nonce.as_str(), server_nonce.as_str());
        write_frame(
            stream,
            &TransferFrame::AuthResponse {
                pair_code: pair_code.to_string(),
                proof,
            },
            None,
        )
        .await?;

        let auth_result = read_frame(stream, None).await?;
        let (peer_protocol_version, peer_capabilities) =
            parse_outgoing_auth_result_frame(auth_result, session_id, peer_address)?;

        validate_peer_protocol(
            peer_protocol_version,
            peer_capabilities.as_slice(),
            "peerAddress",
            peer_address,
        )?;

        let codec = FrameCodec::Bin;
        tracing::info!(
            event = "transfer_protocol_negotiated",
            session_id = session_id,
            peer_address,
            codec = codec.as_str(),
            peer_protocol_version
        );

        let session_key =
            derive_session_key(pair_code, client_nonce.as_str(), server_nonce.as_str());
        Ok(OutgoingHandshakeContext { codec, session_key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_incoming_hello_frame_should_reject_non_hello_frame() {
        let result = parse_incoming_hello_frame(TransferFrame::Ping { ts: 1 });
        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_protocol_hello_invalid");
    }

    #[test]
    fn parse_outgoing_challenge_frame_should_map_peer_error() {
        let result = parse_outgoing_challenge_frame(
            TransferFrame::Error {
                code: "transfer_auth_failed".to_string(),
                message: "bad pair code".to_string(),
            },
            "session-id",
            "127.0.0.1:9000",
        );
        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_auth_failed");
        assert_eq!(error.message, "目标设备拒绝连接");
        assert!(!error.causes.is_empty());
    }

    #[test]
    fn parse_outgoing_auth_result_frame_should_extract_defaults_from_auth_ok() {
        let (version, capabilities) = parse_outgoing_auth_result_frame(
            TransferFrame::AuthOk {
                peer_device_id: "peer".to_string(),
                peer_name: "peer-name".to_string(),
                protocol_version: None,
                capabilities: None,
            },
            "session-id",
            "127.0.0.1:9000",
        )
        .expect("auth ok should be accepted");

        assert_eq!(version, 1);
        assert!(capabilities.is_empty());
    }

    #[test]
    fn parse_outgoing_auth_result_frame_should_reject_unexpected_frame() {
        let result = parse_outgoing_auth_result_frame(
            TransferFrame::Ping { ts: 2 },
            "session-id",
            "127.0.0.1:9000",
        );
        assert!(result.is_err());
        let error = match result {
            Ok(_) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(error.code, "transfer_protocol_auth_invalid");
    }
}

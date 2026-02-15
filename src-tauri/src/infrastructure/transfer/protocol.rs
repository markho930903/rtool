use std::io;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::core::{AppError, AppResult};

pub const PROTOCOL_VERSION_V2: u16 = 2;
pub const CAPABILITY_CODEC_BIN_V2: &str = "codec-bin-v2";
pub const CAPABILITY_ACK_BATCH_V2: &str = "ack-batch-v2";
pub const CAPABILITY_PIPELINE_V2: &str = "pipeline-v2";

const FRAME_MAX_BYTES: usize = 16 * 1024 * 1024;
const MODE_PLAIN_JSON: u8 = 0;
const MODE_ENCRYPTED_JSON: u8 = 1;
const MODE_PLAIN_BIN: u8 = 2;
const MODE_ENCRYPTED_BIN: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameCodec {
    JsonV1,
    BinV2,
}

impl FrameCodec {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JsonV1 => "json-v1",
            Self::BinV2 => "bin-v2",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFileFrame {
    pub file_id: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub chunk_size: u32,
    pub chunk_count: u32,
    pub blake3: String,
    pub mime_type: Option<String>,
    pub is_folder_archive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingChunkFrame {
    pub file_id: String,
    pub missing_chunk_indexes: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AckFrameItem {
    pub file_id: String,
    pub chunk_index: u32,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferFrame {
    Hello {
        device_id: String,
        device_name: String,
        nonce: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        protocol_version: Option<u16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        capabilities: Option<Vec<String>>,
    },
    AuthChallenge {
        nonce: String,
        expires_at: i64,
    },
    AuthResponse {
        pair_code: String,
        proof: String,
    },
    AuthOk {
        peer_device_id: String,
        peer_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        protocol_version: Option<u16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        capabilities: Option<Vec<String>>,
    },
    Manifest {
        session_id: String,
        direction: String,
        save_dir: String,
        files: Vec<ManifestFileFrame>,
    },
    ManifestAck {
        session_id: String,
        missing_chunks: Vec<MissingChunkFrame>,
    },
    Chunk {
        session_id: String,
        file_id: String,
        chunk_index: u32,
        total_chunks: u32,
        hash: String,
        data: String,
    },
    ChunkBinary {
        session_id: String,
        file_id: String,
        chunk_index: u32,
        total_chunks: u32,
        hash: String,
        data: Vec<u8>,
    },
    Ack {
        session_id: String,
        file_id: String,
        chunk_index: u32,
        ok: bool,
        error: Option<String>,
    },
    AckBatch {
        session_id: String,
        items: Vec<AckFrameItem>,
    },
    FileDone {
        session_id: String,
        file_id: String,
        blake3: String,
    },
    SessionDone {
        session_id: String,
        ok: bool,
        error: Option<String>,
    },
    Error {
        code: String,
        message: String,
    },
    Ping {
        ts: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TransferFrameBinary {
    Hello {
        device_id: String,
        device_name: String,
        nonce: String,
        protocol_version: Option<u16>,
        capabilities: Option<Vec<String>>,
    },
    AuthChallenge {
        nonce: String,
        expires_at: i64,
    },
    AuthResponse {
        pair_code: String,
        proof: String,
    },
    AuthOk {
        peer_device_id: String,
        peer_name: String,
        protocol_version: Option<u16>,
        capabilities: Option<Vec<String>>,
    },
    Manifest {
        session_id: String,
        direction: String,
        save_dir: String,
        files: Vec<ManifestFileFrame>,
    },
    ManifestAck {
        session_id: String,
        missing_chunks: Vec<MissingChunkFrame>,
    },
    Chunk {
        session_id: String,
        file_id: String,
        chunk_index: u32,
        total_chunks: u32,
        hash: String,
        data: String,
    },
    ChunkBinary {
        session_id: String,
        file_id: String,
        chunk_index: u32,
        total_chunks: u32,
        hash: String,
        data: Vec<u8>,
    },
    Ack {
        session_id: String,
        file_id: String,
        chunk_index: u32,
        ok: bool,
        error: Option<String>,
    },
    AckBatch {
        session_id: String,
        items: Vec<AckFrameItem>,
    },
    FileDone {
        session_id: String,
        file_id: String,
        blake3: String,
    },
    SessionDone {
        session_id: String,
        ok: bool,
        error: Option<String>,
    },
    Error {
        code: String,
        message: String,
    },
    Ping {
        ts: i64,
    },
}

impl From<&TransferFrame> for TransferFrameBinary {
    fn from(value: &TransferFrame) -> Self {
        match value {
            TransferFrame::Hello {
                device_id,
                device_name,
                nonce,
                protocol_version,
                capabilities,
            } => Self::Hello {
                device_id: device_id.clone(),
                device_name: device_name.clone(),
                nonce: nonce.clone(),
                protocol_version: *protocol_version,
                capabilities: capabilities.clone(),
            },
            TransferFrame::AuthChallenge { nonce, expires_at } => Self::AuthChallenge {
                nonce: nonce.clone(),
                expires_at: *expires_at,
            },
            TransferFrame::AuthResponse { pair_code, proof } => Self::AuthResponse {
                pair_code: pair_code.clone(),
                proof: proof.clone(),
            },
            TransferFrame::AuthOk {
                peer_device_id,
                peer_name,
                protocol_version,
                capabilities,
            } => Self::AuthOk {
                peer_device_id: peer_device_id.clone(),
                peer_name: peer_name.clone(),
                protocol_version: *protocol_version,
                capabilities: capabilities.clone(),
            },
            TransferFrame::Manifest {
                session_id,
                direction,
                save_dir,
                files,
            } => Self::Manifest {
                session_id: session_id.clone(),
                direction: direction.clone(),
                save_dir: save_dir.clone(),
                files: files.clone(),
            },
            TransferFrame::ManifestAck {
                session_id,
                missing_chunks,
            } => Self::ManifestAck {
                session_id: session_id.clone(),
                missing_chunks: missing_chunks.clone(),
            },
            TransferFrame::Chunk {
                session_id,
                file_id,
                chunk_index,
                total_chunks,
                hash,
                data,
            } => Self::Chunk {
                session_id: session_id.clone(),
                file_id: file_id.clone(),
                chunk_index: *chunk_index,
                total_chunks: *total_chunks,
                hash: hash.clone(),
                data: data.clone(),
            },
            TransferFrame::ChunkBinary {
                session_id,
                file_id,
                chunk_index,
                total_chunks,
                hash,
                data,
            } => Self::ChunkBinary {
                session_id: session_id.clone(),
                file_id: file_id.clone(),
                chunk_index: *chunk_index,
                total_chunks: *total_chunks,
                hash: hash.clone(),
                data: data.clone(),
            },
            TransferFrame::Ack {
                session_id,
                file_id,
                chunk_index,
                ok,
                error,
            } => Self::Ack {
                session_id: session_id.clone(),
                file_id: file_id.clone(),
                chunk_index: *chunk_index,
                ok: *ok,
                error: error.clone(),
            },
            TransferFrame::AckBatch { session_id, items } => Self::AckBatch {
                session_id: session_id.clone(),
                items: items.clone(),
            },
            TransferFrame::FileDone {
                session_id,
                file_id,
                blake3,
            } => Self::FileDone {
                session_id: session_id.clone(),
                file_id: file_id.clone(),
                blake3: blake3.clone(),
            },
            TransferFrame::SessionDone {
                session_id,
                ok,
                error,
            } => Self::SessionDone {
                session_id: session_id.clone(),
                ok: *ok,
                error: error.clone(),
            },
            TransferFrame::Error { code, message } => Self::Error {
                code: code.clone(),
                message: message.clone(),
            },
            TransferFrame::Ping { ts } => Self::Ping { ts: *ts },
        }
    }
}

impl From<TransferFrameBinary> for TransferFrame {
    fn from(value: TransferFrameBinary) -> Self {
        match value {
            TransferFrameBinary::Hello {
                device_id,
                device_name,
                nonce,
                protocol_version,
                capabilities,
            } => Self::Hello {
                device_id,
                device_name,
                nonce,
                protocol_version,
                capabilities,
            },
            TransferFrameBinary::AuthChallenge { nonce, expires_at } => {
                Self::AuthChallenge { nonce, expires_at }
            }
            TransferFrameBinary::AuthResponse { pair_code, proof } => {
                Self::AuthResponse { pair_code, proof }
            }
            TransferFrameBinary::AuthOk {
                peer_device_id,
                peer_name,
                protocol_version,
                capabilities,
            } => Self::AuthOk {
                peer_device_id,
                peer_name,
                protocol_version,
                capabilities,
            },
            TransferFrameBinary::Manifest {
                session_id,
                direction,
                save_dir,
                files,
            } => Self::Manifest {
                session_id,
                direction,
                save_dir,
                files,
            },
            TransferFrameBinary::ManifestAck {
                session_id,
                missing_chunks,
            } => Self::ManifestAck {
                session_id,
                missing_chunks,
            },
            TransferFrameBinary::Chunk {
                session_id,
                file_id,
                chunk_index,
                total_chunks,
                hash,
                data,
            } => Self::Chunk {
                session_id,
                file_id,
                chunk_index,
                total_chunks,
                hash,
                data,
            },
            TransferFrameBinary::ChunkBinary {
                session_id,
                file_id,
                chunk_index,
                total_chunks,
                hash,
                data,
            } => Self::ChunkBinary {
                session_id,
                file_id,
                chunk_index,
                total_chunks,
                hash,
                data,
            },
            TransferFrameBinary::Ack {
                session_id,
                file_id,
                chunk_index,
                ok,
                error,
            } => Self::Ack {
                session_id,
                file_id,
                chunk_index,
                ok,
                error,
            },
            TransferFrameBinary::AckBatch { session_id, items } => {
                Self::AckBatch { session_id, items }
            }
            TransferFrameBinary::FileDone {
                session_id,
                file_id,
                blake3,
            } => Self::FileDone {
                session_id,
                file_id,
                blake3,
            },
            TransferFrameBinary::SessionDone {
                session_id,
                ok,
                error,
            } => Self::SessionDone {
                session_id,
                ok,
                error,
            },
            TransferFrameBinary::Error { code, message } => Self::Error { code, message },
            TransferFrameBinary::Ping { ts } => Self::Ping { ts },
        }
    }
}

fn app_error(code: &str, message: impl Into<String>) -> AppError {
    AppError::new(code, "文件传输协议错误").with_detail(message.into())
}

pub fn random_hex(bytes: usize) -> String {
    let mut value = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut value);
    hex_encode(value.as_slice())
}

pub fn derive_proof(pair_code: &str, client_nonce: &str, server_nonce: &str) -> String {
    blake3::hash(format!("{pair_code}:{client_nonce}:{server_nonce}").as_bytes())
        .to_hex()
        .to_string()
}

pub fn derive_session_key(pair_code: &str, client_nonce: &str, server_nonce: &str) -> [u8; 32] {
    let hash =
        blake3::hash(format!("session:{pair_code}:{client_nonce}:{server_nonce}").as_bytes());
    *hash.as_bytes()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for value in bytes {
        output.push_str(format!("{value:02x}").as_str());
    }
    output
}

fn write_u32_be(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn encrypt_payload(key_bytes: &[u8; 32], plain: &[u8]) -> AppResult<Vec<u8>> {
    let key = Key::from_slice(key_bytes);
    let cipher = XChaCha20Poly1305::new(key);

    let mut nonce_bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let cipher_text = cipher
        .encrypt(nonce, plain)
        .map_err(|error| app_error("transfer_encrypt_failed", error.to_string()))?;

    let mut output = Vec::with_capacity(24 + cipher_text.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&cipher_text);
    Ok(output)
}

fn decrypt_payload(key_bytes: &[u8; 32], payload: &[u8]) -> AppResult<Vec<u8>> {
    if payload.len() < 24 {
        return Err(app_error(
            "transfer_decrypt_payload_invalid",
            "加密包长度不足",
        ));
    }

    let (nonce_bytes, cipher_bytes) = payload.split_at(24);
    let key = Key::from_slice(key_bytes);
    let cipher = XChaCha20Poly1305::new(key);
    let nonce = XNonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, cipher_bytes)
        .map_err(|error| app_error("transfer_decrypt_failed", error.to_string()))
}

fn serialize_frame(frame: &TransferFrame, codec: FrameCodec) -> AppResult<Vec<u8>> {
    match codec {
        FrameCodec::JsonV1 => serde_json::to_vec(frame)
            .map_err(|error| app_error("transfer_frame_serialize_failed", error.to_string())),
        FrameCodec::BinV2 => {
            let binary = TransferFrameBinary::from(frame);
            bincode::serde::encode_to_vec(&binary, bincode::config::standard())
                .map_err(|error| app_error("transfer_frame_serialize_failed", error.to_string()))
        }
    }
}

fn deserialize_frame(payload: &[u8], codec: FrameCodec) -> AppResult<TransferFrame> {
    match codec {
        FrameCodec::JsonV1 => serde_json::from_slice::<TransferFrame>(payload)
            .map_err(|error| app_error("transfer_frame_parse_failed", error.to_string())),
        FrameCodec::BinV2 => bincode::serde::decode_from_slice::<TransferFrameBinary, _>(
            payload,
            bincode::config::standard(),
        )
        .map(|(frame, _)| TransferFrame::from(frame))
        .map_err(|error| app_error("transfer_frame_parse_failed", error.to_string())),
    }
}

pub async fn write_frame_to<W: AsyncWrite + Unpin>(
    writer: &mut W,
    frame: &TransferFrame,
    session_key: Option<&[u8; 32]>,
    codec: FrameCodec,
) -> AppResult<()> {
    let serialized = serialize_frame(frame, codec)?;
    let encrypted = session_key.is_some();
    let payload = if let Some(key) = session_key {
        encrypt_payload(key, serialized.as_slice())?
    } else {
        serialized
    };

    if payload.len() > FRAME_MAX_BYTES {
        return Err(app_error(
            "transfer_frame_too_large",
            format!("payload too large: {}", payload.len()),
        ));
    }

    let mode = match (encrypted, codec) {
        (false, FrameCodec::JsonV1) => MODE_PLAIN_JSON,
        (true, FrameCodec::JsonV1) => MODE_ENCRYPTED_JSON,
        (false, FrameCodec::BinV2) => MODE_PLAIN_BIN,
        (true, FrameCodec::BinV2) => MODE_ENCRYPTED_BIN,
    };

    let mut header = Vec::with_capacity(5);
    header.push(mode);
    write_u32_be(&mut header, payload.len() as u32);

    writer
        .write_all(header.as_slice())
        .await
        .map_err(io_to_error)?;
    writer
        .write_all(payload.as_slice())
        .await
        .map_err(io_to_error)?;
    Ok(())
}

pub async fn read_frame_from<R: AsyncRead + Unpin>(
    reader: &mut R,
    session_key: Option<&[u8; 32]>,
    expected_codec: Option<FrameCodec>,
) -> AppResult<TransferFrame> {
    let mut header = [0u8; 5];
    reader.read_exact(&mut header).await.map_err(io_to_error)?;

    let mode = header[0];
    let length = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
    if length == 0 || length > FRAME_MAX_BYTES {
        return Err(app_error(
            "transfer_frame_length_invalid",
            format!("invalid frame length: {length}"),
        ));
    }

    let mut payload = vec![0u8; length];
    reader
        .read_exact(payload.as_mut_slice())
        .await
        .map_err(io_to_error)?;

    let (encrypted, actual_codec) = match mode {
        MODE_PLAIN_JSON => (false, FrameCodec::JsonV1),
        MODE_ENCRYPTED_JSON => (true, FrameCodec::JsonV1),
        MODE_PLAIN_BIN => (false, FrameCodec::BinV2),
        MODE_ENCRYPTED_BIN => (true, FrameCodec::BinV2),
        _ => {
            return Err(app_error(
                "transfer_frame_mode_invalid",
                format!("invalid frame mode: {mode}"),
            ));
        }
    };

    if let Some(codec) = expected_codec
        && codec != actual_codec
    {
        return Err(app_error(
            "transfer_frame_codec_unexpected",
            format!(
                "expected codec={}, actual codec={}",
                codec.as_str(),
                actual_codec.as_str()
            ),
        ));
    }

    let plain = if encrypted {
        let key = session_key.ok_or_else(|| {
            app_error(
                "transfer_frame_unexpected_encrypted",
                "收到加密帧但会话密钥不存在",
            )
        })?;
        decrypt_payload(key, payload.as_slice())?
    } else {
        payload
    };

    deserialize_frame(plain.as_slice(), actual_codec)
}

pub async fn write_frame(
    stream: &mut TcpStream,
    frame: &TransferFrame,
    session_key: Option<&[u8; 32]>,
) -> AppResult<()> {
    write_frame_to(stream, frame, session_key, FrameCodec::JsonV1).await
}

pub async fn read_frame(
    stream: &mut TcpStream,
    session_key: Option<&[u8; 32]>,
) -> AppResult<TransferFrame> {
    read_frame_from(stream, session_key, Some(FrameCodec::JsonV1)).await
}

fn io_to_error(error: io::Error) -> AppError {
    match error.kind() {
        io::ErrorKind::UnexpectedEof
        | io::ErrorKind::ConnectionAborted
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::BrokenPipe => {
            AppError::new("transfer_connection_closed", "传输连接已断开")
                .with_detail(error.to_string())
        }
        _ => AppError::new("transfer_io_error", "文件传输 I/O 错误").with_detail(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_and_session_key_should_be_stable() {
        let proof = derive_proof("12345678", "aa", "bb");
        assert_eq!(proof.len(), 64);

        let key_1 = derive_session_key("12345678", "aa", "bb");
        let key_2 = derive_session_key("12345678", "aa", "bb");
        assert_eq!(key_1, key_2);
        assert_eq!(key_1.len(), 32);
    }

    #[test]
    fn codec_roundtrip_should_work() {
        let frame = TransferFrame::AckBatch {
            session_id: "session-1".to_string(),
            items: vec![AckFrameItem {
                file_id: "file-1".to_string(),
                chunk_index: 1,
                ok: true,
                error: None,
            }],
        };

        let json_payload = serialize_frame(&frame, FrameCodec::JsonV1).expect("json serialize");
        let json_decoded = deserialize_frame(json_payload.as_slice(), FrameCodec::JsonV1)
            .expect("json deserialize");
        assert!(matches!(json_decoded, TransferFrame::AckBatch { .. }));

        let bin_payload = serialize_frame(&frame, FrameCodec::BinV2).expect("bin serialize");
        let bin_decoded =
            deserialize_frame(bin_payload.as_slice(), FrameCodec::BinV2).expect("bin deserialize");
        assert!(matches!(bin_decoded, TransferFrame::AckBatch { .. }));
    }
}

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

    let bin_payload = serialize_frame(&frame, FrameCodec::Bin).expect("bin serialize");
    let bin_decoded =
        deserialize_frame(bin_payload.as_slice(), FrameCodec::Bin).expect("bin deserialize");
    assert!(matches!(bin_decoded, TransferFrame::AckBatch { .. }));
}

#[tokio::test]
async fn read_frame_from_should_reject_invalid_mode() {
    let (mut writer, mut reader) = tokio::io::duplex(64);
    let payload = [99_u8, 0, 0, 0, 1, 0];
    tokio::io::AsyncWriteExt::write_all(&mut writer, &payload)
        .await
        .expect("write invalid frame");

    let error = read_frame_from(&mut reader, None, Some(FrameCodec::Bin))
        .await
        .expect_err("invalid mode should fail");
    assert_eq!(error.code, "transfer_frame_mode_invalid");
}

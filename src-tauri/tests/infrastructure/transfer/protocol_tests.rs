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
    let json_decoded =
        deserialize_frame(json_payload.as_slice(), FrameCodec::JsonV1).expect("json deserialize");
    assert!(matches!(json_decoded, TransferFrame::AckBatch { .. }));

    let bin_payload = serialize_frame(&frame, FrameCodec::BinV2).expect("bin serialize");
    let bin_decoded =
        deserialize_frame(bin_payload.as_slice(), FrameCodec::BinV2).expect("bin deserialize");
    assert!(matches!(bin_decoded, TransferFrame::AckBatch { .. }));
}

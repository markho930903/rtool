use super::*;

#[test]
fn file_hash_should_match_blake3() {
    let path =
        std::env::temp_dir().join(format!("rtool-transfer-hash-{}.txt", uuid::Uuid::new_v4()));
    let payload = b"transfer-hash-test";
    std::fs::write(path.as_path(), payload).expect("write temp file");

    let expected = blake3::hash(payload).to_hex().to_string();
    let actual = file_hash_hex(path.as_path()).expect("hash file");
    assert_eq!(expected, actual);

    let _ = std::fs::remove_file(path.as_path());
}

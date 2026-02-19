use super::*;

#[tokio::test]
async fn should_reject_invalid_level() {
    let result = client_log(
        "invalid".to_string(),
        Some("req".to_string()),
        "ui".to_string(),
        "message".to_string(),
        None,
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        result.expect_err("expected error").code,
        "invalid_log_level"
    );
}

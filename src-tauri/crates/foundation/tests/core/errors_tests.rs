use super::*;
use anyhow::Context as _;
use std::io;

#[test]
fn invoke_error_should_preserve_app_error_payload() {
    let app_error = AppError::new("db_error", "数据库失败")
        .with_cause("disk full")
        .with_context("module", "storage")
        .with_request_id("req-1");

    let invoke_error = InvokeError::from(app_error.clone());
    assert_eq!(invoke_error.code, "db_error");
    assert_eq!(invoke_error.message, "数据库失败");
    assert_eq!(invoke_error.request_id.as_deref(), Some("req-1"));
    assert_eq!(invoke_error.context.len(), 1);
    assert_eq!(invoke_error.context[0].key, "module");
    assert!(!invoke_error.causes.is_empty());
}

#[test]
fn invoke_error_should_downcast_app_error_from_anyhow() {
    let app_error = AppError::new("clipboard_error", "剪贴板失败").with_cause("denied");
    let anyhow_error = anyhow::Error::new(app_error.clone());
    let invoke_error = InvokeError::from_anyhow(anyhow_error);

    assert_eq!(invoke_error.code, "clipboard_error");
    assert_eq!(invoke_error.message, "剪贴板失败");
    assert!(!invoke_error.causes.is_empty());
}

#[test]
fn invoke_error_should_collect_anyhow_context_chain() {
    let result: anyhow::Result<()> = (|| {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        Err::<(), io::Error>(io_err).context("读取配置失败")?;
        Ok(())
    })();

    let invoke_error = InvokeError::from_anyhow(result.expect_err("should fail"));
    assert_eq!(invoke_error.code, DEFAULT_CODE);
    assert!(!invoke_error.causes.is_empty());
    assert!(
        invoke_error
            .causes
            .iter()
            .any(|cause| cause.contains("读取配置失败") || cause.contains("permission denied"))
    );
}

#[test]
fn app_error_with_source_should_capture_chain_and_type() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file missing");
    let app_error = AppError::new("io_error", "I/O 失败").with_source(io_err);
    assert!(
        app_error
            .context
            .iter()
            .any(|item| item.key == "sourceType" && item.value.contains("std::io"))
    );
    assert!(
        app_error
            .context
            .iter()
            .any(|item| item.key == "sourceChainDepth" && item.value == "1")
    );
    assert!(
        app_error
            .causes
            .iter()
            .any(|cause| cause.contains("file missing"))
    );
}

#[test]
fn sanitize_cause_for_release_should_hide_sensitive_data() {
    assert_eq!(
        sanitize_cause_for_release("token=secret-value"),
        RELEASE_REDACTED_CAUSE
    );
    assert_eq!(
        sanitize_cause_for_release("/Users/example/private/file"),
        RELEASE_REDACTED_CAUSE
    );
    assert_eq!(
        sanitize_cause_for_release("normal short message"),
        "normal short message"
    );
}

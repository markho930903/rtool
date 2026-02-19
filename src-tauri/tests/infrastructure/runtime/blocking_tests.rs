use super::*;

#[tokio::test]
async fn run_blocking_should_return_value() {
    let result = run_blocking("sum", || Ok::<_, AppError>(1 + 2))
        .await
        .expect("run blocking success");
    assert_eq!(result, 3);
}

#[tokio::test]
async fn run_blocking_should_map_inner_error() {
    let result = run_blocking::<(), _>("inner_error", || {
        Err(AppError::new("inner", "inner failure"))
    })
    .await;

    assert!(result.is_err());
    assert_eq!(result.expect_err("expect err").code, "inner");
}

#[tokio::test]
async fn run_blocking_should_map_panic_error() {
    let result = run_blocking::<(), _>("panic_case", || panic!("panic in blocking job")).await;
    assert!(result.is_err());
    assert_eq!(
        result.expect_err("expect panic mapping").code,
        "blocking_task_panicked"
    );
}

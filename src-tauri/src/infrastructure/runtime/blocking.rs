use crate::core::{AppError, AppResult};

pub async fn run_blocking<T, F>(label: &'static str, job: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    let handle = tokio::task::spawn_blocking(job);
    match handle.await {
        Ok(result) => result,
        Err(error) => {
            if error.is_cancelled() {
                return Err(
                    AppError::new("blocking_task_canceled", "阻塞任务被取消").with_detail(label)
                );
            }

            if error.is_panic() {
                return Err(
                    AppError::new("blocking_task_panicked", "阻塞任务发生 panic")
                        .with_detail(format!("{label}: {}", join_error_detail(&error))),
                );
            }

            Err(AppError::new("blocking_task_failed", "阻塞任务执行失败")
                .with_detail(format!("{label}: {}", join_error_detail(&error))))
        }
    }
}

fn join_error_detail(error: &tokio::task::JoinError) -> String {
    let debug_text = format!("{error:?}");
    if debug_text.trim().is_empty() {
        "join error".to_string()
    } else {
        debug_text
    }
}

#[cfg(test)]
mod tests {
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
}

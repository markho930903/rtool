use crate::{AppError, AppResult};

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
                return Err(AppError::new("blocking_task_canceled", "阻塞任务被取消")
                    .with_context("blockingTask", label));
            }

            if error.is_panic() {
                return Err(
                    AppError::new("blocking_task_panicked", "阻塞任务发生 panic")
                        .with_context("joinError", join_error_detail(&error))
                        .with_context("blockingTask", label),
                );
            }

            Err(AppError::new("blocking_task_failed", "阻塞任务执行失败")
                .with_context("joinError", join_error_detail(&error))
                .with_context("blockingTask", label))
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


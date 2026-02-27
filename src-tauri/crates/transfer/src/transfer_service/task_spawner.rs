use std::future::Future;
use std::pin::Pin;

use tokio::task::JoinHandle;

use protocol::{AppError, AppResult};

pub type TransferTask = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub trait TransferTaskSpawner: Send + Sync {
    fn spawn(&self, task_name: &'static str, task: TransferTask) -> AppResult<JoinHandle<()>>;
}

#[derive(Default)]
pub struct TokioTransferTaskSpawner;

impl TransferTaskSpawner for TokioTransferTaskSpawner {
    fn spawn(&self, task_name: &'static str, task: TransferTask) -> AppResult<JoinHandle<()>> {
        if tokio::runtime::Handle::try_current().is_err() {
            return Err(
                AppError::new("transfer_runtime_unavailable", "传输后台任务运行时不可用")
                    .with_context("task", task_name),
            );
        }

        Ok(tokio::spawn(task))
    }
}

#[derive(Default)]
pub struct NoopTransferTaskSpawner;

impl TransferTaskSpawner for NoopTransferTaskSpawner {
    fn spawn(&self, task_name: &'static str, _task: TransferTask) -> AppResult<JoinHandle<()>> {
        Err(
            AppError::new("transfer_runtime_unavailable", "传输后台任务运行时不可用")
                .with_context("task", task_name),
        )
    }
}

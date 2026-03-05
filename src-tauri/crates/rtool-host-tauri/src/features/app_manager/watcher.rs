use crate::constants::RUNTIME_WORKER_APP_MANAGER;
use crate::host::launcher::TauriLauncherHost;
use crate::shared::command_runtime::run_blocking_command;
use rtool_app::AppManagerApplicationService;
use rtool_kernel::RuntimeBudget;
use rtool_kernel::RuntimeOrchestrator;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::Notify;
use tokio::time::sleep;

fn app_manager_watcher_started() -> &'static AtomicBool {
    static STARTED: OnceLock<AtomicBool> = OnceLock::new();
    STARTED.get_or_init(|| AtomicBool::new(false))
}

fn app_manager_watcher_notify() -> Arc<Notify> {
    static SIGNAL: OnceLock<Arc<Notify>> = OnceLock::new();
    SIGNAL.get_or_init(|| Arc::new(Notify::new())).clone()
}

pub(super) fn trigger_app_manager_watcher_refresh() {
    app_manager_watcher_notify().notify_one();
}

fn next_poll_interval(current: Duration, min_secs: u64, max_secs: u64) -> Duration {
    let doubled = current.as_secs().saturating_mul(2);
    Duration::from_secs(doubled.clamp(min_secs, max_secs))
}

pub(super) fn ensure_app_manager_watcher_started(
    app: &tauri::AppHandle,
    service: AppManagerApplicationService,
    orchestrator: RuntimeOrchestrator,
) {
    let started = app_manager_watcher_started();
    if started
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    orchestrator.mark_running(RUNTIME_WORKER_APP_MANAGER);
    orchestrator.set_queue_depth(RUNTIME_WORKER_APP_MANAGER, 0);
    let app_handle = app.clone();
    let budget = RuntimeBudget::global().clone();
    let wake_signal = app_manager_watcher_notify();
    let orchestrator_for_task = orchestrator.clone();
    tauri::async_runtime::spawn(async move {
        let mut wait_for = Duration::from_secs(budget.app_manager_poll_base_secs);
        let mut run_immediately = true;
        loop {
            if run_immediately {
                run_immediately = false;
            } else {
                tokio::select! {
                    _ = sleep(wait_for) => {}
                    _ = wake_signal.notified() => {
                        wait_for = Duration::from_secs(budget.app_manager_poll_min_secs);
                    }
                }
            }

            let host = TauriLauncherHost::new(app_handle.clone());
            let poll_result = run_blocking_command(
                "app_manager_auto_refresh_poll",
                Some("app_manager_watcher".to_string()),
                Some("main".to_string()),
                "app_manager_auto_refresh_poll",
                move || service.poll_auto_refresh(&host),
            )
            .await;
            match poll_result {
                Ok(Some(payload)) => {
                    let _ = app_handle.emit("rtool://app-manager/index-updated", payload);
                    wait_for = Duration::from_secs(budget.app_manager_poll_min_secs);
                }
                Ok(None) => {
                    wait_for = next_poll_interval(
                        wait_for,
                        budget.app_manager_poll_min_secs,
                        budget.app_manager_poll_max_secs,
                    );
                }
                Err(error) => {
                    orchestrator_for_task.mark_error(
                        RUNTIME_WORKER_APP_MANAGER,
                        format!("{}: {}", error.code, error.message),
                    );
                    tracing::debug!(
                        event = "app_manager_auto_refresh_poll_failed",
                        code = error.code.as_str(),
                        message = error.message.as_str(),
                        retry_in_secs = wait_for.as_secs()
                    );
                    wait_for = next_poll_interval(
                        wait_for,
                        budget.app_manager_poll_min_secs,
                        budget.app_manager_poll_max_secs,
                    );
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::next_poll_interval;
    use std::time::Duration;

    #[test]
    fn next_poll_interval_clamps_to_minimum() {
        assert_eq!(
            next_poll_interval(Duration::from_secs(1), 5, 120),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn next_poll_interval_doubles_within_bounds() {
        assert_eq!(
            next_poll_interval(Duration::from_secs(5), 5, 120),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn next_poll_interval_clamps_to_maximum() {
        assert_eq!(
            next_poll_interval(Duration::from_secs(70), 5, 120),
            Duration::from_secs(120)
        );
    }
}

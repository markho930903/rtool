use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerId {
    Clipboard,
    AppManager,
    Screenshot,
    Launcher,
}

impl WorkerId {
    pub const ALL: [Self; 4] = [
        Self::Clipboard,
        Self::AppManager,
        Self::Screenshot,
        Self::Launcher,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
            Self::AppManager => "app_manager",
            Self::Screenshot => "screenshot",
            Self::Launcher => "launcher",
        }
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeWorkerLifecycle {
    Starting,
    Running,
    Degraded,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct RuntimeWorkerStatus {
    pub worker: WorkerId,
    pub lifecycle: RuntimeWorkerLifecycle,
    pub running: bool,
    pub queue_depth: u32,
    pub restart_count: u32,
    pub started_at: i64,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug)]
struct WorkerState {
    lifecycle: RuntimeWorkerLifecycle,
    queue_depth: u32,
    restart_count: u32,
    started_at: i64,
    last_error: Option<String>,
}

#[derive(Clone, Default)]
pub struct RuntimeOrchestrator {
    workers: Arc<Mutex<HashMap<WorkerId, WorkerState>>>,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .unwrap_or_default()
}

impl RuntimeOrchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_worker(&self, worker: WorkerId) {
        let mut guard = self
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.entry(worker).or_insert_with(|| WorkerState {
            lifecycle: RuntimeWorkerLifecycle::Stopped,
            queue_depth: 0,
            restart_count: 0,
            started_at: now_millis(),
            last_error: None,
        });
    }

    pub fn register_workers(&self, workers: &[WorkerId]) {
        for worker in workers {
            self.register_worker(*worker);
        }
    }

    pub fn mark_running(&self, worker: WorkerId) {
        self.register_worker(worker);
        let mut guard = self
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(state) = guard.get_mut(&worker) {
            state.lifecycle = RuntimeWorkerLifecycle::Running;
            state.last_error = None;
        }
    }

    pub fn mark_stopped(&self, worker: WorkerId) {
        self.register_worker(worker);
        let mut guard = self
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(state) = guard.get_mut(&worker) {
            state.lifecycle = RuntimeWorkerLifecycle::Stopped;
            state.queue_depth = 0;
            state.last_error = None;
        }
    }

    pub fn set_queue_depth(&self, worker: WorkerId, depth: usize) {
        self.register_worker(worker);
        let mut guard = self
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(state) = guard.get_mut(&worker) {
            state.queue_depth = u32::try_from(depth).unwrap_or(u32::MAX);
        }
    }

    pub fn mark_error(&self, worker: WorkerId, error: impl Into<String>) {
        self.register_worker(worker);
        let mut guard = self
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(state) = guard.get_mut(&worker) {
            state.lifecycle = RuntimeWorkerLifecycle::Degraded;
            state.last_error = Some(error.into());
        }
    }

    pub fn mark_restarted(&self, worker: WorkerId) {
        self.register_worker(worker);
        let mut guard = self
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(state) = guard.get_mut(&worker) {
            state.restart_count = state.restart_count.saturating_add(1);
            state.lifecycle = RuntimeWorkerLifecycle::Running;
            state.started_at = now_millis();
            state.last_error = None;
        }
    }

    pub fn worker_snapshot(&self) -> Vec<RuntimeWorkerStatus> {
        let guard = self
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut snapshot = guard
            .iter()
            .map(|(worker, state)| RuntimeWorkerStatus {
                worker: *worker,
                lifecycle: state.lifecycle,
                running: matches!(state.lifecycle, RuntimeWorkerLifecycle::Running),
                queue_depth: state.queue_depth,
                restart_count: state.restart_count,
                started_at: state.started_at,
                last_error: state.last_error.clone(),
            })
            .collect::<Vec<_>>();
        snapshot.sort_by(|left, right| left.worker.cmp(&right.worker));
        snapshot
    }
}

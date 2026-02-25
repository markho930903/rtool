use app_core::models::{
    ResourceCrateIdDto, ResourceCrateStatsDto, ResourceHistoryDto, ResourceModuleIdDto,
    ResourceModuleStatsDto, ResourceOverviewDto, ResourcePointDto, ResourceSnapshotDto,
};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessesToUpdate, System};

const DEFAULT_HISTORY_LIMIT: usize = 1800;
const DEFAULT_SAMPLING_INTERVAL_MS: u64 = 1000;
const DEFAULT_DURATION_SAMPLE_LIMIT: usize = 240;

const ALL_MODULES: [ResourceModuleIdDto; 11] = [
    ResourceModuleIdDto::Launcher,
    ResourceModuleIdDto::LauncherIndex,
    ResourceModuleIdDto::LauncherFallback,
    ResourceModuleIdDto::LauncherCache,
    ResourceModuleIdDto::Clipboard,
    ResourceModuleIdDto::AppManager,
    ResourceModuleIdDto::Transfer,
    ResourceModuleIdDto::Logging,
    ResourceModuleIdDto::Locale,
    ResourceModuleIdDto::Dashboard,
    ResourceModuleIdDto::System,
];

const ALL_CRATES: [ResourceCrateIdDto; 6] = [
    ResourceCrateIdDto::LauncherApp,
    ResourceCrateIdDto::Clipboard,
    ResourceCrateIdDto::Transfer,
    ResourceCrateIdDto::Infra,
    ResourceCrateIdDto::TauriShell,
    ResourceCrateIdDto::Core,
];

static GLOBAL_MONITOR: OnceLock<Arc<ResourceMonitor>> = OnceLock::new();

pub type TickCallback = Arc<dyn Fn(i64) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy)]
pub struct MonitorOptions {
    pub history_limit: usize,
    pub sampling_interval_ms: u64,
    pub duration_sample_limit: usize,
}

impl Default for MonitorOptions {
    fn default() -> Self {
        Self {
            history_limit: DEFAULT_HISTORY_LIMIT,
            sampling_interval_ms: DEFAULT_SAMPLING_INTERVAL_MS,
            duration_sample_limit: DEFAULT_DURATION_SAMPLE_LIMIT,
        }
    }
}

#[derive(Debug, Clone)]
struct InflightCommand {
    module_id: ResourceModuleIdDto,
}

#[derive(Debug, Default, Clone)]
struct ModuleAccumulator {
    calls: u64,
    error_calls: u64,
    duration_sum_ms: u64,
    duration_samples: VecDeque<u64>,
    duration_sample_sum_ms: u64,
    last_seen_at: Option<i64>,
}

impl ModuleAccumulator {
    fn push_duration(&mut self, duration_ms: u64, max_samples: usize) {
        self.duration_samples.push_back(duration_ms);
        self.duration_sample_sum_ms = self.duration_sample_sum_ms.saturating_add(duration_ms);
        while self.duration_samples.len() > max_samples {
            let Some(removed) = self.duration_samples.pop_front() else {
                break;
            };
            self.duration_sample_sum_ms = self.duration_sample_sum_ms.saturating_sub(removed);
        }
    }
}

fn record_module_observation_internal(
    state: &mut MonitorState,
    module_id: ResourceModuleIdDto,
    success: bool,
    duration_ms: u64,
    max_samples: usize,
    observed_at: i64,
) {
    let duration_ms = duration_ms.max(1);
    let accumulator = state.modules.entry(module_id).or_default();
    accumulator.calls = accumulator.calls.saturating_add(1);
    if !success {
        accumulator.error_calls = accumulator.error_calls.saturating_add(1);
    }
    accumulator.duration_sum_ms = accumulator.duration_sum_ms.saturating_add(duration_ms);
    accumulator.push_duration(duration_ms, max_samples);
    accumulator.last_seen_at = Some(observed_at);
}

#[derive(Debug)]
struct MonitorState {
    history: VecDeque<ResourcePointDto>,
    history_limit: usize,
    inflight: HashMap<String, InflightCommand>,
    modules: HashMap<ResourceModuleIdDto, ModuleAccumulator>,
    last_point: Option<ResourcePointDto>,
}

impl MonitorState {
    fn new(history_limit: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(history_limit),
            history_limit,
            inflight: HashMap::new(),
            modules: HashMap::new(),
            last_point: None,
        }
    }

    fn push_history(&mut self, point: ResourcePointDto) {
        self.last_point = Some(point.clone());
        self.history.push_back(point);
        while self.history.len() > self.history_limit {
            self.history.pop_front();
        }
    }

    fn reset(&mut self) {
        self.history.clear();
        self.inflight.clear();
        self.modules.clear();
        self.last_point = None;
    }
}

#[derive(Debug)]
pub struct ResourceMonitor {
    options: MonitorOptions,
    state: Mutex<MonitorState>,
    sampler_started: AtomicBool,
}

impl ResourceMonitor {
    fn new(options: MonitorOptions) -> Self {
        let history_limit = options.history_limit.max(1);
        Self {
            options: MonitorOptions {
                history_limit,
                sampling_interval_ms: options.sampling_interval_ms.max(200),
                duration_sample_limit: options.duration_sample_limit.max(16),
            },
            state: Mutex::new(MonitorState::new(history_limit)),
            sampler_started: AtomicBool::new(false),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, MonitorState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

pub fn initialize_global_monitor(options: MonitorOptions) -> Arc<ResourceMonitor> {
    let monitor = GLOBAL_MONITOR.get_or_init(|| Arc::new(ResourceMonitor::new(options)));
    Arc::clone(monitor)
}

pub fn start_sampling(callback: Option<TickCallback>) -> bool {
    let Some(monitor) = GLOBAL_MONITOR.get().cloned() else {
        return false;
    };
    if monitor.sampler_started.swap(true, Ordering::AcqRel) {
        return false;
    }

    let interval = Duration::from_millis(monitor.options.sampling_interval_ms);
    let monitor_for_thread = Arc::clone(&monitor);
    let spawn_result = thread::Builder::new()
        .name("resource-monitor-sampler".to_string())
        .spawn(move || {
            let mut system = System::new_all();
            let pid = Pid::from_u32(std::process::id());
            loop {
                let point = collect_point(&mut system, pid);
                record_sample_internal(&monitor_for_thread, point.clone());
                if let Some(handler) = callback.as_ref() {
                    handler(point.sampled_at);
                }
                thread::sleep(interval);
            }
        });

    if spawn_result.is_err() {
        monitor.sampler_started.store(false, Ordering::Release);
        return false;
    }

    true
}

pub fn record_command_start(command: &str, request_id: &str) {
    if should_skip_command(command) {
        return;
    }
    if request_id.trim().is_empty() {
        return;
    }
    let Some(monitor) = GLOBAL_MONITOR.get() else {
        return;
    };
    let module_id = command_to_module(command);
    let mut state = monitor.lock_state();
    state
        .inflight
        .insert(request_id.to_string(), InflightCommand { module_id });
}

pub fn record_command_end(command: &str, request_id: &str, success: bool, duration_ms: u64) {
    if should_skip_command(command) {
        return;
    }
    let Some(monitor) = GLOBAL_MONITOR.get() else {
        return;
    };
    let mut state = monitor.lock_state();
    let fallback = command_to_module(command);
    let module_id = state
        .inflight
        .remove(request_id)
        .map(|item| item.module_id)
        .unwrap_or(fallback);
    record_module_observation_internal(
        &mut state,
        module_id,
        success,
        duration_ms,
        monitor.options.duration_sample_limit,
        now_ms(),
    );
}

pub fn record_module_observation(module_id: ResourceModuleIdDto, success: bool, duration_ms: u64) {
    let Some(monitor) = GLOBAL_MONITOR.get() else {
        return;
    };
    let mut state = monitor.lock_state();
    record_module_observation_internal(
        &mut state,
        module_id,
        success,
        duration_ms,
        monitor.options.duration_sample_limit,
        now_ms(),
    );
}

pub fn snapshot() -> ResourceSnapshotDto {
    let Some(monitor) = GLOBAL_MONITOR.get() else {
        return empty_snapshot(now_ms());
    };
    let state = monitor.lock_state();
    build_snapshot(&state)
}

pub fn history(limit: Option<usize>) -> ResourceHistoryDto {
    let Some(monitor) = GLOBAL_MONITOR.get() else {
        return empty_history(0, DEFAULT_SAMPLING_INTERVAL_MS);
    };
    let state = monitor.lock_state();
    let requested = limit.unwrap_or(state.history_limit).max(1);
    let skip = state.history.len().saturating_sub(requested);
    let points: Vec<ResourcePointDto> = state.history.iter().skip(skip).cloned().collect();
    ResourceHistoryDto {
        window_ms: monitor
            .options
            .sampling_interval_ms
            .saturating_mul(points.len() as u64),
        step_ms: monitor.options.sampling_interval_ms,
        points,
    }
}

pub fn reset_session() {
    let Some(monitor) = GLOBAL_MONITOR.get() else {
        return;
    };
    let mut state = monitor.lock_state();
    state.reset();
}

fn record_sample_internal(monitor: &ResourceMonitor, point: ResourcePointDto) {
    let mut state = monitor.lock_state();
    state.push_history(point);
}

fn collect_point(system: &mut System, pid: Pid) -> ResourcePointDto {
    system.refresh_cpu_usage();
    system.refresh_memory();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let sampled_at = now_ms();
    let process = system.process(pid);
    ResourcePointDto {
        sampled_at,
        process_cpu_percent: process.map(|item| round2(f64::from(item.cpu_usage()))),
        process_memory_bytes: process.map(|item| item.memory()).filter(|value| *value > 0),
        system_used_memory_bytes: non_zero(system.used_memory()),
        system_total_memory_bytes: non_zero(system.total_memory()),
    }
}

fn build_snapshot(state: &MonitorState) -> ResourceSnapshotDto {
    let base_point = state
        .last_point
        .clone()
        .unwrap_or_else(|| empty_point(now_ms()));
    let total_duration: u64 = state
        .modules
        .values()
        .map(|item| item.duration_sample_sum_ms)
        .sum();

    let modules = ALL_MODULES
        .iter()
        .copied()
        .map(|module_id| {
            let acc = state.modules.get(&module_id);
            let calls = acc.map(|item| item.calls).unwrap_or_default();
            let error_calls = acc.map(|item| item.error_calls).unwrap_or_default();
            let avg_duration_ms =
                acc.and_then(|item| average_duration(item.calls, item.duration_sum_ms));
            let p95_duration_ms = acc.and_then(|item| percentile95(&item.duration_samples));
            let active_share_percent = acc.and_then(|item| {
                if total_duration == 0 {
                    return None;
                }
                Some(round2(
                    (item.duration_sample_sum_ms as f64 / total_duration as f64) * 100.0,
                ))
            });
            let estimated_cpu_percent = match (active_share_percent, base_point.process_cpu_percent)
            {
                (Some(share), Some(cpu)) => Some(round2(cpu * share / 100.0)),
                _ => None,
            };
            let estimated_memory_bytes =
                match (active_share_percent, base_point.process_memory_bytes) {
                    (Some(share), Some(memory)) => {
                        Some(((memory as f64) * (share / 100.0)).round() as u64)
                    }
                    _ => None,
                };
            ResourceModuleStatsDto {
                module_id,
                calls,
                error_calls,
                avg_duration_ms,
                p95_duration_ms,
                active_share_percent,
                estimated_cpu_percent,
                estimated_memory_bytes,
                last_seen_at: acc.and_then(|item| item.last_seen_at),
            }
        })
        .collect::<Vec<_>>();

    let crates = aggregate_crates(&modules);

    ResourceSnapshotDto {
        sampled_at: base_point.sampled_at,
        overview: ResourceOverviewDto {
            sampled_at: base_point.sampled_at,
            process_cpu_percent: base_point.process_cpu_percent,
            process_memory_bytes: base_point.process_memory_bytes,
            system_used_memory_bytes: base_point.system_used_memory_bytes,
            system_total_memory_bytes: base_point.system_total_memory_bytes,
        },
        modules,
        crates,
    }
}

fn aggregate_crates(modules: &[ResourceModuleStatsDto]) -> Vec<ResourceCrateStatsDto> {
    #[derive(Default)]
    struct CrateAcc {
        calls: u64,
        error_calls: u64,
        weighted_duration_sum: f64,
        weighted_duration_calls: u64,
        p95_duration_ms: Option<u64>,
        active_share_percent: f64,
        estimated_cpu_percent: f64,
        estimated_memory_bytes: u64,
        has_share: bool,
        has_cpu: bool,
        has_memory: bool,
    }

    let mut map: HashMap<ResourceCrateIdDto, CrateAcc> = HashMap::new();
    for module in modules {
        let crate_id = module_to_crate(module.module_id);
        let acc = map.entry(crate_id).or_default();
        acc.calls = acc.calls.saturating_add(module.calls);
        acc.error_calls = acc.error_calls.saturating_add(module.error_calls);
        if let Some(avg) = module.avg_duration_ms {
            acc.weighted_duration_sum += avg * module.calls as f64;
            acc.weighted_duration_calls = acc.weighted_duration_calls.saturating_add(module.calls);
        }
        acc.p95_duration_ms = match (acc.p95_duration_ms, module.p95_duration_ms) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (None, Some(right)) => Some(right),
            (left, None) => left,
        };
        if let Some(share) = module.active_share_percent {
            acc.active_share_percent += share;
            acc.has_share = true;
        }
        if let Some(cpu) = module.estimated_cpu_percent {
            acc.estimated_cpu_percent += cpu;
            acc.has_cpu = true;
        }
        if let Some(memory) = module.estimated_memory_bytes {
            acc.estimated_memory_bytes = acc.estimated_memory_bytes.saturating_add(memory);
            acc.has_memory = true;
        }
    }

    ALL_CRATES
        .iter()
        .copied()
        .map(|crate_id| {
            let acc = map.remove(&crate_id).unwrap_or_default();
            ResourceCrateStatsDto {
                crate_id,
                calls: acc.calls,
                error_calls: acc.error_calls,
                avg_duration_ms: if acc.weighted_duration_calls == 0 {
                    None
                } else {
                    Some(round2(
                        acc.weighted_duration_sum / acc.weighted_duration_calls as f64,
                    ))
                },
                p95_duration_ms: acc.p95_duration_ms,
                active_share_percent: if acc.has_share {
                    Some(round2(acc.active_share_percent))
                } else {
                    None
                },
                estimated_cpu_percent: if acc.has_cpu {
                    Some(round2(acc.estimated_cpu_percent))
                } else {
                    None
                },
                estimated_memory_bytes: if acc.has_memory {
                    Some(acc.estimated_memory_bytes)
                } else {
                    None
                },
            }
        })
        .collect()
}

fn empty_snapshot(sampled_at: i64) -> ResourceSnapshotDto {
    ResourceSnapshotDto {
        sampled_at,
        overview: ResourceOverviewDto {
            sampled_at,
            process_cpu_percent: None,
            process_memory_bytes: None,
            system_used_memory_bytes: None,
            system_total_memory_bytes: None,
        },
        modules: ALL_MODULES
            .iter()
            .copied()
            .map(|module_id| ResourceModuleStatsDto {
                module_id,
                calls: 0,
                error_calls: 0,
                avg_duration_ms: None,
                p95_duration_ms: None,
                active_share_percent: None,
                estimated_cpu_percent: None,
                estimated_memory_bytes: None,
                last_seen_at: None,
            })
            .collect(),
        crates: ALL_CRATES
            .iter()
            .copied()
            .map(|crate_id| ResourceCrateStatsDto {
                crate_id,
                calls: 0,
                error_calls: 0,
                avg_duration_ms: None,
                p95_duration_ms: None,
                active_share_percent: None,
                estimated_cpu_percent: None,
                estimated_memory_bytes: None,
            })
            .collect(),
    }
}

fn empty_history(window_ms: u64, step_ms: u64) -> ResourceHistoryDto {
    ResourceHistoryDto {
        points: Vec::new(),
        window_ms,
        step_ms,
    }
}

fn empty_point(sampled_at: i64) -> ResourcePointDto {
    ResourcePointDto {
        sampled_at,
        process_cpu_percent: None,
        process_memory_bytes: None,
        system_used_memory_bytes: None,
        system_total_memory_bytes: None,
    }
}

fn now_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok());
    millis.unwrap_or_default()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn non_zero(value: u64) -> Option<u64> {
    if value == 0 {
        return None;
    }
    Some(value)
}

fn average_duration(calls: u64, duration_sum_ms: u64) -> Option<f64> {
    if calls == 0 {
        return None;
    }
    Some(round2(duration_sum_ms as f64 / calls as f64))
}

fn percentile95(samples: &VecDeque<u64>) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.iter().copied().collect::<Vec<_>>();
    sorted.sort_unstable();
    let rank = ((sorted.len() as f64) * 0.95).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len().saturating_sub(1));
    sorted.get(index).copied()
}

fn should_skip_command(command: &str) -> bool {
    command.starts_with("resource_monitor_")
}

fn command_to_module(command: &str) -> ResourceModuleIdDto {
    if command.starts_with("launcher_") {
        return ResourceModuleIdDto::Launcher;
    }
    if command.starts_with("clipboard_") {
        return ResourceModuleIdDto::Clipboard;
    }
    if command.starts_with("app_manager_") {
        return ResourceModuleIdDto::AppManager;
    }
    if command.starts_with("transfer_") {
        return ResourceModuleIdDto::Transfer;
    }
    if command.starts_with("logging_") {
        return ResourceModuleIdDto::Logging;
    }
    if command == "dashboard_snapshot" {
        return ResourceModuleIdDto::Dashboard;
    }
    if command.starts_with("app_") && command.contains("locale") {
        return ResourceModuleIdDto::Locale;
    }
    ResourceModuleIdDto::System
}

fn module_to_crate(module_id: ResourceModuleIdDto) -> ResourceCrateIdDto {
    match module_id {
        ResourceModuleIdDto::Launcher
        | ResourceModuleIdDto::LauncherIndex
        | ResourceModuleIdDto::LauncherFallback
        | ResourceModuleIdDto::LauncherCache
        | ResourceModuleIdDto::AppManager => ResourceCrateIdDto::LauncherApp,
        ResourceModuleIdDto::Clipboard => ResourceCrateIdDto::Clipboard,
        ResourceModuleIdDto::Transfer => ResourceCrateIdDto::Transfer,
        ResourceModuleIdDto::Logging => ResourceCrateIdDto::Infra,
        ResourceModuleIdDto::Locale => ResourceCrateIdDto::Core,
        ResourceModuleIdDto::Dashboard | ResourceModuleIdDto::System => {
            ResourceCrateIdDto::TauriShell
        }
    }
}

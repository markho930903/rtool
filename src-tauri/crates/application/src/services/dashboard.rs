use app_core::AppResult;
use app_core::models::{
    AppHealthSnapshotDto, AppRuntimeInfoDto, DashboardSnapshotDto, SystemInfoDto,
    TransferRuntimeStatusDto,
};
use app_launcher_app::launcher::index::get_indexer_runtime_status;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessesToUpdate, System};

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn non_zero_u64(value: u64) -> Option<u64> {
    if value == 0 {
        return None;
    }

    Some(value)
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DashboardApplicationService;

impl DashboardApplicationService {
    pub fn snapshot(
        self,
        app_name: String,
        app_version: String,
        build_mode: String,
        uptime_seconds: u64,
        db_path: PathBuf,
    ) -> AppResult<DashboardSnapshotDto> {
        let sampled_at = now_millis();

        let mut system = System::new_all();
        system.refresh_memory();

        let current_pid = Pid::from_u32(std::process::id());
        system.refresh_processes(ProcessesToUpdate::Some(&[current_pid]), true);

        let process_memory_bytes = system.process(current_pid).map(|process| process.memory());

        let app = AppRuntimeInfoDto {
            app_name,
            app_version,
            build_mode,
            uptime_seconds,
            process_memory_bytes,
            database_size_bytes: std::fs::metadata(&db_path)
                .ok()
                .map(|metadata| metadata.len()),
        };

        let cpu_brand = system
            .cpus()
            .first()
            .and_then(|cpu| non_empty_string(cpu.brand().to_string()));

        let cpu_cores = System::physical_core_count()
            .or_else(|| {
                let fallback = system.cpus().len();
                if fallback == 0 { None } else { Some(fallback) }
            })
            .and_then(|count| u32::try_from(count).ok());

        let system = SystemInfoDto {
            os_name: System::name(),
            os_version: System::os_version(),
            kernel_version: System::kernel_version(),
            arch: non_empty_string(System::cpu_arch()),
            host_name: System::host_name(),
            cpu_brand,
            cpu_cores,
            total_memory_bytes: non_zero_u64(system.total_memory()),
            used_memory_bytes: non_zero_u64(system.used_memory()),
        };

        Ok(DashboardSnapshotDto {
            sampled_at,
            app,
            system,
        })
    }

    pub fn health_snapshot(
        self,
        transfer: TransferRuntimeStatusDto,
    ) -> AppResult<AppHealthSnapshotDto> {
        Ok(AppHealthSnapshotDto {
            sampled_at: now_millis(),
            transfer,
            launcher: get_indexer_runtime_status(),
        })
    }
}

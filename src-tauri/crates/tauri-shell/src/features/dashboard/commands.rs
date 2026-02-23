use super::run_blocking_command;
use crate::app::state::AppState;
use app_core::InvokeError;
use app_core::models::{AppRuntimeInfoDto, DashboardSnapshotDto, SystemInfoDto};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::State;

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

#[tauri::command]
pub async fn dashboard_snapshot(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<DashboardSnapshotDto, InvokeError> {
    let uptime_seconds = state.started_at.elapsed().as_secs();
    let db_path = state.db_path.clone();
    run_blocking_command(
        "dashboard_snapshot",
        request_id,
        window_label,
        "dashboard_snapshot",
        move || {
            let sampled_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| i64::try_from(duration.as_millis()).ok())
                .unwrap_or_default();

            let mut system = System::new_all();
            system.refresh_memory();

            let current_pid = Pid::from_u32(std::process::id());
            system.refresh_processes(ProcessesToUpdate::Some(&[current_pid]), true);

            let process_memory_bytes = system.process(current_pid).map(|process| process.memory());

            let app_info = AppRuntimeInfoDto {
                app_name: env!("CARGO_PKG_NAME").to_string(),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                build_mode: if cfg!(debug_assertions) {
                    "debug".to_string()
                } else {
                    "release".to_string()
                },
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

            let system_info = SystemInfoDto {
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
                app: app_info,
                system: system_info,
            })
        },
    )
    .await
}

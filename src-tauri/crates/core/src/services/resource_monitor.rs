use protocol::models::{
    ActionResultDto, ResourceHistoryDto, ResourceModuleIdDto, ResourceSnapshotDto,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceMonitorApplicationService;

impl ResourceMonitorApplicationService {
    pub fn snapshot(self) -> ResourceSnapshotDto {
        rtool_system::snapshot()
    }

    pub fn history(self, limit: Option<u32>) -> ResourceHistoryDto {
        let limit = limit
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value > 0);
        rtool_system::history(limit)
    }

    pub fn reset_session(self) -> ActionResultDto {
        rtool_system::reset_session();
        ActionResultDto {
            ok: true,
            message: "resource monitor session reset".to_string(),
        }
    }

    pub fn record_module_observation(
        self,
        module_id: ResourceModuleIdDto,
        success: bool,
        duration_ms: u64,
    ) {
        rtool_system::record_module_observation(module_id, success, duration_ms);
    }
}

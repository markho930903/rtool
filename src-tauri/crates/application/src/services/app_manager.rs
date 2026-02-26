use app_core::AppResult;
use app_core::models::{
    AppManagerActionResultDto, AppManagerCleanupInputDto, AppManagerCleanupResultDto,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerIndexUpdatedPayloadDto, AppManagerPageDto, AppManagerQueryDto,
    AppManagerResidueScanInputDto, AppManagerResidueScanResultDto, AppManagerResolveSizesInputDto,
    AppManagerResolveSizesResultDto, AppManagerSnapshotMetaDto, AppManagerStartupUpdateInputDto,
    AppManagerUninstallInputDto, ManagedAppDetailDto,
};
use app_launcher_app::app_manager::{
    cleanup_managed_app_residue, export_managed_app_scan_result, get_managed_app_detail,
    get_managed_app_detail_core, get_managed_app_detail_heavy, list_managed_apps,
    list_managed_apps_snapshot_meta, open_uninstall_help, poll_managed_apps_auto_refresh,
    refresh_managed_apps_index, resolve_managed_app_sizes, scan_managed_app_residue,
    set_managed_app_startup, uninstall_managed_app,
};
use app_launcher_app::host::LauncherHost;

#[derive(Debug, Clone, Copy, Default)]
pub struct AppManagerApplicationService;

impl AppManagerApplicationService {
    pub fn list(
        self,
        host: &dyn LauncherHost,
        query: AppManagerQueryDto,
    ) -> AppResult<AppManagerPageDto> {
        list_managed_apps(host, query)
    }

    pub fn list_snapshot_meta(
        self,
        host: &dyn LauncherHost,
    ) -> AppResult<AppManagerSnapshotMetaDto> {
        list_managed_apps_snapshot_meta(host)
    }

    pub fn resolve_sizes(
        self,
        host: &dyn LauncherHost,
        input: AppManagerResolveSizesInputDto,
    ) -> AppResult<AppManagerResolveSizesResultDto> {
        resolve_managed_app_sizes(host, input)
    }

    pub fn get_detail(
        self,
        host: &dyn LauncherHost,
        query: AppManagerDetailQueryDto,
    ) -> AppResult<ManagedAppDetailDto> {
        get_managed_app_detail(host, query)
    }

    pub fn get_detail_core(
        self,
        host: &dyn LauncherHost,
        query: AppManagerDetailQueryDto,
    ) -> AppResult<ManagedAppDetailDto> {
        get_managed_app_detail_core(host, query)
    }

    pub fn get_detail_heavy(
        self,
        host: &dyn LauncherHost,
        input: AppManagerResidueScanInputDto,
    ) -> AppResult<AppManagerResidueScanResultDto> {
        get_managed_app_detail_heavy(host, input)
    }

    pub fn scan_residue(
        self,
        host: &dyn LauncherHost,
        input: AppManagerResidueScanInputDto,
    ) -> AppResult<AppManagerResidueScanResultDto> {
        scan_managed_app_residue(host, input)
    }

    pub fn cleanup(
        self,
        host: &dyn LauncherHost,
        input: AppManagerCleanupInputDto,
    ) -> AppResult<AppManagerCleanupResultDto> {
        cleanup_managed_app_residue(host, input)
    }

    pub fn export_scan_result(
        self,
        host: &dyn LauncherHost,
        input: AppManagerExportScanInputDto,
    ) -> AppResult<AppManagerExportScanResultDto> {
        export_managed_app_scan_result(host, input)
    }

    pub fn refresh_index(self, host: &dyn LauncherHost) -> AppResult<AppManagerActionResultDto> {
        refresh_managed_apps_index(host)
    }

    pub fn set_startup(
        self,
        host: &dyn LauncherHost,
        input: AppManagerStartupUpdateInputDto,
    ) -> AppResult<AppManagerActionResultDto> {
        set_managed_app_startup(host, input)
    }

    pub fn uninstall(
        self,
        host: &dyn LauncherHost,
        input: AppManagerUninstallInputDto,
    ) -> AppResult<AppManagerActionResultDto> {
        uninstall_managed_app(host, input)
    }

    pub fn open_uninstall_help(
        self,
        host: &dyn LauncherHost,
        app_id: String,
    ) -> AppResult<AppManagerActionResultDto> {
        open_uninstall_help(host, app_id)
    }

    pub fn poll_auto_refresh(
        self,
        host: &dyn LauncherHost,
    ) -> AppResult<Option<AppManagerIndexUpdatedPayloadDto>> {
        poll_managed_apps_auto_refresh(host)
    }
}

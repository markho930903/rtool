use rtool_contracts::AppResult;
use rtool_contracts::models::{
    AppManagerActionResultDto, AppManagerCleanupInputDto, AppManagerCleanupResultDto,
    AppManagerDetailQueryDto, AppManagerExportScanInputDto, AppManagerExportScanResultDto,
    AppManagerIndexUpdatedPayloadDto, AppManagerPageDto, AppManagerQueryDto,
    AppManagerResidueScanInputDto, AppManagerResidueScanResultDto, AppManagerResolveSizesInputDto,
    AppManagerResolveSizesResultDto, AppManagerSnapshotMetaDto, AppManagerStartupUpdateInputDto,
    AppManagerUninstallInputDto, ManagedAppDetailDto,
};
use rtool_discovery::app_manager::{
    cleanup_managed_app_residue, export_managed_app_scan_result, get_managed_app_detail_core,
    get_managed_app_detail_heavy, list_managed_apps, list_managed_apps_snapshot_meta,
    open_permission_help, open_uninstall_help, poll_managed_apps_auto_refresh,
    refresh_managed_apps_index, resolve_managed_app_sizes, set_managed_app_startup,
    uninstall_managed_app,
};
use rtool_platform::launcher::LauncherHost;

#[derive(Debug, Clone, Copy, Default)]
pub struct AppManagerApplicationService;

macro_rules! forward_no_arg {
    ($name:ident, $result:ty, $target:path) => {
        pub fn $name(self, host: &dyn LauncherHost) -> AppResult<$result> {
            $target(host)
        }
    };
}

macro_rules! forward_with_arg {
    ($name:ident, $arg:ident : $arg_ty:ty, $result:ty, $target:path) => {
        pub fn $name(self, host: &dyn LauncherHost, $arg: $arg_ty) -> AppResult<$result> {
            $target(host, $arg)
        }
    };
}

impl AppManagerApplicationService {
    forward_with_arg!(list, query: AppManagerQueryDto, AppManagerPageDto, list_managed_apps);
    forward_no_arg!(
        list_snapshot_meta,
        AppManagerSnapshotMetaDto,
        list_managed_apps_snapshot_meta
    );
    forward_with_arg!(
        resolve_sizes,
        input: AppManagerResolveSizesInputDto,
        AppManagerResolveSizesResultDto,
        resolve_managed_app_sizes
    );
    forward_with_arg!(
        get_detail_core,
        query: AppManagerDetailQueryDto,
        ManagedAppDetailDto,
        get_managed_app_detail_core
    );
    forward_with_arg!(
        get_detail_heavy,
        input: AppManagerResidueScanInputDto,
        AppManagerResidueScanResultDto,
        get_managed_app_detail_heavy
    );
    forward_with_arg!(
        cleanup,
        input: AppManagerCleanupInputDto,
        AppManagerCleanupResultDto,
        cleanup_managed_app_residue
    );
    forward_with_arg!(
        export_scan_result,
        input: AppManagerExportScanInputDto,
        AppManagerExportScanResultDto,
        export_managed_app_scan_result
    );
    forward_no_arg!(
        refresh_index,
        AppManagerActionResultDto,
        refresh_managed_apps_index
    );
    forward_with_arg!(
        set_startup,
        input: AppManagerStartupUpdateInputDto,
        AppManagerActionResultDto,
        set_managed_app_startup
    );
    forward_with_arg!(
        uninstall,
        input: AppManagerUninstallInputDto,
        AppManagerActionResultDto,
        uninstall_managed_app
    );
    forward_with_arg!(
        open_uninstall_help,
        app_id: String,
        AppManagerActionResultDto,
        open_uninstall_help
    );
    forward_with_arg!(
        open_permission_help,
        app_id: String,
        AppManagerActionResultDto,
        open_permission_help
    );
    forward_no_arg!(
        poll_auto_refresh,
        Option<AppManagerIndexUpdatedPayloadDto>,
        poll_managed_apps_auto_refresh
    );
}

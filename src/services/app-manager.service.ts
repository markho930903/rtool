import type {
  AppManagerActionResult,
  AppManagerCleanupInput,
  AppManagerCleanupResult,
  AppManagerExportScanResult,
  AppManagerResolveSizesInput,
  AppManagerResolveSizesResult,
  AppManagerResidueScanResult,
  AppManagerSnapshotMeta,
  ManagedAppDetail,
  AppManagerPage,
  AppManagerQuery,
  AppManagerResidueScanMode,
  AppManagerStartupUpdateInput,
  AppManagerUninstallInput,
} from "@/components/app-manager/types";
import type {
  AppManagerActionResultDto,
  AppManagerCleanupInputDto,
  AppManagerCleanupResultDto,
  AppManagerExportScanResultDto,
  AppManagerPageDto,
  AppManagerQueryDto,
  AppManagerResolveSizesInputDto,
  AppManagerResolveSizesResultDto,
  AppManagerResidueScanResultDto,
  AppManagerSnapshotMetaDto,
  AppManagerStartupUpdateInputDto,
  AppManagerUninstallInputDto,
  ManagedAppDetailDto,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export async function appManagerList(query?: AppManagerQuery): Promise<AppManagerPage> {
  const dto = await invokeWithLog<AppManagerPageDto>("app_manager_list", {
    query: query as AppManagerQueryDto | undefined,
  });
  return dto as AppManagerPage;
}

export async function appManagerListSnapshotMeta(): Promise<AppManagerSnapshotMeta> {
  const dto = await invokeWithLog<AppManagerSnapshotMetaDto>("app_manager_list_snapshot_meta");
  return dto as AppManagerSnapshotMeta;
}

export async function appManagerResolveSizes(
  input: AppManagerResolveSizesInput,
): Promise<AppManagerResolveSizesResult> {
  const dto = await invokeWithLog<AppManagerResolveSizesResultDto>("app_manager_resolve_sizes", {
    input: input as AppManagerResolveSizesInputDto,
  });
  return dto as AppManagerResolveSizesResult;
}

export async function appManagerRefreshIndex(): Promise<AppManagerActionResult> {
  const dto = await invokeWithLog<AppManagerActionResultDto>("app_manager_refresh_index");
  return dto as AppManagerActionResult;
}

export async function appManagerGetDetail(appId: string): Promise<ManagedAppDetail> {
  const dto = await invokeWithLog<ManagedAppDetailDto>("app_manager_get_detail", { query: { appId } });
  return dto as ManagedAppDetail;
}

export async function appManagerGetDetailCore(appId: string): Promise<ManagedAppDetail> {
  const dto = await invokeWithLog<ManagedAppDetailDto>("app_manager_get_detail_core", { query: { appId } });
  return dto as ManagedAppDetail;
}

export async function appManagerGetDetailHeavy(
  appId: string,
  mode: AppManagerResidueScanMode = "deep",
): Promise<AppManagerResidueScanResult> {
  const dto = await invokeWithLog<AppManagerResidueScanResultDto>("app_manager_get_detail_heavy", {
    input: { appId, mode },
  });
  return dto as AppManagerResidueScanResult;
}

export async function appManagerScanResidue(appId: string): Promise<AppManagerResidueScanResult> {
  const dto = await invokeWithLog<AppManagerResidueScanResultDto>("app_manager_scan_residue", {
    input: { appId, mode: "deep" },
  });
  return dto as AppManagerResidueScanResult;
}

export async function appManagerCleanup(input: AppManagerCleanupInput): Promise<AppManagerCleanupResult> {
  const dto = await invokeWithLog<AppManagerCleanupResultDto>("app_manager_cleanup", {
    input: input as AppManagerCleanupInputDto,
  });
  return dto as AppManagerCleanupResult;
}

export async function appManagerExportScanResult(appId: string): Promise<AppManagerExportScanResult> {
  const dto = await invokeWithLog<AppManagerExportScanResultDto>("app_manager_export_scan_result", {
    input: { appId },
  });
  return dto as AppManagerExportScanResult;
}

export async function appManagerOpenDirectory(path: string): Promise<void> {
  await invokeWithLog("transfer_open_download_dir", { path });
}

export async function appManagerSetStartup(input: AppManagerStartupUpdateInput): Promise<AppManagerActionResult> {
  const dto = await invokeWithLog<AppManagerActionResultDto>("app_manager_set_startup", {
    input: input as AppManagerStartupUpdateInputDto,
  });
  return dto as AppManagerActionResult;
}

export async function appManagerUninstall(input: AppManagerUninstallInput): Promise<AppManagerActionResult> {
  const dto = await invokeWithLog<AppManagerActionResultDto>("app_manager_uninstall", {
    input: input as AppManagerUninstallInputDto,
  });
  return dto as AppManagerActionResult;
}

export async function appManagerOpenUninstallHelp(appId: string): Promise<AppManagerActionResult> {
  const dto = await invokeWithLog<AppManagerActionResultDto>("app_manager_open_uninstall_help", { appId });
  return dto as AppManagerActionResult;
}

export async function appManagerRevealPath(path: string): Promise<void> {
  await invokeWithLog("app_manager_reveal_path", { path });
}

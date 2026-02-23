import type {
  AppManagerActionResultDto,
  AppManagerCleanupInputDto,
  AppManagerCleanupResultDto,
  AppManagerExportScanResultDto,
  AppManagerPageDto,
  AppManagerQueryDto,
  AppManagerResidueScanResultDto,
  AppManagerStartupUpdateInputDto,
  AppManagerUninstallInputDto,
  ManagedAppDetailDto,
} from "@/contracts";
import type {
  AppManagerActionResult,
  AppManagerCleanupInput,
  AppManagerCleanupResult,
  AppManagerExportScanResult,
  AppManagerResidueScanResult,
  ManagedAppDetail,
  AppManagerPage,
  AppManagerQuery,
  AppManagerStartupUpdateInput,
  AppManagerUninstallInput,
} from "@/components/app-manager/types";
import { normalizeDto } from "@/services/contracts-adapter";
import { invokeWithLog } from "@/services/invoke";

export async function appManagerList(query?: AppManagerQuery): Promise<AppManagerPage> {
  const dto = await invokeWithLog<AppManagerPageDto>("app_manager_list", {
    query: query as AppManagerQueryDto | undefined,
  });
  return normalizeDto<AppManagerPage>(dto);
}

export async function appManagerRefreshIndex(): Promise<AppManagerActionResult> {
  const dto = await invokeWithLog<AppManagerActionResultDto>("app_manager_refresh_index");
  return normalizeDto<AppManagerActionResult>(dto);
}

export async function appManagerGetDetail(appId: string): Promise<ManagedAppDetail> {
  const dto = await invokeWithLog<ManagedAppDetailDto>("app_manager_get_detail", { query: { appId } });
  return normalizeDto<ManagedAppDetail>(dto);
}

export async function appManagerScanResidue(appId: string): Promise<AppManagerResidueScanResult> {
  const dto = await invokeWithLog<AppManagerResidueScanResultDto>("app_manager_scan_residue", { input: { appId } });
  return normalizeDto<AppManagerResidueScanResult>(dto);
}

export async function appManagerCleanup(input: AppManagerCleanupInput): Promise<AppManagerCleanupResult> {
  const dto = await invokeWithLog<AppManagerCleanupResultDto>("app_manager_cleanup", {
    input: input as AppManagerCleanupInputDto,
  });
  return normalizeDto<AppManagerCleanupResult>(dto);
}

export async function appManagerExportScanResult(appId: string): Promise<AppManagerExportScanResult> {
  const dto = await invokeWithLog<AppManagerExportScanResultDto>("app_manager_export_scan_result", { input: { appId } });
  return normalizeDto<AppManagerExportScanResult>(dto);
}

export async function appManagerOpenDirectory(path: string): Promise<void> {
  await invokeWithLog("transfer_open_download_dir", { path });
}

export async function appManagerSetStartup(input: AppManagerStartupUpdateInput): Promise<AppManagerActionResult> {
  const dto = await invokeWithLog<AppManagerActionResultDto>("app_manager_set_startup", {
    input: input as AppManagerStartupUpdateInputDto,
  });
  return normalizeDto<AppManagerActionResult>(dto);
}

export async function appManagerUninstall(input: AppManagerUninstallInput): Promise<AppManagerActionResult> {
  const dto = await invokeWithLog<AppManagerActionResultDto>("app_manager_uninstall", {
    input: input as AppManagerUninstallInputDto,
  });
  return normalizeDto<AppManagerActionResult>(dto);
}

export async function appManagerOpenUninstallHelp(appId: string): Promise<AppManagerActionResult> {
  const dto = await invokeWithLog<AppManagerActionResultDto>("app_manager_open_uninstall_help", { appId });
  return normalizeDto<AppManagerActionResult>(dto);
}

export async function appManagerRevealPath(path: string): Promise<void> {
  await invokeWithLog("app_manager_reveal_path", { path });
}

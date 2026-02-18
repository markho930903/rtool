import { invokeWithLog } from "@/services/invoke";
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

export async function appManagerList(query?: AppManagerQuery): Promise<AppManagerPage> {
  return invokeWithLog<AppManagerPage>("app_manager_list", { query });
}

export async function appManagerRefreshIndex(): Promise<AppManagerActionResult> {
  return invokeWithLog<AppManagerActionResult>("app_manager_refresh_index");
}

export async function appManagerGetDetail(appId: string): Promise<ManagedAppDetail> {
  return invokeWithLog<ManagedAppDetail>("app_manager_get_detail", { query: { appId } });
}

export async function appManagerScanResidue(appId: string): Promise<AppManagerResidueScanResult> {
  return invokeWithLog<AppManagerResidueScanResult>("app_manager_scan_residue", { input: { appId } });
}

export async function appManagerCleanup(input: AppManagerCleanupInput): Promise<AppManagerCleanupResult> {
  return invokeWithLog<AppManagerCleanupResult>("app_manager_cleanup", { input });
}

export async function appManagerExportScanResult(appId: string): Promise<AppManagerExportScanResult> {
  return invokeWithLog<AppManagerExportScanResult>("app_manager_export_scan_result", { input: { appId } });
}

export async function appManagerOpenDirectory(path: string): Promise<void> {
  await invokeWithLog("transfer_open_download_dir", { path });
}

export async function appManagerSetStartup(input: AppManagerStartupUpdateInput): Promise<AppManagerActionResult> {
  return invokeWithLog<AppManagerActionResult>("app_manager_set_startup", { input });
}

export async function appManagerUninstall(input: AppManagerUninstallInput): Promise<AppManagerActionResult> {
  return invokeWithLog<AppManagerActionResult>("app_manager_uninstall", { input });
}

export async function appManagerOpenUninstallHelp(appId: string): Promise<AppManagerActionResult> {
  return invokeWithLog<AppManagerActionResult>("app_manager_open_uninstall_help", { appId });
}

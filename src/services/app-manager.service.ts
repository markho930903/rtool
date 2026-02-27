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
  CommandRequestDto,
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
import { transferOpenDownloadDir } from "@/services/transfer.service";

function invokeAppManager<T>(kind: string, payload?: Record<string, unknown>): Promise<T> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload;
  }
  return invokeWithLog<T>("app_manager_handle", { request });
}

export async function appManagerList(query?: AppManagerQuery): Promise<AppManagerPage> {
  const dto = await invokeAppManager<AppManagerPageDto>("list", {
    query: query as AppManagerQueryDto | undefined,
  });
  return dto as AppManagerPage;
}

export async function appManagerListSnapshotMeta(): Promise<AppManagerSnapshotMeta> {
  const dto = await invokeAppManager<AppManagerSnapshotMetaDto>("list_snapshot_meta");
  return dto as AppManagerSnapshotMeta;
}

export async function appManagerResolveSizes(
  input: AppManagerResolveSizesInput,
): Promise<AppManagerResolveSizesResult> {
  const dto = await invokeAppManager<AppManagerResolveSizesResultDto>("resolve_sizes", {
    input: input as AppManagerResolveSizesInputDto,
  });
  return dto as AppManagerResolveSizesResult;
}

export async function appManagerRefreshIndex(): Promise<AppManagerActionResult> {
  const dto = await invokeAppManager<AppManagerActionResultDto>("refresh_index");
  return dto as AppManagerActionResult;
}

export async function appManagerGetDetail(appId: string): Promise<ManagedAppDetail> {
  const dto = await invokeAppManager<ManagedAppDetailDto>("get_detail", { query: { appId } });
  return dto as ManagedAppDetail;
}

export async function appManagerGetDetailCore(appId: string): Promise<ManagedAppDetail> {
  const dto = await invokeAppManager<ManagedAppDetailDto>("get_detail_core", { query: { appId } });
  return dto as ManagedAppDetail;
}

export async function appManagerGetDetailHeavy(
  appId: string,
  mode: AppManagerResidueScanMode = "deep",
): Promise<AppManagerResidueScanResult> {
  const dto = await invokeAppManager<AppManagerResidueScanResultDto>("get_detail_heavy", {
    input: { appId, mode },
  });
  return dto as AppManagerResidueScanResult;
}

export async function appManagerScanResidue(appId: string): Promise<AppManagerResidueScanResult> {
  const dto = await invokeAppManager<AppManagerResidueScanResultDto>("scan_residue", {
    input: { appId, mode: "deep" },
  });
  return dto as AppManagerResidueScanResult;
}

export async function appManagerCleanup(input: AppManagerCleanupInput): Promise<AppManagerCleanupResult> {
  const dto = await invokeAppManager<AppManagerCleanupResultDto>("cleanup", {
    input: input as AppManagerCleanupInputDto,
  });
  return dto as AppManagerCleanupResult;
}

export async function appManagerExportScanResult(appId: string): Promise<AppManagerExportScanResult> {
  const dto = await invokeAppManager<AppManagerExportScanResultDto>("export_scan_result", {
    input: { appId },
  });
  return dto as AppManagerExportScanResult;
}

export async function appManagerOpenDirectory(path: string): Promise<void> {
  await transferOpenDownloadDir(path);
}

export async function appManagerSetStartup(input: AppManagerStartupUpdateInput): Promise<AppManagerActionResult> {
  const dto = await invokeAppManager<AppManagerActionResultDto>("set_startup", {
    input: input as AppManagerStartupUpdateInputDto,
  });
  return dto as AppManagerActionResult;
}

export async function appManagerUninstall(input: AppManagerUninstallInput): Promise<AppManagerActionResult> {
  const dto = await invokeAppManager<AppManagerActionResultDto>("uninstall", {
    input: input as AppManagerUninstallInputDto,
  });
  return dto as AppManagerActionResult;
}

export async function appManagerOpenUninstallHelp(appId: string): Promise<AppManagerActionResult> {
  const dto = await invokeAppManager<AppManagerActionResultDto>("open_uninstall_help", { appId });
  return dto as AppManagerActionResult;
}

export async function appManagerRevealPath(path: string): Promise<void> {
  await invokeAppManager<void>("reveal_path", { path });
}

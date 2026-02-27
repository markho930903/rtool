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
  AppManagerDetailQueryDto,
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

export const APP_MANAGER_COMMAND_KIND = {
  list: "list",
  listSnapshotMeta: "list_snapshot_meta",
  resolveSizes: "resolve_sizes",
  refreshIndex: "refresh_index",
  getDetail: "get_detail",
  getDetailCore: "get_detail_core",
  getDetailHeavy: "get_detail_heavy",
  scanResidue: "scan_residue",
  cleanup: "cleanup",
  exportScanResult: "export_scan_result",
  setStartup: "set_startup",
  uninstall: "uninstall",
  openUninstallHelp: "open_uninstall_help",
  revealPath: "reveal_path",
} as const;

type AppManagerCommandKind = (typeof APP_MANAGER_COMMAND_KIND)[keyof typeof APP_MANAGER_COMMAND_KIND];

type AppManagerCommandPayloadMap = {
  [APP_MANAGER_COMMAND_KIND.list]: { query: AppManagerQueryDto | undefined };
  [APP_MANAGER_COMMAND_KIND.listSnapshotMeta]: undefined;
  [APP_MANAGER_COMMAND_KIND.resolveSizes]: { input: AppManagerResolveSizesInputDto };
  [APP_MANAGER_COMMAND_KIND.refreshIndex]: undefined;
  [APP_MANAGER_COMMAND_KIND.getDetail]: { query: AppManagerDetailQueryDto };
  [APP_MANAGER_COMMAND_KIND.getDetailCore]: { query: AppManagerDetailQueryDto };
  [APP_MANAGER_COMMAND_KIND.getDetailHeavy]: { input: { appId: string; mode: AppManagerResidueScanMode } };
  [APP_MANAGER_COMMAND_KIND.scanResidue]: { input: { appId: string; mode: AppManagerResidueScanMode } };
  [APP_MANAGER_COMMAND_KIND.cleanup]: { input: AppManagerCleanupInputDto };
  [APP_MANAGER_COMMAND_KIND.exportScanResult]: { input: { appId: string } };
  [APP_MANAGER_COMMAND_KIND.setStartup]: { input: AppManagerStartupUpdateInputDto };
  [APP_MANAGER_COMMAND_KIND.uninstall]: { input: AppManagerUninstallInputDto };
  [APP_MANAGER_COMMAND_KIND.openUninstallHelp]: { appId: string };
  [APP_MANAGER_COMMAND_KIND.revealPath]: { path: string };
};

type AppManagerCommandResultMap = {
  [APP_MANAGER_COMMAND_KIND.list]: AppManagerPageDto;
  [APP_MANAGER_COMMAND_KIND.listSnapshotMeta]: AppManagerSnapshotMetaDto;
  [APP_MANAGER_COMMAND_KIND.resolveSizes]: AppManagerResolveSizesResultDto;
  [APP_MANAGER_COMMAND_KIND.refreshIndex]: AppManagerActionResultDto;
  [APP_MANAGER_COMMAND_KIND.getDetail]: ManagedAppDetailDto;
  [APP_MANAGER_COMMAND_KIND.getDetailCore]: ManagedAppDetailDto;
  [APP_MANAGER_COMMAND_KIND.getDetailHeavy]: AppManagerResidueScanResultDto;
  [APP_MANAGER_COMMAND_KIND.scanResidue]: AppManagerResidueScanResultDto;
  [APP_MANAGER_COMMAND_KIND.cleanup]: AppManagerCleanupResultDto;
  [APP_MANAGER_COMMAND_KIND.exportScanResult]: AppManagerExportScanResultDto;
  [APP_MANAGER_COMMAND_KIND.setStartup]: AppManagerActionResultDto;
  [APP_MANAGER_COMMAND_KIND.uninstall]: AppManagerActionResultDto;
  [APP_MANAGER_COMMAND_KIND.openUninstallHelp]: AppManagerActionResultDto;
  [APP_MANAGER_COMMAND_KIND.revealPath]: void;
};

function invokeAppManager<K extends AppManagerCommandKind>(
  kind: K,
  payload?: AppManagerCommandPayloadMap[K],
): Promise<AppManagerCommandResultMap[K]> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload as Record<string, unknown>;
  }
  return invokeWithLog<AppManagerCommandResultMap[K]>("app_manager_handle", { request });
}

export async function appManagerList(query?: AppManagerQuery): Promise<AppManagerPage> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.list, {
    query: query as AppManagerQueryDto | undefined,
  });
  return dto as AppManagerPage;
}

export async function appManagerListSnapshotMeta(): Promise<AppManagerSnapshotMeta> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.listSnapshotMeta);
  return dto as AppManagerSnapshotMeta;
}

export async function appManagerResolveSizes(
  input: AppManagerResolveSizesInput,
): Promise<AppManagerResolveSizesResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.resolveSizes, {
    input: input as AppManagerResolveSizesInputDto,
  });
  return dto as AppManagerResolveSizesResult;
}

export async function appManagerRefreshIndex(): Promise<AppManagerActionResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.refreshIndex);
  return dto as AppManagerActionResult;
}

export async function appManagerGetDetail(appId: string): Promise<ManagedAppDetail> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.getDetail, { query: { appId } });
  return dto as ManagedAppDetail;
}

export async function appManagerGetDetailCore(appId: string): Promise<ManagedAppDetail> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.getDetailCore, { query: { appId } });
  return dto as ManagedAppDetail;
}

export async function appManagerGetDetailHeavy(
  appId: string,
  mode: AppManagerResidueScanMode = "deep",
): Promise<AppManagerResidueScanResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.getDetailHeavy, {
    input: { appId, mode },
  });
  return dto as AppManagerResidueScanResult;
}

export async function appManagerScanResidue(appId: string): Promise<AppManagerResidueScanResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.scanResidue, {
    input: { appId, mode: "deep" },
  });
  return dto as AppManagerResidueScanResult;
}

export async function appManagerCleanup(input: AppManagerCleanupInput): Promise<AppManagerCleanupResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.cleanup, {
    input: input as AppManagerCleanupInputDto,
  });
  return dto as AppManagerCleanupResult;
}

export async function appManagerExportScanResult(appId: string): Promise<AppManagerExportScanResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.exportScanResult, {
    input: { appId },
  });
  return dto as AppManagerExportScanResult;
}

export async function appManagerOpenDirectory(path: string): Promise<void> {
  await transferOpenDownloadDir(path);
}

export async function appManagerSetStartup(input: AppManagerStartupUpdateInput): Promise<AppManagerActionResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.setStartup, {
    input: input as AppManagerStartupUpdateInputDto,
  });
  return dto as AppManagerActionResult;
}

export async function appManagerUninstall(input: AppManagerUninstallInput): Promise<AppManagerActionResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.uninstall, {
    input: input as AppManagerUninstallInputDto,
  });
  return dto as AppManagerActionResult;
}

export async function appManagerOpenUninstallHelp(appId: string): Promise<AppManagerActionResult> {
  const dto = await invokeAppManager(APP_MANAGER_COMMAND_KIND.openUninstallHelp, { appId });
  return dto as AppManagerActionResult;
}

export async function appManagerRevealPath(path: string): Promise<void> {
  await invokeAppManager(APP_MANAGER_COMMAND_KIND.revealPath, { path });
}

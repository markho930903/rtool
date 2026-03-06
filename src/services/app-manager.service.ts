import type {
  AppManagerActionResult,
  AppManagerCleanupInput,
  AppManagerCleanupResult,
  AppManagerExportScanResult,
  AppManagerPage,
  AppManagerQuery,
  AppManagerResidueScanMode,
  AppManagerResidueScanResult,
  AppManagerResolveSizesInput,
  AppManagerResolveSizesResult,
  AppManagerSnapshotMeta,
  AppManagerStartupUpdateInput,
  AppManagerUninstallInput,
  ManagedAppDetail,
} from "@/components/app-manager/types";
import type { AppManagerQueryDto, AppManagerRequestDto } from "@/contracts";
import { invokeFeature } from "@/services/invoke";

type AppManagerRequestKind = AppManagerRequestDto["kind"];
type AppManagerRequest<K extends AppManagerRequestKind> = Extract<AppManagerRequestDto, { kind: K }>;

function createAppManagerRequest<K extends AppManagerRequestKind>(request: AppManagerRequest<K>): AppManagerRequest<K> {
  return request;
}

function invokeAppManager<TResult, K extends AppManagerRequestKind>(request: AppManagerRequest<K>): Promise<TResult> {
  return invokeFeature<TResult>("app_manager", request);
}

function toAppManagerQueryDto(query?: AppManagerQuery): AppManagerQueryDto {
  return {
    keyword: query?.keyword?.trim() ? query.keyword.trim() : null,
    category: query?.category ?? "all",
    limit: query?.limit ?? null,
    cursor: query?.cursor ?? null,
  };
}

export function appManagerList(query?: AppManagerQuery): Promise<AppManagerPage> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "list",
      payload: {
        query: toAppManagerQueryDto(query),
      },
    }),
  );
}

export function appManagerListSnapshotMeta(): Promise<AppManagerSnapshotMeta> {
  return invokeAppManager(createAppManagerRequest({ kind: "list_snapshot_meta" }));
}

export function appManagerResolveSizes(input: AppManagerResolveSizesInput): Promise<AppManagerResolveSizesResult> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "resolve_sizes",
      payload: { input },
    }),
  );
}

export function appManagerRefreshIndex(): Promise<AppManagerActionResult> {
  return invokeAppManager(createAppManagerRequest({ kind: "refresh_index" }));
}

export function appManagerGetDetailCore(appId: string): Promise<ManagedAppDetail> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "get_detail_core",
      payload: {
        query: { appId },
      },
    }),
  );
}

export function appManagerGetDetailHeavy(
  appId: string,
  mode: AppManagerResidueScanMode = "deep",
): Promise<AppManagerResidueScanResult> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "get_detail_heavy",
      payload: {
        input: { appId, mode },
      },
    }),
  );
}

export function appManagerCleanup(input: AppManagerCleanupInput): Promise<AppManagerCleanupResult> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "cleanup",
      payload: {
        input: {
          ...input,
          skipOnError: input.skipOnError ?? null,
          confirmedFingerprint: input.confirmedFingerprint ?? null,
        },
      },
    }),
  );
}

export function appManagerExportScanResult(appId: string): Promise<AppManagerExportScanResult> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "export_scan_result",
      payload: {
        input: { appId },
      },
    }),
  );
}

export function appManagerOpenDirectory(path: string): Promise<void> {
  return appManagerRevealPath(path);
}

export function appManagerSetStartup(input: AppManagerStartupUpdateInput): Promise<AppManagerActionResult> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "set_startup",
      payload: { input },
    }),
  );
}

export function appManagerUninstall(input: AppManagerUninstallInput): Promise<AppManagerActionResult> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "uninstall",
      payload: { input },
    }),
  );
}

export function appManagerOpenUninstallHelp(appId: string): Promise<AppManagerActionResult> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "open_uninstall_help",
      payload: { appId },
    }),
  );
}

export function appManagerOpenPermissionHelp(appId: string): Promise<AppManagerActionResult> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "open_permission_help",
      payload: { appId },
    }),
  );
}

export function appManagerRevealPath(path: string): Promise<void> {
  return invokeAppManager(
    createAppManagerRequest({
      kind: "reveal_path",
      payload: { path },
    }),
  );
}

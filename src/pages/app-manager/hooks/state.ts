import type {
  AppManagerActionResult,
  AppManagerCleanupDeleteMode,
  AppManagerCleanupResult,
  AppManagerExportScanResult,
  AppManagerResidueScanResult,
  ManagedApp,
  ManagedAppDetail,
} from "@/components/app-manager/types";

export type AppSizeState = "pending" | "resolving" | "exact" | "estimated";

export interface AppManagerPerAppUiState {
  coreDetail: ManagedAppDetail | null;
  heavyDetail: AppManagerResidueScanResult | null;
  coreLoading: boolean;
  heavyLoading: boolean;
  detailError: string | null;
  selectedResidueIds: string[];
  selectionTouchedByUser: boolean;
  includeMain: boolean;
  deleteMode: AppManagerCleanupDeleteMode;
  cleanupLoading: boolean;
  cleanupResult: AppManagerCleanupResult | null;
  cleanupError: string | null;
  startupLoading: boolean;
  uninstallLoading: boolean;
  openHelpLoading: boolean;
  openPermissionHelpLoading: boolean;
  exportLoading: boolean;
  openExportDirLoading: boolean;
  exportResult: AppManagerExportScanResult | null;
  exportError: string | null;
  actionResult: AppManagerActionResult | null;
  actionError: string | null;
}

export function createAppManagerPerAppUiState(): AppManagerPerAppUiState {
  return {
    coreDetail: null,
    heavyDetail: null,
    coreLoading: false,
    heavyLoading: false,
    detailError: null,
    selectedResidueIds: [],
    selectionTouchedByUser: false,
    includeMain: true,
    deleteMode: "trash",
    cleanupLoading: false,
    cleanupResult: null,
    cleanupError: null,
    startupLoading: false,
    uninstallLoading: false,
    openHelpLoading: false,
    openPermissionHelpLoading: false,
    exportLoading: false,
    openExportDirLoading: false,
    exportResult: null,
    exportError: null,
    actionResult: null,
    actionError: null,
  };
}

export function getPerAppUiState(
  stateByAppId: Record<string, AppManagerPerAppUiState>,
  appId: string | null | undefined,
): AppManagerPerAppUiState {
  if (!appId) {
    return createAppManagerPerAppUiState();
  }
  return stateByAppId[appId] ?? createAppManagerPerAppUiState();
}

export function updatePerAppUiState(
  stateByAppId: Record<string, AppManagerPerAppUiState>,
  appId: string,
  updater: (state: AppManagerPerAppUiState) => AppManagerPerAppUiState,
): Record<string, AppManagerPerAppUiState> {
  const current = stateByAppId[appId] ?? createAppManagerPerAppUiState();
  const next = updater(current);
  if (next === current && stateByAppId[appId]) {
    return stateByAppId;
  }
  return {
    ...stateByAppId,
    [appId]: next,
  };
}

export function retainById<T>(record: Record<string, T>, keep: Set<string>): Record<string, T> {
  if (Object.keys(record).length === 0) {
    return record;
  }

  let removed = false;
  const next: Record<string, T> = {};
  for (const [key, value] of Object.entries(record)) {
    if (keep.has(key)) {
      next[key] = value;
    } else {
      removed = true;
    }
  }
  return removed ? next : record;
}

export function uniqueById(items: ManagedApp[]): ManagedApp[] {
  const map = new Map<string, ManagedApp>();
  for (const item of items) {
    map.set(item.id, item);
  }
  return [...map.values()];
}

export function initialSizeState(item: ManagedApp): AppSizeState {
  if (item.sizeAccuracy === "exact" && item.sizeBytes !== null) {
    return "exact";
  }
  if (item.sizeBytes !== null) {
    return "estimated";
  }
  return "pending";
}

export function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

export function formatIndexedAt(timestamp: number | null): string {
  if (!timestamp || !Number.isFinite(timestamp) || timestamp <= 0) {
    return "-";
  }
  return new Date(timestamp * 1000).toLocaleString();
}

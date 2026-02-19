import { create } from "zustand";

import type {
  AppManagerActionResult,
  AppManagerCleanupInput,
  AppManagerCleanupResult,
  AppManagerExportScanResult,
  AppManagerResidueScanResult,
  ManagedApp,
  ManagedAppDetail,
} from "@/components/app-manager/types";
import {
  appManagerCleanup,
  appManagerExportScanResult,
  appManagerGetDetail,
  appManagerList,
  appManagerOpenDirectory,
  appManagerOpenUninstallHelp,
  appManagerRefreshIndex,
  appManagerScanResidue,
  appManagerSetStartup,
  appManagerUninstall,
} from "@/services/app-manager.service";

const EXPERIMENTAL_THIRD_PARTY_STARTUP_KEY = "rtool.appManager.experimentalThirdPartyStartup";

function readExperimentalThirdPartyStartup(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  try {
    return window.localStorage.getItem(EXPERIMENTAL_THIRD_PARTY_STARTUP_KEY) === "1";
  } catch {
    return false;
  }
}

function persistExperimentalThirdPartyStartup(enabled: boolean): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(EXPERIMENTAL_THIRD_PARTY_STARTUP_KEY, enabled ? "1" : "0");
  } catch {
    // ignore storage failures
  }
}

function updateActionLoading(map: Record<string, boolean>, appId: string, loading: boolean): Record<string, boolean> {
  if (loading) {
    return { ...map, [appId]: true };
  }
  const next = { ...map };
  delete next[appId];
  return next;
}

function deduplicateItemIds(values: string[]): string[] {
  const set = new Set(values);
  return [...set];
}

interface AppManagerState {
  items: ManagedApp[];
  loading: boolean;
  loadingMore: boolean;
  refreshing: boolean;
  actionLoadingById: Record<string, boolean>;
  detailLoadingById: Record<string, boolean>;
  scanLoadingById: Record<string, boolean>;
  cleanupLoadingById: Record<string, boolean>;
  exportLoadingById: Record<string, boolean>;
  openExportDirLoadingById: Record<string, boolean>;
  keyword: string;
  category: string;
  nextCursor: string | null;
  indexedAt: number | null;
  error: string | null;
  detailError: string | null;
  scanError: string | null;
  cleanupError: string | null;
  exportError: string | null;
  lastActionResult: AppManagerActionResult | null;
  selectedAppId: string | null;
  detailById: Record<string, ManagedAppDetail>;
  scanResultById: Record<string, AppManagerResidueScanResult>;
  cleanupResultById: Record<string, AppManagerCleanupResult>;
  exportResultById: Record<string, AppManagerExportScanResult>;
  selectedResidueIdsByAppId: Record<string, string[]>;
  deleteModeByAppId: Record<string, "trash" | "permanent">;
  includeMainAppByAppId: Record<string, boolean>;
  experimentalThirdPartyStartup: boolean;
}

interface AppManagerActions {
  setKeyword: (keyword: string) => void;
  setCategory: (category: string) => void;
  setExperimentalThirdPartyStartup: (enabled: boolean) => void;
  selectApp: (appId: string) => Promise<void>;
  loadFirstPage: () => Promise<void>;
  loadMore: () => Promise<void>;
  refreshIndex: () => Promise<void>;
  loadDetail: (appId: string, force?: boolean) => Promise<void>;
  scanResidue: (appId: string) => Promise<void>;
  toggleResidueItem: (appId: string, itemId: string, checked: boolean) => void;
  selectRecommendedResidues: (appId: string) => void;
  clearResidueSelection: (appId: string) => void;
  setDeleteMode: (appId: string, mode: "trash" | "permanent") => void;
  setIncludeMainApp: (appId: string, includeMainApp: boolean) => void;
  exportScanResult: (appId: string) => Promise<AppManagerExportScanResult | null>;
  openExportDirectory: (appId: string) => Promise<void>;
  cleanupSelected: (appId: string) => Promise<void>;
  retryFailedCleanup: (appId: string) => Promise<void>;
  deepUninstall: (appId: string) => Promise<void>;
  toggleStartup: (app: ManagedApp, enabled: boolean) => Promise<void>;
  uninstall: (app: ManagedApp) => Promise<void>;
  openUninstallHelp: (app: ManagedApp) => Promise<void>;
  clearLastActionResult: () => void;
}

type AppManagerStore = AppManagerState & AppManagerActions;

export const useAppManagerStore = create<AppManagerStore>((set, get) => ({
  items: [],
  loading: false,
  loadingMore: false,
  refreshing: false,
  actionLoadingById: {},
  detailLoadingById: {},
  scanLoadingById: {},
  cleanupLoadingById: {},
  exportLoadingById: {},
  openExportDirLoadingById: {},
  keyword: "",
  category: "all",
  nextCursor: null,
  indexedAt: null,
  error: null,
  detailError: null,
  scanError: null,
  cleanupError: null,
  exportError: null,
  lastActionResult: null,
  selectedAppId: null,
  detailById: {},
  scanResultById: {},
  cleanupResultById: {},
  exportResultById: {},
  selectedResidueIdsByAppId: {},
  deleteModeByAppId: {},
  includeMainAppByAppId: {},
  experimentalThirdPartyStartup: readExperimentalThirdPartyStartup(),

  setKeyword(keyword) {
    set({ keyword });
  },

  setCategory(category) {
    set({ category });
  },

  setExperimentalThirdPartyStartup(enabled) {
    persistExperimentalThirdPartyStartup(enabled);
    set({ experimentalThirdPartyStartup: enabled });
  },

  async selectApp(appId) {
    const current = get().selectedAppId;
    if (current === appId) {
      return;
    }
    set({ selectedAppId: appId, detailError: null, scanError: null, cleanupError: null, exportError: null });
    await get().loadDetail(appId);
  },

  async loadFirstPage() {
    const { keyword, category, selectedAppId } = get();
    set({ loading: true, error: null });
    try {
      const page = await appManagerList({
        keyword: keyword.trim() || undefined,
        category: category === "all" ? undefined : category,
      });
      const fallbackSelected = page.items[0]?.id ?? null;
      const selectedStillExists = selectedAppId ? page.items.some((item) => item.id === selectedAppId) : false;
      const nextSelectedAppId = selectedStillExists ? selectedAppId : fallbackSelected;
      set({
        items: page.items,
        nextCursor: page.nextCursor ?? null,
        indexedAt: page.indexedAt,
        loading: false,
        selectedAppId: nextSelectedAppId,
      });
      if (nextSelectedAppId) {
        await get().loadDetail(nextSelectedAppId);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ loading: false, error: message });
    }
  },

  async loadMore() {
    const { loadingMore, nextCursor, keyword, category } = get();
    if (loadingMore || !nextCursor) {
      return;
    }
    set({ loadingMore: true, error: null });
    try {
      const page = await appManagerList({
        keyword: keyword.trim() || undefined,
        category: category === "all" ? undefined : category,
        cursor: nextCursor,
      });
      set((state) => ({
        items: [...state.items, ...page.items],
        nextCursor: page.nextCursor ?? null,
        indexedAt: page.indexedAt,
        loadingMore: false,
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ loadingMore: false, error: message });
    }
  },

  async refreshIndex() {
    set({ refreshing: true, error: null });
    try {
      const result = await appManagerRefreshIndex();
      set({ lastActionResult: result, refreshing: false });
      await get().loadFirstPage();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ refreshing: false, error: message });
    }
  },

  async loadDetail(appId, force = false) {
    const detail = get().detailById[appId];
    if (!force && detail) {
      return;
    }
    set((state) => ({
      detailLoadingById: updateActionLoading(state.detailLoadingById, appId, true),
      detailError: null,
    }));
    try {
      const nextDetail = await appManagerGetDetail(appId);
      set((state) => ({
        detailById: { ...state.detailById, [appId]: nextDetail },
        items: state.items.map((item) => {
          if (item.id !== appId) {
            return item;
          }
          return {
            ...item,
            estimatedSizeBytes:
              nextDetail.sizeSummary.appBytes ?? item.estimatedSizeBytes,
          };
        }),
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ detailError: message });
    } finally {
      set((state) => ({
        detailLoadingById: updateActionLoading(state.detailLoadingById, appId, false),
      }));
    }
  },

  async scanResidue(appId) {
    set((state) => ({
      scanLoadingById: updateActionLoading(state.scanLoadingById, appId, true),
      scanError: null,
    }));
    try {
      const result = await appManagerScanResidue(appId);
      const recommendedIds = result.groups
        .flatMap((group) => group.items)
        .filter((item) => item.recommended)
        .map((item) => item.itemId);
      set((state) => ({
        scanResultById: { ...state.scanResultById, [appId]: result },
        selectedResidueIdsByAppId: {
          ...state.selectedResidueIdsByAppId,
          [appId]: deduplicateItemIds(recommendedIds),
        },
        deleteModeByAppId: {
          ...state.deleteModeByAppId,
          [appId]: state.deleteModeByAppId[appId] ?? "trash",
        },
        includeMainAppByAppId: {
          ...state.includeMainAppByAppId,
          [appId]: state.includeMainAppByAppId[appId] ?? true,
        },
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ scanError: message });
    } finally {
      set((state) => ({
        scanLoadingById: updateActionLoading(state.scanLoadingById, appId, false),
      }));
    }
  },

  toggleResidueItem(appId, itemId, checked) {
    set((state) => {
      const current = state.selectedResidueIdsByAppId[appId] ?? [];
      const setValue = new Set(current);
      if (checked) {
        setValue.add(itemId);
      } else {
        setValue.delete(itemId);
      }
      return {
        selectedResidueIdsByAppId: {
          ...state.selectedResidueIdsByAppId,
          [appId]: [...setValue],
        },
      };
    });
  },

  selectRecommendedResidues(appId) {
    const result = get().scanResultById[appId];
    if (!result) {
      return;
    }
    const recommendedIds = result.groups
      .flatMap((group) => group.items)
      .filter((item) => item.recommended)
      .map((item) => item.itemId);
    set((state) => ({
      selectedResidueIdsByAppId: {
        ...state.selectedResidueIdsByAppId,
        [appId]: deduplicateItemIds(recommendedIds),
      },
    }));
  },

  clearResidueSelection(appId) {
    set((state) => ({
      selectedResidueIdsByAppId: {
        ...state.selectedResidueIdsByAppId,
        [appId]: [],
      },
    }));
  },

  setDeleteMode(appId, mode) {
    set((state) => ({
      deleteModeByAppId: {
        ...state.deleteModeByAppId,
        [appId]: mode,
      },
    }));
  },

  setIncludeMainApp(appId, includeMainApp) {
    set((state) => ({
      includeMainAppByAppId: {
        ...state.includeMainAppByAppId,
        [appId]: includeMainApp,
      },
    }));
  },

  async exportScanResult(appId) {
    set((state) => ({
      exportLoadingById: updateActionLoading(state.exportLoadingById, appId, true),
      exportError: null,
    }));
    try {
      const result = await appManagerExportScanResult(appId);
      set((state) => ({
        exportResultById: { ...state.exportResultById, [appId]: result },
      }));
      const openResult = await (async () => {
        try {
          await appManagerOpenDirectory(result.directoryPath);
          return { ok: true as const };
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          return { ok: false as const, message };
        }
      })();
      if (!openResult.ok) {
        set({
          exportError: `导出成功，但打开目录失败：${openResult.message}`,
        });
      }
      return result;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ exportError: message });
      return null;
    } finally {
      set((state) => ({
        exportLoadingById: updateActionLoading(state.exportLoadingById, appId, false),
      }));
    }
  },

  async openExportDirectory(appId) {
    const exportResult = get().exportResultById[appId];
    if (!exportResult) {
      set({ exportError: "暂无可打开的导出目录，请先导出扫描结果" });
      return;
    }
    set((state) => ({
      openExportDirLoadingById: updateActionLoading(state.openExportDirLoadingById, appId, true),
      exportError: null,
    }));
    try {
      await appManagerOpenDirectory(exportResult.directoryPath);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ exportError: message });
    } finally {
      set((state) => ({
        openExportDirLoadingById: updateActionLoading(state.openExportDirLoadingById, appId, false),
      }));
    }
  },

  async cleanupSelected(appId) {
    const state = get();
    const app = state.items.find((item) => item.id === appId);
    if (!app) {
      set({ cleanupError: "应用不存在或索引已变化" });
      return;
    }
    const selectedItemIds = state.selectedResidueIdsByAppId[appId] ?? [];
    const includeMainApp = state.includeMainAppByAppId[appId] ?? true;
    const deleteMode = state.deleteModeByAppId[appId] ?? "trash";

    if (!includeMainApp && selectedItemIds.length === 0) {
      set({ cleanupError: "请至少选择一个清理项或启用主程序卸载" });
      return;
    }

    const input: AppManagerCleanupInput = {
      appId,
      selectedItemIds,
      deleteMode,
      includeMainApp,
      skipOnError: true,
      confirmedFingerprint: includeMainApp ? app.fingerprint : undefined,
    };

    set((draft) => ({
      cleanupLoadingById: updateActionLoading(draft.cleanupLoadingById, appId, true),
      cleanupError: null,
    }));
    try {
      const result = await appManagerCleanup(input);
      set((draft) => ({
        cleanupResultById: { ...draft.cleanupResultById, [appId]: result },
        selectedResidueIdsByAppId: {
          ...draft.selectedResidueIdsByAppId,
          [appId]: [],
        },
      }));
      await get().refreshIndex();
      await get().loadDetail(appId, true);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ cleanupError: message });
    } finally {
      set((draft) => ({
        cleanupLoadingById: updateActionLoading(draft.cleanupLoadingById, appId, false),
      }));
    }
  },

  async retryFailedCleanup(appId) {
    const cleanupResult = get().cleanupResultById[appId];
    if (!cleanupResult) {
      set({ cleanupError: "暂无可重试的失败项" });
      return;
    }
    const retryableIds = cleanupResult.failed
      .map((row) => row.itemId)
      .filter((itemId) => itemId !== "main-app");
    if (retryableIds.length === 0) {
      set({ cleanupError: "暂无可重试的残留项" });
      return;
    }
    set((state) => ({
      selectedResidueIdsByAppId: {
        ...state.selectedResidueIdsByAppId,
        [appId]: deduplicateItemIds(retryableIds),
      },
      includeMainAppByAppId: {
        ...state.includeMainAppByAppId,
        [appId]: false,
      },
    }));
    await get().cleanupSelected(appId);
  },

  async deepUninstall(appId) {
    const state = get();
    if (!state.scanResultById[appId] && !state.scanLoadingById[appId]) {
      await get().scanResidue(appId);
    }
    get().setIncludeMainApp(appId, true);
    await get().cleanupSelected(appId);
  },

  async toggleStartup(app, enabled) {
    set((state) => ({
      actionLoadingById: updateActionLoading(state.actionLoadingById, app.id, true),
      error: null,
    }));
    try {
      const result = await appManagerSetStartup({ appId: app.id, enabled });
      set({ lastActionResult: result });
      await get().loadFirstPage();
      await get().loadDetail(app.id, true);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ error: message });
    } finally {
      set((state) => ({
        actionLoadingById: updateActionLoading(state.actionLoadingById, app.id, false),
      }));
    }
  },

  async uninstall(app) {
    set((state) => ({
      actionLoadingById: updateActionLoading(state.actionLoadingById, app.id, true),
      error: null,
    }));
    try {
      const result = await appManagerUninstall({
        appId: app.id,
        confirmedFingerprint: app.fingerprint,
      });
      set({ lastActionResult: result });
      await get().loadFirstPage();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ error: message });
    } finally {
      set((state) => ({
        actionLoadingById: updateActionLoading(state.actionLoadingById, app.id, false),
      }));
    }
  },

  async openUninstallHelp(app) {
    set((state) => ({
      actionLoadingById: updateActionLoading(state.actionLoadingById, app.id, true),
      error: null,
    }));
    try {
      const result = await appManagerOpenUninstallHelp(app.id);
      set({ lastActionResult: result });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ error: message });
    } finally {
      set((state) => ({
        actionLoadingById: updateActionLoading(state.actionLoadingById, app.id, false),
      }));
    }
  },

  clearLastActionResult() {
    set({ lastActionResult: null });
  },
}));

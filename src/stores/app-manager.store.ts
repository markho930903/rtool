import { create } from "zustand";

import type {
  AppManagerActionResult,
  AppManagerCleanupDeleteMode,
  AppManagerCleanupInput,
  AppManagerCleanupResult,
  AppManagerExportScanResult,
  AppManagerIndexState,
  AppManagerIndexUpdatedPayload,
  AppManagerQueryCategory,
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
const APP_MANAGER_PAGE_SIZE = 80;
const DETAIL_CACHE_TTL_MS = 60_000;
const DETAIL_CACHE_MAX = 80;
const SNAPSHOT_FETCH_LIMIT = 300;

let listInFlight: Promise<void> | null = null;
const detailInFlight = new Map<string, Promise<void>>();

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

function matchesKeyword(item: ManagedApp, keyword: string): boolean {
  const normalized = keyword.trim().toLowerCase();
  if (!normalized) {
    return true;
  }
  const fields = [
    item.name,
    item.path,
    item.bundleOrAppId,
    item.publisher,
    item.identity.primaryId,
    ...item.identity.aliases,
  ]
    .filter(Boolean)
    .map((value) => String(value).toLowerCase());
  return fields.some((value) => value.includes(normalized));
}

function matchesCategory(item: ManagedApp, category: AppManagerQueryCategory): boolean {
  if (category === "all") {
    return true;
  }
  if (category === "startup") {
    return item.startupEnabled;
  }
  if (category === "rtool") {
    return item.source === "rtool";
  }
  return item.source === "application";
}

function filteredItems(allItems: ManagedApp[], keyword: string, category: AppManagerQueryCategory): ManagedApp[] {
  return allItems.filter((item) => matchesCategory(item, category) && matchesKeyword(item, keyword));
}

function nextCursorFor(total: number, loaded: number): string | null {
  return loaded < total ? String(loaded) : null;
}

function withDetailSize(item: ManagedApp, detail: ManagedAppDetail): ManagedApp {
  const sizeBytes = detail.sizeSummary.appBytes ?? item.sizeBytes ?? null;
  return {
    ...item,
    sizeBytes,
    sizeAccuracy: sizeBytes === null ? item.sizeAccuracy : "exact",
    sizeComputedAt: Math.floor(Date.now() / 1000),
  };
}

interface AppManagerState {
  allItems: ManagedApp[];
  items: ManagedApp[];
  loading: boolean;
  loadingMore: boolean;
  refreshing: boolean;
  actionLoadingById: Record<string, boolean>;
  detailLoadingById: Record<string, boolean>;
  detailInFlightById: Record<string, boolean>;
  scanLoadingById: Record<string, boolean>;
  cleanupLoadingById: Record<string, boolean>;
  exportLoadingById: Record<string, boolean>;
  openExportDirLoadingById: Record<string, boolean>;
  keyword: string;
  category: AppManagerQueryCategory;
  nextCursor: string | null;
  indexedAt: number | null;
  revision: number;
  indexState: AppManagerIndexState;
  snapshotLoaded: boolean;
  indexDirty: boolean;
  error: string | null;
  detailError: string | null;
  scanError: string | null;
  cleanupError: string | null;
  exportError: string | null;
  lastActionResult: AppManagerActionResult | null;
  selectedAppId: string | null;
  detailById: Record<string, ManagedAppDetail>;
  detailLoadedAtById: Record<string, number>;
  scanResultById: Record<string, AppManagerResidueScanResult>;
  cleanupResultById: Record<string, AppManagerCleanupResult>;
  exportResultById: Record<string, AppManagerExportScanResult>;
  selectedResidueIdsByAppId: Record<string, string[]>;
  deleteModeByAppId: Record<string, AppManagerCleanupDeleteMode>;
  includeMainAppByAppId: Record<string, boolean>;
  experimentalThirdPartyStartup: boolean;
}

interface AppManagerActions {
  setKeyword: (keyword: string) => void;
  setCategory: (category: AppManagerQueryCategory) => void;
  setExperimentalThirdPartyStartup: (enabled: boolean) => void;
  handleIndexUpdated: (payload: AppManagerIndexUpdatedPayload, inAppManagerRoute?: boolean) => void;
  selectApp: (appId: string) => Promise<void>;
  ensureSnapshotLoaded: (force?: boolean) => Promise<void>;
  loadFirstPage: (force?: boolean) => Promise<void>;
  loadMore: () => Promise<void>;
  refreshIndex: () => Promise<void>;
  loadDetail: (appId: string, force?: boolean) => Promise<void>;
  scanResidue: (appId: string) => Promise<void>;
  toggleResidueItem: (appId: string, itemId: string, checked: boolean) => void;
  selectRecommendedResidues: (appId: string) => void;
  clearResidueSelection: (appId: string) => void;
  setDeleteMode: (appId: string, mode: AppManagerCleanupDeleteMode) => void;
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

function applyLocalFilterState(
  state: Pick<AppManagerState, "allItems" | "keyword" | "category" | "selectedAppId">,
  loadedCount = APP_MANAGER_PAGE_SIZE,
): Pick<AppManagerState, "items" | "nextCursor" | "selectedAppId"> {
  const filtered = filteredItems(state.allItems, state.keyword, state.category);
  const clampedLoadedCount = Math.max(0, Math.min(loadedCount, filtered.length));
  const items = filtered.slice(0, clampedLoadedCount);
  const selectedStillExists = state.selectedAppId && items.some((item) => item.id === state.selectedAppId);
  const selectedAppId = selectedStillExists ? state.selectedAppId : (items[0]?.id ?? null);
  return {
    items,
    nextCursor: nextCursorFor(filtered.length, clampedLoadedCount),
    selectedAppId,
  };
}

async function fetchAllPagesSnapshot(): Promise<{
  items: ManagedApp[];
  indexedAt: number;
  revision: number;
  indexState: AppManagerIndexState;
}> {
  const allItems: ManagedApp[] = [];
  let cursor: string | undefined;
  let indexedAt = 0;
  let revision = 0;
  let indexState: AppManagerIndexState = "ready";

  while (true) {
    const page = await appManagerList({ limit: SNAPSHOT_FETCH_LIMIT, cursor });
    allItems.push(...page.items);
    indexedAt = page.indexedAt;
    revision = page.revision;
    indexState = page.indexState;
    if (!page.nextCursor) {
      break;
    }
    cursor = page.nextCursor;
  }

  return { items: allItems, indexedAt, revision, indexState };
}

export const useAppManagerStore = create<AppManagerStore>((set, get) => ({
  allItems: [],
  items: [],
  loading: false,
  loadingMore: false,
  refreshing: false,
  actionLoadingById: {},
  detailLoadingById: {},
  detailInFlightById: {},
  scanLoadingById: {},
  cleanupLoadingById: {},
  exportLoadingById: {},
  openExportDirLoadingById: {},
  keyword: "",
  category: "all",
  nextCursor: null,
  indexedAt: null,
  revision: 0,
  indexState: "ready",
  snapshotLoaded: false,
  indexDirty: false,
  error: null,
  detailError: null,
  scanError: null,
  cleanupError: null,
  exportError: null,
  lastActionResult: null,
  selectedAppId: null,
  detailById: {},
  detailLoadedAtById: {},
  scanResultById: {},
  cleanupResultById: {},
  exportResultById: {},
  selectedResidueIdsByAppId: {},
  deleteModeByAppId: {},
  includeMainAppByAppId: {},
  experimentalThirdPartyStartup: readExperimentalThirdPartyStartup(),

  setKeyword(keyword) {
    set((state) => ({
      keyword,
      ...applyLocalFilterState({
        allItems: state.allItems,
        keyword,
        category: state.category,
        selectedAppId: state.selectedAppId,
      }),
    }));
  },

  setCategory(category) {
    set((state) => ({
      category,
      ...applyLocalFilterState({
        allItems: state.allItems,
        keyword: state.keyword,
        category,
        selectedAppId: state.selectedAppId,
      }),
    }));
  },

  setExperimentalThirdPartyStartup(enabled) {
    persistExperimentalThirdPartyStartup(enabled);
    set({ experimentalThirdPartyStartup: enabled });
  },

  handleIndexUpdated(payload, inAppManagerRoute = true) {
    set((state) => ({
      revision: Math.max(state.revision, payload.revision),
      indexedAt: payload.indexedAt,
      indexDirty: true,
    }));
    if (get().snapshotLoaded && inAppManagerRoute) {
      void get().ensureSnapshotLoaded(true);
    }
  },

  async selectApp(appId) {
    const current = get().selectedAppId;
    if (current === appId) {
      return;
    }
    set({ selectedAppId: appId, detailError: null, scanError: null, cleanupError: null, exportError: null });
    await get().loadDetail(appId);
  },

  async ensureSnapshotLoaded(force = false) {
    const state = get();
    if (!force && state.snapshotLoaded && !state.indexDirty) {
      return;
    }
    if (listInFlight) {
      return listInFlight;
    }

    const run = async () => {
      set({ loading: true, error: null });
      try {
        const snapshot = await fetchAllPagesSnapshot();
        set((draft) => {
          const next = applyLocalFilterState(
            {
              allItems: snapshot.items,
              keyword: draft.keyword,
              category: draft.category,
              selectedAppId: draft.selectedAppId,
            },
            APP_MANAGER_PAGE_SIZE,
          );
          return {
            allItems: snapshot.items,
            items: next.items,
            nextCursor: next.nextCursor,
            selectedAppId: next.selectedAppId,
            indexedAt: snapshot.indexedAt,
            revision: snapshot.revision,
            indexState: snapshot.indexState,
            loading: false,
            snapshotLoaded: true,
            indexDirty: false,
          };
        });
        const selected = get().selectedAppId;
        if (selected) {
          await get().loadDetail(selected);
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        set({ loading: false, error: message });
      } finally {
        listInFlight = null;
      }
    };

    listInFlight = run();
    return listInFlight;
  },

  async loadFirstPage(force = false) {
    await get().ensureSnapshotLoaded(force);
  },

  async loadMore() {
    const state = get();
    if (state.loadingMore || !state.nextCursor) {
      return;
    }
    set({ loadingMore: true });
    set((draft) => {
      const filtered = filteredItems(draft.allItems, draft.keyword, draft.category);
      const currentLoaded = Number.parseInt(draft.nextCursor ?? "0", 10);
      const nextLoaded = Math.min(filtered.length, currentLoaded + APP_MANAGER_PAGE_SIZE);
      return {
        ...applyLocalFilterState(
          {
            allItems: draft.allItems,
            keyword: draft.keyword,
            category: draft.category,
            selectedAppId: draft.selectedAppId,
          },
          nextLoaded,
        ),
        loadingMore: false,
      };
    });
  },

  async refreshIndex() {
    set({ refreshing: true, error: null });
    try {
      const result = await appManagerRefreshIndex();
      set({ lastActionResult: result, indexDirty: true });
      await get().ensureSnapshotLoaded(true);
      set({ refreshing: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ refreshing: false, error: message });
    }
  },

  async loadDetail(appId, force = false) {
    const cachedDetail = get().detailById[appId];
    const loadedAt = get().detailLoadedAtById[appId] ?? 0;
    if (!force && cachedDetail && Date.now() - loadedAt <= DETAIL_CACHE_TTL_MS) {
      return;
    }

    const existingFlight = detailInFlight.get(appId);
    if (existingFlight) {
      return existingFlight;
    }

    const run = async () => {
      set((state) => ({
        detailLoadingById: updateActionLoading(state.detailLoadingById, appId, true),
        detailInFlightById: updateActionLoading(state.detailInFlightById, appId, true),
        detailError: null,
      }));
      try {
        const nextDetail = await appManagerGetDetail(appId);
        set((state) => {
          const nextDetailById = { ...state.detailById, [appId]: nextDetail };
          const nextLoadedAtById = { ...state.detailLoadedAtById, [appId]: Date.now() };

          const keys = Object.keys(nextDetailById);
          if (keys.length > DETAIL_CACHE_MAX) {
            const oldest = [...keys]
              .filter((key) => key !== appId)
              .sort((left, right) => (nextLoadedAtById[left] ?? 0) - (nextLoadedAtById[right] ?? 0))[0];
            if (oldest) {
              delete nextDetailById[oldest];
              delete nextLoadedAtById[oldest];
            }
          }

          return {
            detailById: nextDetailById,
            detailLoadedAtById: nextLoadedAtById,
            allItems: state.allItems.map((item) => (item.id === appId ? withDetailSize(item, nextDetail) : item)),
            items: state.items.map((item) => (item.id === appId ? withDetailSize(item, nextDetail) : item)),
          };
        });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        set({ detailError: message });
      } finally {
        detailInFlight.delete(appId);
        set((state) => ({
          detailLoadingById: updateActionLoading(state.detailLoadingById, appId, false),
          detailInFlightById: updateActionLoading(state.detailInFlightById, appId, false),
        }));
      }
    };

    const promise = run();
    detailInFlight.set(appId, promise);
    return promise;
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
    const app = state.allItems.find((item) => item.id === appId);
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
      set({ lastActionResult: result, indexDirty: true });
      await get().ensureSnapshotLoaded(true);
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
      set({ lastActionResult: result, indexDirty: true });
      await get().ensureSnapshotLoaded(true);
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

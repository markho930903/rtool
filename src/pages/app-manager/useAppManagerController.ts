import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type {
  AppManagerActionResult,
  AppManagerCleanupDeleteMode,
  AppManagerCleanupResult,
  AppManagerExportScanResult,
  AppManagerIndexUpdatedPayload,
  AppManagerResidueItem,
  AppManagerResidueScanResult,
  ManagedApp,
  ManagedAppDetail,
} from "@/components/app-manager/types";
import { useAsyncEffect } from "@/hooks/useAsyncEffect";
import {
  appManagerCleanup,
  appManagerExportScanResult,
  appManagerGetDetailCore,
  appManagerGetDetailHeavy,
  appManagerList,
  appManagerOpenDirectory,
  appManagerOpenUninstallHelp,
  appManagerRefreshIndex,
  appManagerResolveSizes,
  appManagerRevealPath,
  appManagerSetStartup,
  appManagerUninstall,
} from "@/services/app-manager.service";

const PAGE_SIZE = 120;
const SIZE_BATCH = 10;
const SIZE_PRIORITY_COUNT = 24;
const KEYWORD_DEBOUNCE_MS = 220;
type AppSizeState = "pending" | "resolving" | "exact" | "estimated";

function uniqueById(items: ManagedApp[]): ManagedApp[] {
  const map = new Map<string, ManagedApp>();
  for (const item of items) {
    map.set(item.id, item);
  }
  return [...map.values()];
}

function initialSizeState(item: ManagedApp): AppSizeState {
  if (item.sizeAccuracy === "exact" && item.sizeBytes !== null) {
    return "exact";
  }
  if (item.sizeBytes !== null) {
    return "estimated";
  }
  return "pending";
}

function retainById<T>(record: Record<string, T>, keep: Set<string>): Record<string, T> {
  if (Object.keys(record).length === 0) {
    return record;
  }
  const next: Record<string, T> = {};
  for (const [key, value] of Object.entries(record)) {
    if (keep.has(key)) {
      next[key] = value;
    }
  }
  return next;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

export function useAppManagerController() {
  const [items, setItems] = useState<ManagedApp[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [keyword, setKeyword] = useState("");
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [totalCount, setTotalCount] = useState(0);
  const [indexedAt, setIndexedAt] = useState<number | null>(null);
  const [revision, setRevision] = useState(0);
  const [indexState, setIndexState] = useState<"ready" | "building" | "degraded">("ready");
  const [listError, setListError] = useState<string | null>(null);

  const [selectedAppId, setSelectedAppId] = useState<string | null>(null);
  const [sizeStateByAppId, setSizeStateByAppId] = useState<Record<string, AppSizeState>>({});

  const [detailCoreById, setDetailCoreById] = useState<Record<string, ManagedAppDetail>>({});
  const [detailHeavyById, setDetailHeavyById] = useState<Record<string, AppManagerResidueScanResult>>({});
  const [detailCoreLoadingById, setDetailCoreLoadingById] = useState<Record<string, boolean>>({});
  const [detailHeavyLoadingById, setDetailHeavyLoadingById] = useState<Record<string, boolean>>({});
  const [detailError, setDetailError] = useState<string | null>(null);

  const [selectedResidueIdsByAppId, setSelectedResidueIdsByAppId] = useState<Record<string, string[]>>({});
  const [selectionTouchedByUserByAppId, setSelectionTouchedByUserByAppId] = useState<Record<string, boolean>>({});
  const [includeMainByAppId, setIncludeMainByAppId] = useState<Record<string, boolean>>({});
  const [deleteModeByAppId, setDeleteModeByAppId] = useState<Record<string, AppManagerCleanupDeleteMode>>({});
  const [cleanupLoadingByAppId, setCleanupLoadingByAppId] = useState<Record<string, boolean>>({});
  const [cleanupResultByAppId, setCleanupResultByAppId] = useState<Record<string, AppManagerCleanupResult>>({});
  const [cleanupError, setCleanupError] = useState<string | null>(null);
  const [startupLoadingByAppId, setStartupLoadingByAppId] = useState<Record<string, boolean>>({});
  const [uninstallLoadingByAppId, setUninstallLoadingByAppId] = useState<Record<string, boolean>>({});
  const [openHelpLoadingByAppId, setOpenHelpLoadingByAppId] = useState<Record<string, boolean>>({});
  const [exportLoadingByAppId, setExportLoadingByAppId] = useState<Record<string, boolean>>({});
  const [openExportDirLoadingByAppId, setOpenExportDirLoadingByAppId] = useState<Record<string, boolean>>({});
  const [exportResultByAppId, setExportResultByAppId] = useState<Record<string, AppManagerExportScanResult>>({});
  const [actionResultByAppId, setActionResultByAppId] = useState<Record<string, AppManagerActionResult>>({});
  const [actionError, setActionError] = useState<string | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);

  const sizeQueueRef = useRef<string[]>([]);
  const sizeQueuedSetRef = useRef<Set<string>>(new Set());
  const sizeFlushingRef = useRef(false);
  const sizeStateRef = useRef<Record<string, AppSizeState>>({});

  const itemsRef = useRef<ManagedApp[]>([]);
  const keywordRef = useRef("");
  const listRequestSeqRef = useRef(0);
  const firstListLoadDoneRef = useRef(false);

  const detailCoreByIdRef = useRef<Record<string, ManagedAppDetail>>({});
  const detailHeavyByIdRef = useRef<Record<string, AppManagerResidueScanResult>>({});
  const selectedResidueIdsByAppIdRef = useRef<Record<string, string[]>>({});
  const selectionTouchedByUserByAppIdRef = useRef<Record<string, boolean>>({});
  const detailHeavyRequestSeqByAppRef = useRef<Record<string, number>>({});
  const includeMainByAppIdRef = useRef<Record<string, boolean>>({});
  const deleteModeByAppIdRef = useRef<Record<string, AppManagerCleanupDeleteMode>>({});

  useEffect(() => {
    sizeStateRef.current = sizeStateByAppId;
  }, [sizeStateByAppId]);

  useEffect(() => {
    itemsRef.current = items;
  }, [items]);

  useEffect(() => {
    keywordRef.current = keyword;
  }, [keyword]);

  useEffect(() => {
    detailCoreByIdRef.current = detailCoreById;
  }, [detailCoreById]);

  useEffect(() => {
    detailHeavyByIdRef.current = detailHeavyById;
  }, [detailHeavyById]);

  useEffect(() => {
    selectedResidueIdsByAppIdRef.current = selectedResidueIdsByAppId;
  }, [selectedResidueIdsByAppId]);

  useEffect(() => {
    selectionTouchedByUserByAppIdRef.current = selectionTouchedByUserByAppId;
  }, [selectionTouchedByUserByAppId]);

  useEffect(() => {
    includeMainByAppIdRef.current = includeMainByAppId;
  }, [includeMainByAppId]);

  useEffect(() => {
    deleteModeByAppIdRef.current = deleteModeByAppId;
  }, [deleteModeByAppId]);

  useEffect(() => {
    setActionError(null);
    setExportError(null);
  }, [selectedAppId]);

  const updateSizeState = useCallback((appIds: string[], state: AppSizeState) => {
    setSizeStateByAppId((prev) => {
      const next = { ...prev };
      for (const appId of appIds) {
        if ((next[appId] ?? "pending") !== "exact") {
          next[appId] = state;
        }
      }
      return next;
    });
  }, []);

  const flushSizeQueue = useCallback(async () => {
    if (sizeFlushingRef.current) {
      return;
    }
    sizeFlushingRef.current = true;

    try {
      while (sizeQueueRef.current.length > 0) {
        const visibleIds = new Set(itemsRef.current.map((item) => item.id));
        const batch = sizeQueueRef.current.splice(0, SIZE_BATCH);
        for (const appId of batch) {
          sizeQueuedSetRef.current.delete(appId);
        }

        const candidates = batch.filter((appId) => {
          if (!visibleIds.has(appId)) {
            return false;
          }
          return (sizeStateRef.current[appId] ?? "pending") !== "exact";
        });
        if (candidates.length === 0) {
          continue;
        }

        updateSizeState(candidates, "resolving");
        try {
          const resolved = await appManagerResolveSizes({ appIds: candidates });
          const resolvedById = new Map(resolved.items.map((item) => [item.appId, item]));
          setItems((prev) =>
            prev.map((item) => {
              const next = resolvedById.get(item.id);
              if (!next) {
                return item;
              }
              return {
                ...item,
                sizeBytes: next.sizeBytes,
                sizeAccuracy: next.sizeAccuracy,
                sizeComputedAt: next.sizeComputedAt,
              };
            }),
          );
          setSizeStateByAppId((prev) => {
            const next = { ...prev };
            for (const appId of candidates) {
              const value = resolvedById.get(appId);
              if (!value) {
                next[appId] = "pending";
                continue;
              }
              next[appId] = value.sizeAccuracy === "exact" ? "exact" : "estimated";
            }
            return next;
          });
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          setListError(message);
          updateSizeState(candidates, "pending");
        }
        await delay(12);
      }
    } finally {
      sizeFlushingRef.current = false;
    }
  }, [updateSizeState]);

  const enqueueSizeResolution = useCallback(
    (appIds: string[], priority = false) => {
      const queue = sizeQueueRef.current;
      for (const appId of appIds) {
        const state = sizeStateRef.current[appId] ?? "pending";
        if (state === "exact") {
          continue;
        }
        if (sizeQueuedSetRef.current.has(appId)) {
          continue;
        }
        sizeQueuedSetRef.current.add(appId);
        if (priority) {
          queue.unshift(appId);
        } else {
          queue.push(appId);
        }
      }
      void flushSizeQueue();
    },
    [flushSizeQueue],
  );

  const applyListPage = useCallback(
    (nextItems: ManagedApp[], replace: boolean): ManagedApp[] => {
      const base = replace ? [] : itemsRef.current;
      const merged = replace ? nextItems : uniqueById([...base, ...nextItems]);
      const keep = new Set(merged.map((item) => item.id));

      itemsRef.current = merged;
      setItems(merged);

      setSizeStateByAppId((prev) => {
        const next: Record<string, AppSizeState> = replace ? {} : retainById(prev, keep);
        for (const item of merged) {
          if (!next[item.id]) {
            next[item.id] = initialSizeState(item);
          }
        }
        return next;
      });

      if (replace) {
        sizeQueueRef.current = [];
        sizeQueuedSetRef.current.clear();

        setDetailCoreById((prev) => retainById(prev, keep));
        setDetailHeavyById((prev) => retainById(prev, keep));
        setDetailCoreLoadingById((prev) => retainById(prev, keep));
        setDetailHeavyLoadingById((prev) => retainById(prev, keep));
        setSelectedResidueIdsByAppId((prev) => retainById(prev, keep));
        setSelectionTouchedByUserByAppId((prev) => retainById(prev, keep));
        setIncludeMainByAppId((prev) => retainById(prev, keep));
        setDeleteModeByAppId((prev) => retainById(prev, keep));
        setCleanupLoadingByAppId((prev) => retainById(prev, keep));
        setCleanupResultByAppId((prev) => retainById(prev, keep));
        setStartupLoadingByAppId((prev) => retainById(prev, keep));
        setUninstallLoadingByAppId((prev) => retainById(prev, keep));
        setOpenHelpLoadingByAppId((prev) => retainById(prev, keep));
        setExportLoadingByAppId((prev) => retainById(prev, keep));
        setOpenExportDirLoadingByAppId((prev) => retainById(prev, keep));
        setExportResultByAppId((prev) => retainById(prev, keep));
        setActionResultByAppId((prev) => retainById(prev, keep));
      }

      const priority = nextItems.slice(0, SIZE_PRIORITY_COUNT).map((item) => item.id);
      const rest = nextItems.slice(SIZE_PRIORITY_COUNT).map((item) => item.id);
      enqueueSizeResolution(priority, true);
      enqueueSizeResolution(rest, false);

      return merged;
    },
    [enqueueSizeResolution],
  );

  const loadListPage = useCallback(
    async (cursor?: string, replace = false, keywordValue?: string) => {
      const requestSeq = ++listRequestSeqRef.current;
      if (replace) {
        setLoading(true);
      } else {
        setLoadingMore(true);
      }
      if (replace) {
        setListError(null);
      }

      try {
        const keywordText = (keywordValue ?? keywordRef.current).trim();
        const page = await appManagerList({
          keyword: keywordText ? keywordText : undefined,
          limit: PAGE_SIZE,
          cursor,
        });
        if (requestSeq !== listRequestSeqRef.current) {
          return;
        }

        const mergedItems = applyListPage(page.items, replace);
        setNextCursor(page.nextCursor);
        setIndexedAt(page.indexedAt);
        setRevision(page.revision);
        setIndexState(page.indexState);
        setTotalCount(page.totalCount);
        setSelectedAppId((current) => {
          if (current && mergedItems.some((item) => item.id === current)) {
            return current;
          }
          return mergedItems[0]?.id ?? null;
        });
      } catch (error) {
        if (requestSeq !== listRequestSeqRef.current) {
          return;
        }
        setListError(error instanceof Error ? error.message : String(error));
      } finally {
        if (requestSeq === listRequestSeqRef.current) {
          if (replace) {
            setLoading(false);
          } else {
            setLoadingMore(false);
          }
        }
      }
    },
    [applyListPage],
  );

  const loadListFirstPage = useCallback(
    async (keywordValue?: string) => {
      await loadListPage(undefined, true, keywordValue);
    },
    [loadListPage],
  );

  const refreshList = useCallback(async () => {
    setRefreshing(true);
    try {
      await appManagerRefreshIndex();
      await loadListFirstPage(keywordRef.current);
    } catch (error) {
      setListError(error instanceof Error ? error.message : String(error));
    } finally {
      setRefreshing(false);
    }
  }, [loadListFirstPage]);

  const loadDetailCore = useCallback(async (appId: string, force = false) => {
    if (!force && detailCoreByIdRef.current[appId]) {
      return;
    }
    setDetailCoreLoadingById((prev) => ({ ...prev, [appId]: true }));
    setDetailError(null);
    try {
      const detail = await appManagerGetDetailCore(appId);
      setDetailCoreById((prev) => ({ ...prev, [appId]: detail }));
    } catch (error) {
      setDetailError(error instanceof Error ? error.message : String(error));
    } finally {
      setDetailCoreLoadingById((prev) => ({ ...prev, [appId]: false }));
    }
  }, []);

  const applyHeavyResult = useCallback((appId: string, heavy: AppManagerResidueScanResult) => {
    setDetailHeavyById((prev) => ({ ...prev, [appId]: heavy }));

    const allItemIds = new Set(heavy.groups.flatMap((group) => group.items.map((item) => item.itemId)));
    const recommendedIds = heavy.groups
      .flatMap((group) => group.items)
      .filter((item) => item.recommended)
      .map((item) => item.itemId);

    setSelectedResidueIdsByAppId((prev) => {
      const existing = (prev[appId] ?? []).filter((itemId) => allItemIds.has(itemId));
      const touched = selectionTouchedByUserByAppIdRef.current[appId] ?? false;
      const nextSelection = touched ? existing : [...new Set([...existing, ...recommendedIds])];
      return { ...prev, [appId]: nextSelection };
    });

    setSelectionTouchedByUserByAppId((prev) => ({ ...prev, [appId]: prev[appId] ?? false }));
    setIncludeMainByAppId((prev) => ({ ...prev, [appId]: prev[appId] ?? true }));
    setDeleteModeByAppId((prev) => ({ ...prev, [appId]: prev[appId] ?? "trash" }));
  }, []);

  const loadDetailHeavy = useCallback(
    async (appId: string, force = false) => {
      const cached = detailHeavyByIdRef.current[appId];
      if (!force && cached?.scanMode === "deep") {
        return;
      }

      const nextSeq = (detailHeavyRequestSeqByAppRef.current[appId] ?? 0) + 1;
      detailHeavyRequestSeqByAppRef.current[appId] = nextSeq;
      const isLatest = () => detailHeavyRequestSeqByAppRef.current[appId] === nextSeq;

      setDetailHeavyLoadingById((prev) => ({ ...prev, [appId]: true }));
      setDetailError(null);
      try {
        if (!force) {
          try {
            const quick = await appManagerGetDetailHeavy(appId, "quick");
            if (isLatest()) {
              applyHeavyResult(appId, quick);
            }
          } catch {
            // Quick 失败时继续 deep，避免详情空白。
          }
        }
        const deep = await appManagerGetDetailHeavy(appId, "deep");
        if (!isLatest()) {
          return;
        }
        applyHeavyResult(appId, deep);
      } catch (error) {
        if (!isLatest()) {
          return;
        }
        setDetailError(error instanceof Error ? error.message : String(error));
      } finally {
        if (isLatest()) {
          setDetailHeavyLoadingById((prev) => ({ ...prev, [appId]: false }));
        }
      }
    },
    [applyHeavyResult],
  );

  useEffect(() => {
    const delayMs = firstListLoadDoneRef.current ? KEYWORD_DEBOUNCE_MS : 40;
    const timer = window.setTimeout(() => {
      firstListLoadDoneRef.current = true;
      void loadListFirstPage(keyword);
    }, delayMs);
    return () => {
      window.clearTimeout(timer);
    };
  }, [keyword, loadListFirstPage]);

  useAsyncEffect(
    async ({ stack }) => {
      stack.add(() => {
        listRequestSeqRef.current += 1;
      }, "invalidate-list-request-seq");

      const stop = await listen<AppManagerIndexUpdatedPayload>("rtool://app-manager/index-updated", (event) => {
        if (!event.payload) {
          return;
        }
        setRevision((prev) => Math.max(prev, event.payload.revision));
        setIndexedAt(event.payload.indexedAt);
        void loadListFirstPage(keywordRef.current);
      });
      stack.add(stop, "index-updated");
    },
    [loadListFirstPage],
    {
      scope: "app-manager",
      onError: (error) => {
        if (import.meta.env.DEV) {
          console.warn("[app-manager] event listen setup failed", error);
        }
      },
    },
  );

  useEffect(() => {
    if (!selectedAppId) {
      return;
    }
    void loadDetailCore(selectedAppId);
    void loadDetailHeavy(selectedAppId);
    enqueueSizeResolution([selectedAppId], true);
  }, [enqueueSizeResolution, loadDetailCore, loadDetailHeavy, selectedAppId]);

  const selectedApp = useMemo(() => items.find((item) => item.id === selectedAppId) ?? null, [items, selectedAppId]);
  const selectedCore = selectedApp ? (detailCoreById[selectedApp.id] ?? null) : null;
  const selectedHeavy = selectedApp ? (detailHeavyById[selectedApp.id] ?? null) : null;
  const selectedCoreLoading = selectedApp ? Boolean(detailCoreLoadingById[selectedApp.id]) : false;
  const selectedHeavyLoading = selectedApp ? Boolean(detailHeavyLoadingById[selectedApp.id]) : false;
  const selectedDeepCompleting = selectedApp
    ? Boolean(detailHeavyLoadingById[selectedApp.id] && detailHeavyById[selectedApp.id]?.scanMode === "quick")
    : false;
  const selectedResidueIds = selectedApp ? (selectedResidueIdsByAppId[selectedApp.id] ?? []) : [];
  const selectedIncludeMain = selectedApp ? (includeMainByAppId[selectedApp.id] ?? true) : true;
  const selectedDeleteMode = selectedApp ? (deleteModeByAppId[selectedApp.id] ?? "trash") : "trash";
  const selectedCleanupResult = selectedApp ? (cleanupResultByAppId[selectedApp.id] ?? null) : null;
  const selectedCleanupLoading = selectedApp ? Boolean(cleanupLoadingByAppId[selectedApp.id]) : false;
  const selectedStartupLoading = selectedApp ? Boolean(startupLoadingByAppId[selectedApp.id]) : false;
  const selectedUninstallLoading = selectedApp ? Boolean(uninstallLoadingByAppId[selectedApp.id]) : false;
  const selectedOpenHelpLoading = selectedApp ? Boolean(openHelpLoadingByAppId[selectedApp.id]) : false;
  const selectedExportLoading = selectedApp ? Boolean(exportLoadingByAppId[selectedApp.id]) : false;
  const selectedOpenExportDirLoading = selectedApp ? Boolean(openExportDirLoadingByAppId[selectedApp.id]) : false;
  const selectedExportResult = selectedApp ? (exportResultByAppId[selectedApp.id] ?? null) : null;
  const selectedActionResult = selectedApp ? (actionResultByAppId[selectedApp.id] ?? null) : null;
  const selectedDetailError = detailError ?? listError;

  const toggleResidue = useCallback((appId: string, itemId: string, checked: boolean) => {
    setSelectedResidueIdsByAppId((prev) => {
      const current = new Set(prev[appId] ?? []);
      if (checked) {
        current.add(itemId);
      } else {
        current.delete(itemId);
      }
      return { ...prev, [appId]: [...current] };
    });
    setSelectionTouchedByUserByAppId((prev) => ({ ...prev, [appId]: true }));
  }, []);

  const setSelectedResidues = useCallback((appId: string, itemIds: string[]) => {
    setSelectedResidueIdsByAppId((prev) => ({ ...prev, [appId]: [...new Set(itemIds)] }));
    setSelectionTouchedByUserByAppId((prev) => ({ ...prev, [appId]: true }));
  }, []);

  const setIncludeMain = useCallback((appId: string, includeMain: boolean) => {
    setIncludeMainByAppId((prev) => ({ ...prev, [appId]: includeMain }));
  }, []);

  const setDeleteMode = useCallback((appId: string, mode: AppManagerCleanupDeleteMode) => {
    setDeleteModeByAppId((prev) => ({ ...prev, [appId]: mode }));
  }, []);

  const toggleSelectedResidue = useCallback(
    (itemId: string, checked: boolean) => {
      if (!selectedAppId) {
        return;
      }
      toggleResidue(selectedAppId, itemId, checked);
    },
    [selectedAppId, toggleResidue],
  );

  const setSelectedIncludeMain = useCallback(
    (includeMain: boolean) => {
      if (!selectedAppId) {
        return;
      }
      setIncludeMain(selectedAppId, includeMain);
    },
    [selectedAppId, setIncludeMain],
  );

  const setSelectedDeleteMode = useCallback(
    (mode: AppManagerCleanupDeleteMode) => {
      if (!selectedAppId) {
        return;
      }
      setDeleteMode(selectedAppId, mode);
    },
    [selectedAppId, setDeleteMode],
  );

  const selectAllResiduesForSelectedApp = useCallback(
    (itemIds: string[]) => {
      if (!selectedAppId) {
        return;
      }
      setSelectedResidues(selectedAppId, itemIds);
    },
    [selectedAppId, setSelectedResidues],
  );

  const runCleanup = useCallback(
    async (
      app: ManagedApp,
      payload: {
        selectedItemIds: string[];
        includeMainApp?: boolean;
        deleteMode: AppManagerCleanupDeleteMode;
      },
    ) => {
      const includeMainApp = payload.includeMainApp ?? true;
      if (!includeMainApp && payload.selectedItemIds.length === 0) {
        setCleanupError("app_manager_cleanup_selection_required");
        return;
      }

      const appId = app.id;
      setCleanupLoadingByAppId((prev) => ({ ...prev, [appId]: true }));
      setCleanupError(null);
      try {
        const result = await appManagerCleanup({
          appId,
          selectedItemIds: payload.selectedItemIds,
          includeMainApp,
          deleteMode: payload.deleteMode,
          skipOnError: true,
          confirmedFingerprint: includeMainApp ? app.fingerprint : undefined,
        });
        setCleanupResultByAppId((prev) => ({ ...prev, [appId]: result }));
        setSelectedResidueIdsByAppId((prev) => ({ ...prev, [appId]: [] }));
        setSelectionTouchedByUserByAppId((prev) => ({ ...prev, [appId]: false }));
        await loadListFirstPage(keywordRef.current);
        if (itemsRef.current.some((item) => item.id === appId)) {
          await Promise.all([loadDetailCore(appId, true), loadDetailHeavy(appId, true)]);
        }
      } catch (error) {
        setCleanupError(error instanceof Error ? error.message : String(error));
      } finally {
        setCleanupLoadingByAppId((prev) => ({ ...prev, [appId]: false }));
      }
    },
    [loadDetailCore, loadDetailHeavy, loadListFirstPage],
  );

  const cleanupNow = useCallback(async () => {
    if (!selectedApp) {
      return;
    }
    const appId = selectedApp.id;
    await runCleanup(selectedApp, {
      selectedItemIds: selectedResidueIdsByAppIdRef.current[appId] ?? [],
      includeMainApp: includeMainByAppIdRef.current[appId] ?? true,
      deleteMode: deleteModeByAppIdRef.current[appId] ?? "trash",
    });
  }, [runCleanup, selectedApp]);

  const retryFailed = useCallback(async () => {
    if (!selectedApp || !selectedCleanupResult) {
      return;
    }
    const retryMainApp = selectedCleanupResult.failed.some((item) => item.itemId === "main-app");
    const retryIds = selectedCleanupResult.failed.map((item) => item.itemId).filter((itemId) => itemId !== "main-app");
    if (!retryMainApp && retryIds.length === 0) {
      return;
    }
    const dedupedRetryIds = [...new Set(retryIds)];
    setSelectedResidueIdsByAppId((prev) => ({ ...prev, [selectedApp.id]: dedupedRetryIds }));
    setIncludeMainByAppId((prev) => ({ ...prev, [selectedApp.id]: retryMainApp }));
    await runCleanup(selectedApp, {
      selectedItemIds: dedupedRetryIds,
      includeMainApp: retryMainApp,
      deleteMode: deleteModeByAppIdRef.current[selectedApp.id] ?? "trash",
    });
  }, [runCleanup, selectedApp, selectedCleanupResult]);

  const toggleStartup = useCallback(async () => {
    if (!selectedApp) {
      return;
    }
    const appId = selectedApp.id;
    setStartupLoadingByAppId((prev) => ({ ...prev, [appId]: true }));
    setActionError(null);
    try {
      const result = await appManagerSetStartup({
        appId,
        enabled: !selectedApp.startupEnabled,
      });
      setActionResultByAppId((prev) => ({ ...prev, [appId]: result }));
      await loadListFirstPage(keywordRef.current);
      await loadDetailCore(appId, true);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setStartupLoadingByAppId((prev) => ({ ...prev, [appId]: false }));
    }
  }, [loadDetailCore, loadListFirstPage, selectedApp]);

  const runUninstall = useCallback(async () => {
    if (!selectedApp) {
      return;
    }
    const appId = selectedApp.id;
    setUninstallLoadingByAppId((prev) => ({ ...prev, [appId]: true }));
    setActionError(null);
    try {
      const result = await appManagerUninstall({
        appId,
        confirmedFingerprint: selectedApp.fingerprint,
      });
      setActionResultByAppId((prev) => ({ ...prev, [appId]: result }));
      await loadListFirstPage(keywordRef.current);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setUninstallLoadingByAppId((prev) => ({ ...prev, [appId]: false }));
    }
  }, [loadListFirstPage, selectedApp]);

  const openUninstallHelp = useCallback(async () => {
    if (!selectedApp) {
      return;
    }
    const appId = selectedApp.id;
    setOpenHelpLoadingByAppId((prev) => ({ ...prev, [appId]: true }));
    setActionError(null);
    try {
      const result = await appManagerOpenUninstallHelp(appId);
      setActionResultByAppId((prev) => ({ ...prev, [appId]: result }));
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setOpenHelpLoadingByAppId((prev) => ({ ...prev, [appId]: false }));
    }
  }, [selectedApp]);

  const exportScanResult = useCallback(async () => {
    if (!selectedApp) {
      return;
    }
    const appId = selectedApp.id;
    setExportLoadingByAppId((prev) => ({ ...prev, [appId]: true }));
    setExportError(null);
    try {
      const result = await appManagerExportScanResult(appId);
      setExportResultByAppId((prev) => ({ ...prev, [appId]: result }));
    } catch (error) {
      setExportError(error instanceof Error ? error.message : String(error));
    } finally {
      setExportLoadingByAppId((prev) => ({ ...prev, [appId]: false }));
    }
  }, [selectedApp]);

  const openExportDirectory = useCallback(async () => {
    if (!selectedApp) {
      return;
    }
    const appId = selectedApp.id;
    const exported = exportResultByAppId[appId];
    if (!exported) {
      setExportError("app_manager_export_missing_result");
      return;
    }
    setOpenExportDirLoadingByAppId((prev) => ({ ...prev, [appId]: true }));
    setExportError(null);
    try {
      await appManagerOpenDirectory(exported.directoryPath);
    } catch (error) {
      setExportError(error instanceof Error ? error.message : String(error));
    } finally {
      setOpenExportDirLoadingByAppId((prev) => ({ ...prev, [appId]: false }));
    }
  }, [exportResultByAppId, selectedApp]);

  const revealPath = useCallback(async (path: string) => {
    await appManagerRevealPath(path);
  }, []);

  const scanAgain = useCallback(async () => {
    if (!selectedApp) {
      return;
    }
    await loadDetailHeavy(selectedApp.id, true);
  }, [loadDetailHeavy, selectedApp]);

  const hasMore = Boolean(nextCursor);

  const onLoadMore = useCallback(async () => {
    if (!nextCursor || loadingMore) {
      return;
    }
    await loadListPage(nextCursor, false, keywordRef.current);
  }, [loadListPage, loadingMore, nextCursor]);

  const list = {
    items,
    loading: loading || refreshing,
    loadingMore,
    hasMore,
    keyword,
    totalCount,
    indexedAt,
    revision,
    indexState,
    listError,
    selectedAppId,
  };

  const detail = {
    selectedApp,
    coreDetail: selectedCore,
    heavyDetail: selectedHeavy,
    coreLoading: selectedCoreLoading,
    heavyLoading: selectedHeavyLoading,
    deepCompleting: selectedDeepCompleting,
    detailError: selectedDetailError,
    selectedResidueIds,
    selectedIncludeMain,
    selectedDeleteMode,
    cleanupLoading: selectedCleanupLoading,
    cleanupResult: selectedCleanupResult,
    cleanupError,
    startupLoading: selectedStartupLoading,
    uninstallLoading: selectedUninstallLoading,
    openHelpLoading: selectedOpenHelpLoading,
    exportLoading: selectedExportLoading,
    openExportDirLoading: selectedOpenExportDirLoading,
    exportResult: selectedExportResult,
    exportError,
    actionResult: selectedActionResult,
    actionError,
  };

  const actions = {
    setKeyword,
    setSelectedAppId,
    refreshList,
    onLoadMore,
    onToggleResidue: toggleSelectedResidue,
    onSelectAllResidues: selectAllResiduesForSelectedApp,
    onToggleIncludeMain: setSelectedIncludeMain,
    onSetDeleteMode: setSelectedDeleteMode,
    onCleanupNow: cleanupNow,
    onRetryFailed: retryFailed,
    onRevealPath: revealPath,
    onScanAgain: scanAgain,
    onToggleStartup: toggleStartup,
    onOpenUninstallHelp: openUninstallHelp,
    onUninstall: runUninstall,
    onExportScanResult: exportScanResult,
    onOpenExportDirectory: openExportDirectory,
  };

  return { list, detail, actions };
}

export type AppManagerController = ReturnType<typeof useAppManagerController>;
export type AppManagerResidueEntry = AppManagerResidueItem;

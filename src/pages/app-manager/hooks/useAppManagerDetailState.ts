import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type {
  AppManagerCleanupDeleteMode,
  AppManagerResidueScanResult,
} from "@/components/app-manager/types";
import { useLatestRef } from "@/hooks/useLatestRef";
import { appManagerGetDetailCore, appManagerGetDetailHeavy } from "@/services/app-manager.service";

import {
  getPerAppUiState,
  retainById,
  updatePerAppUiState,
  type AppManagerPerAppUiState,
} from "./state";

interface UseAppManagerDetailStateOptions {
  selectedAppId: string | null;
}

export function useAppManagerDetailState(options: UseAppManagerDetailStateOptions) {
  const { selectedAppId } = options;
  const [uiStateByAppId, setUiStateByAppId] = useState<Record<string, AppManagerPerAppUiState>>({});

  const uiStateByAppIdRef = useLatestRef(uiStateByAppId);
  const selectedAppIdRef = useLatestRef(selectedAppId);
  const coreRequestSeqByAppRef = useRef<Record<string, number>>({});
  const heavyRequestSeqByAppRef = useRef<Record<string, number>>({});

  const updateAppUiState = useCallback(
    (appId: string, updater: (state: AppManagerPerAppUiState) => AppManagerPerAppUiState) => {
      setUiStateByAppId((previous) => updatePerAppUiState(previous, appId, updater));
    },
    [],
  );

  const setAppUiStatePatch = useCallback(
    (appId: string, patch: Partial<AppManagerPerAppUiState>) => {
      updateAppUiState(appId, (state) => ({ ...state, ...patch }));
    },
    [updateAppUiState],
  );

  const getAppUiState = useCallback(
    (appId: string | null | undefined) => getPerAppUiState(uiStateByAppIdRef.current, appId),
    [uiStateByAppIdRef],
  );

  const pruneAppState = useCallback((keepIds: Set<string>) => {
    setUiStateByAppId((previous) => retainById(previous, keepIds));

    const nextCoreSeq: Record<string, number> = {};
    for (const [appId, seq] of Object.entries(coreRequestSeqByAppRef.current)) {
      if (keepIds.has(appId)) {
        nextCoreSeq[appId] = seq;
      }
    }
    coreRequestSeqByAppRef.current = nextCoreSeq;

    const nextHeavySeq: Record<string, number> = {};
    for (const [appId, seq] of Object.entries(heavyRequestSeqByAppRef.current)) {
      if (keepIds.has(appId)) {
        nextHeavySeq[appId] = seq;
      }
    }
    heavyRequestSeqByAppRef.current = nextHeavySeq;
  }, []);

  const loadDetailCore = useCallback(async (appId: string, force = false) => {
    const existing = uiStateByAppIdRef.current[appId]?.coreDetail;
    if (!force && existing) {
      return;
    }

    const nextSeq = (coreRequestSeqByAppRef.current[appId] ?? 0) + 1;
    coreRequestSeqByAppRef.current[appId] = nextSeq;
    const isLatest = () => coreRequestSeqByAppRef.current[appId] === nextSeq;

    setAppUiStatePatch(appId, {
      coreLoading: true,
      detailError: null,
    });

    try {
      const detail = await appManagerGetDetailCore(appId);
      if (!isLatest()) {
        return;
      }
      setAppUiStatePatch(appId, {
        coreDetail: detail,
        detailError: null,
      });
    } catch (error) {
      if (!isLatest()) {
        return;
      }
      setAppUiStatePatch(appId, {
        detailError: error instanceof Error ? error.message : String(error),
      });
    } finally {
      if (isLatest()) {
        setAppUiStatePatch(appId, {
          coreLoading: false,
        });
      }
    }
  }, [setAppUiStatePatch, uiStateByAppIdRef]);

  const applyHeavyResult = useCallback(
    (appId: string, heavy: AppManagerResidueScanResult) => {
      updateAppUiState(appId, (state) => {
        const allItemIds = new Set(heavy.groups.flatMap((group) => group.items.map((item) => item.itemId)));
        const recommendedIds = heavy.groups
          .flatMap((group) => group.items)
          .filter((item) => item.recommended)
          .map((item) => item.itemId);
        const existingSelected = state.selectedResidueIds.filter((itemId) => allItemIds.has(itemId));
        const selectedResidueIds = state.selectionTouchedByUser
          ? existingSelected
          : [...new Set([...existingSelected, ...recommendedIds])];

        return {
          ...state,
          heavyDetail: heavy,
          selectedResidueIds,
          includeMain: state.includeMain,
          deleteMode: state.deleteMode,
        };
      });
    },
    [updateAppUiState],
  );

  const loadDetailHeavy = useCallback(
    async (appId: string, force = false) => {
      const cached = uiStateByAppIdRef.current[appId]?.heavyDetail;
      if (!force && cached?.scanMode === "deep") {
        return;
      }

      const nextSeq = (heavyRequestSeqByAppRef.current[appId] ?? 0) + 1;
      heavyRequestSeqByAppRef.current[appId] = nextSeq;
      const isLatest = () => heavyRequestSeqByAppRef.current[appId] === nextSeq;

      setAppUiStatePatch(appId, {
        heavyLoading: true,
        detailError: null,
      });

      try {
        if (!force) {
          try {
            const quick = await appManagerGetDetailHeavy(appId, "quick");
            if (isLatest()) {
              applyHeavyResult(appId, quick);
            }
          } catch {
            // quick 失败时继续 deep，避免详情空白。
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
        setAppUiStatePatch(appId, {
          detailError: error instanceof Error ? error.message : String(error),
        });
      } finally {
        if (isLatest()) {
          setAppUiStatePatch(appId, {
            heavyLoading: false,
          });
        }
      }
    },
    [applyHeavyResult, setAppUiStatePatch, uiStateByAppIdRef],
  );

  useEffect(() => {
    if (!selectedAppId) {
      return;
    }
    void loadDetailCore(selectedAppId);
    void loadDetailHeavy(selectedAppId);
  }, [loadDetailCore, loadDetailHeavy, selectedAppId]);

  const toggleResidue = useCallback(
    (appId: string, itemId: string, checked: boolean) => {
      updateAppUiState(appId, (state) => {
        const nextSelectedIds = new Set(state.selectedResidueIds);
        if (checked) {
          nextSelectedIds.add(itemId);
        } else {
          nextSelectedIds.delete(itemId);
        }
        return {
          ...state,
          selectedResidueIds: [...nextSelectedIds],
          selectionTouchedByUser: true,
        };
      });
    },
    [updateAppUiState],
  );

  const setSelectedResidues = useCallback(
    (appId: string, itemIds: string[]) => {
      updateAppUiState(appId, (state) => ({
        ...state,
        selectedResidueIds: [...new Set(itemIds)],
        selectionTouchedByUser: true,
      }));
    },
    [updateAppUiState],
  );

  const setIncludeMain = useCallback(
    (appId: string, includeMain: boolean) => {
      updateAppUiState(appId, (state) => ({ ...state, includeMain }));
    },
    [updateAppUiState],
  );

  const setDeleteMode = useCallback(
    (appId: string, deleteMode: AppManagerCleanupDeleteMode) => {
      updateAppUiState(appId, (state) => ({ ...state, deleteMode }));
    },
    [updateAppUiState],
  );

  const selectedUiState = useMemo(() => getPerAppUiState(uiStateByAppId, selectedAppId), [selectedAppId, uiStateByAppId]);
  const selectedDeepCompleting = Boolean(selectedUiState.heavyLoading && selectedUiState.heavyDetail?.scanMode === "quick");

  const toggleSelectedResidue = useCallback(
    (itemId: string, checked: boolean) => {
      if (!selectedAppIdRef.current) {
        return;
      }
      toggleResidue(selectedAppIdRef.current, itemId, checked);
    },
    [selectedAppIdRef, toggleResidue],
  );

  const selectAllResiduesForSelectedApp = useCallback(
    (itemIds: string[]) => {
      if (!selectedAppIdRef.current) {
        return;
      }
      setSelectedResidues(selectedAppIdRef.current, itemIds);
    },
    [selectedAppIdRef, setSelectedResidues],
  );

  const setSelectedIncludeMain = useCallback(
    (includeMain: boolean) => {
      if (!selectedAppIdRef.current) {
        return;
      }
      setIncludeMain(selectedAppIdRef.current, includeMain);
    },
    [selectedAppIdRef, setIncludeMain],
  );

  const setSelectedDeleteMode = useCallback(
    (mode: AppManagerCleanupDeleteMode) => {
      if (!selectedAppIdRef.current) {
        return;
      }
      setDeleteMode(selectedAppIdRef.current, mode);
    },
    [selectedAppIdRef, setDeleteMode],
  );

  return {
    uiStateByAppId,
    uiStateByAppIdRef,
    selectedUiState,
    selectedDeepCompleting,
    getAppUiState,
    loadDetailCore,
    loadDetailHeavy,
    pruneAppState,
    setAppUiStatePatch,
    updateAppUiState,
    toggleResidue,
    toggleSelectedResidue,
    setSelectedResidues,
    selectAllResiduesForSelectedApp,
    setIncludeMain,
    setSelectedIncludeMain,
    setDeleteMode,
    setSelectedDeleteMode,
  };
}

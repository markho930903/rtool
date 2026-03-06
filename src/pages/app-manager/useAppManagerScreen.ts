import { useCallback, useEffect, useMemo } from "react";

import { useLatestRef } from "@/hooks/useLatestRef";
import type { AppDetailPaneModel } from "@/pages/app-manager/AppDetailPane";
import type { AppListPaneModel } from "@/pages/app-manager/AppListPane";

import { useAppManagerActions } from "./hooks/useAppManagerActions";
import { useAppManagerDetailState } from "./hooks/useAppManagerDetailState";
import { useAppManagerListState } from "./hooks/useAppManagerListState";

export function useAppManagerScreen() {
  const listState = useAppManagerListState();
  const {
    enqueueSizeResolution,
    items,
    keyword,
    listError,
    listModel,
    loadListFirstPage,
    refreshList,
    selectApp,
    selectedAppId,
    selectedAppIdRef,
    setKeyword,
  } = listState;

  const detailState = useAppManagerDetailState({ selectedAppId });
  const {
    getAppUiState,
    loadDetailCore,
    loadDetailHeavy,
    pruneAppState,
    selectAllResiduesForSelectedApp,
    selectedDeepCompleting,
    selectedUiState,
    setAppUiStatePatch,
    setSelectedDeleteMode,
    setSelectedIncludeMain,
    toggleSelectedResidue,
    updateAppUiState,
  } = detailState;

  const itemsRef = useLatestRef(items);
  const keywordRef = useLatestRef(keyword);

  useEffect(() => {
    pruneAppState(new Set(items.map((item) => item.id)));
  }, [items, pruneAppState]);

  useEffect(() => {
    if (!selectedAppId) {
      return;
    }
    enqueueSizeResolution([selectedAppId], true);
  }, [enqueueSizeResolution, selectedAppId]);

  const selectedApp = useMemo(
    () => items.find((item) => item.id === selectedAppId) ?? null,
    [items, selectedAppId],
  );

  const actions = useAppManagerActions({
    selectedApp,
    getCurrentKeyword: () => keywordRef.current,
    getAppUiState,
    hasApp: (appId: string) => itemsRef.current.some((item) => item.id === appId),
    loadListFirstPage,
    loadDetailCore,
    loadDetailHeavy,
    setAppUiStatePatch,
    updateAppUiState,
  });

  const handleSelectApp = useCallback(
    (appId: string) => {
      selectApp(appId);
      if (selectedAppIdRef.current === appId) {
        void loadDetailCore(appId);
        void loadDetailHeavy(appId);
        enqueueSizeResolution([appId], true);
      }
    },
    [enqueueSizeResolution, loadDetailCore, loadDetailHeavy, selectApp, selectedAppIdRef],
  );

  const listPaneModel: AppListPaneModel = {
    items,
    loading: listModel.loading,
    loadingMore: listModel.loadingMore,
    hasMore: listModel.hasMore,
    listError,
    keyword,
    indexedAtText: listModel.indexedAtText,
    revision: listModel.revision,
    indexState: listModel.indexState,
    totalCount: listModel.totalCount,
    selectedAppId,
    onKeywordChange: setKeyword,
    onSelect: handleSelectApp,
    onRefresh: refreshList,
    onLoadMore: listModel.onLoadMore,
  };

  const detailPaneModel: AppDetailPaneModel = {
    detail: {
      selectedApp,
      coreDetail: selectedUiState.coreDetail,
      heavyDetail: selectedUiState.heavyDetail,
      coreLoading: selectedUiState.coreLoading,
      heavyLoading: selectedUiState.heavyLoading,
      deepCompleting: selectedDeepCompleting,
      detailError: selectedUiState.detailError,
    },
    cleanup: {
      selectedResidueIds: selectedUiState.selectedResidueIds,
      selectedIncludeMain: selectedUiState.includeMain,
      selectedDeleteMode: selectedUiState.deleteMode,
      cleanupLoading: selectedUiState.cleanupLoading,
      cleanupResult: selectedUiState.cleanupResult,
      cleanupError: selectedUiState.cleanupError,
    },
    operations: {
      startupLoading: selectedUiState.startupLoading,
      uninstallLoading: selectedUiState.uninstallLoading,
      openHelpLoading: selectedUiState.openHelpLoading,
      openPermissionHelpLoading: selectedUiState.openPermissionHelpLoading,
      exportLoading: selectedUiState.exportLoading,
      openExportDirLoading: selectedUiState.openExportDirLoading,
      exportResult: selectedUiState.exportResult,
      exportError: selectedUiState.exportError,
      actionResult: selectedUiState.actionResult,
      actionError: selectedUiState.actionError,
    },
    actions: {
      onToggleResidue: toggleSelectedResidue,
      onSelectAllResidues: selectAllResiduesForSelectedApp,
      onToggleIncludeMain: setSelectedIncludeMain,
      onSetDeleteMode: setSelectedDeleteMode,
      onCleanupNow: actions.cleanupNow,
      onRetryFailed: actions.retryFailed,
      onRevealPath: actions.revealPath,
      onScanAgain: actions.scanAgain,
      onToggleStartup: actions.toggleStartup,
      onOpenUninstallHelp: actions.openUninstallHelp,
      onOpenPermissionHelp: actions.openPermissionHelp,
      onUninstall: actions.runUninstall,
      onExportScanResult: actions.exportScanResult,
      onOpenExportDirectory: actions.openExportDirectory,
    },
  };

  return { listPaneModel, detailPaneModel };
}

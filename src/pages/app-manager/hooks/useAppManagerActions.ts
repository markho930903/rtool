import { useCallback } from "react";

import type {
  AppManagerCleanupDeleteMode,
  AppManagerExportScanResult,
  ManagedApp,
} from "@/components/app-manager/types";
import {
  appManagerCleanup,
  appManagerExportScanResult,
  appManagerOpenDirectory,
  appManagerOpenPermissionHelp,
  appManagerOpenUninstallHelp,
  appManagerRevealPath,
  appManagerSetStartup,
  appManagerUninstall,
} from "@/services/app-manager.service";

import type { AppManagerPerAppUiState } from "./state";

interface UseAppManagerActionsOptions {
  selectedApp: ManagedApp | null;
  getCurrentKeyword: () => string;
  getAppUiState: (appId: string | null | undefined) => AppManagerPerAppUiState;
  hasApp: (appId: string) => boolean;
  loadListFirstPage: (keywordValue?: string) => Promise<void>;
  loadDetailCore: (appId: string, force?: boolean) => Promise<void>;
  loadDetailHeavy: (appId: string, force?: boolean) => Promise<void>;
  setAppUiStatePatch: (appId: string, patch: Partial<AppManagerPerAppUiState>) => void;
  updateAppUiState: (appId: string, updater: (state: AppManagerPerAppUiState) => AppManagerPerAppUiState) => void;
}

interface SelectedAppActionConfig<T> {
  loadingPatch: Partial<AppManagerPerAppUiState>;
  clearPatch: Partial<AppManagerPerAppUiState>;
  run: (app: ManagedApp) => Promise<T>;
  onSuccess: (app: ManagedApp, result: T) => Promise<void> | void;
  onError: (app: ManagedApp, message: string) => void;
}

export function useAppManagerActions(options: UseAppManagerActionsOptions) {
  const {
    selectedApp,
    getCurrentKeyword,
    getAppUiState,
    hasApp,
    loadListFirstPage,
    loadDetailCore,
    loadDetailHeavy,
    setAppUiStatePatch,
    updateAppUiState,
  } = options;

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
        setAppUiStatePatch(app.id, {
          cleanupError: "app_manager_cleanup_selection_required",
        });
        return;
      }

      setAppUiStatePatch(app.id, {
        cleanupLoading: true,
        cleanupError: null,
      });

      try {
        const result = await appManagerCleanup({
          appId: app.id,
          selectedItemIds: payload.selectedItemIds,
          includeMainApp,
          deleteMode: payload.deleteMode,
          skipOnError: true,
          confirmedFingerprint: includeMainApp ? app.fingerprint : undefined,
        });

        updateAppUiState(app.id, (state) => ({
          ...state,
          cleanupResult: result,
          cleanupError: null,
          selectedResidueIds: [],
          selectionTouchedByUser: false,
        }));

        await loadListFirstPage(getCurrentKeyword());
        if (hasApp(app.id)) {
          await Promise.all([loadDetailCore(app.id, true), loadDetailHeavy(app.id, true)]);
        }
      } catch (error) {
        setAppUiStatePatch(app.id, {
          cleanupError: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setAppUiStatePatch(app.id, {
          cleanupLoading: false,
        });
      }
    },
    [getCurrentKeyword, hasApp, loadDetailCore, loadDetailHeavy, loadListFirstPage, setAppUiStatePatch, updateAppUiState],
  );

  const cleanupNow = useCallback(async () => {
    if (!selectedApp) {
      return;
    }

    const state = getAppUiState(selectedApp.id);
    await runCleanup(selectedApp, {
      selectedItemIds: state.selectedResidueIds,
      includeMainApp: state.includeMain,
      deleteMode: state.deleteMode,
    });
  }, [getAppUiState, runCleanup, selectedApp]);

  const retryFailed = useCallback(async () => {
    if (!selectedApp) {
      return;
    }

    const state = getAppUiState(selectedApp.id);
    const cleanupResult = state.cleanupResult;
    if (!cleanupResult) {
      return;
    }

    const retryMainApp = cleanupResult.failed.some((item) => item.itemId === "main-app");
    const retryIds = cleanupResult.failed.map((item) => item.itemId).filter((itemId) => itemId !== "main-app");
    if (!retryMainApp && retryIds.length === 0) {
      return;
    }

    const dedupedRetryIds = [...new Set(retryIds)];
    updateAppUiState(selectedApp.id, (previous) => ({
      ...previous,
      selectedResidueIds: dedupedRetryIds,
      includeMain: retryMainApp,
    }));

    await runCleanup(selectedApp, {
      selectedItemIds: dedupedRetryIds,
      includeMainApp: retryMainApp,
      deleteMode: getAppUiState(selectedApp.id).deleteMode,
    });
  }, [getAppUiState, runCleanup, selectedApp, updateAppUiState]);

  const runSelectedAppAction = useCallback(
    async function runSelectedAppActionInternal<T>(config: SelectedAppActionConfig<T>): Promise<void> {
      if (!selectedApp) {
        return;
      }

      const appId = selectedApp.id;
      setAppUiStatePatch(appId, {
        ...config.loadingPatch,
        ...config.clearPatch,
      });

      try {
        const result = await config.run(selectedApp);
        await config.onSuccess(selectedApp, result);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        config.onError(selectedApp, message);
      } finally {
        const loadingResetPatch = Object.fromEntries(
          Object.keys(config.loadingPatch).map((key) => [key, false]),
        ) as Partial<AppManagerPerAppUiState>;
        setAppUiStatePatch(appId, loadingResetPatch);
      }
    },
    [selectedApp, setAppUiStatePatch],
  );

  const toggleStartup = useCallback(async () => {
    await runSelectedAppAction({
      loadingPatch: { startupLoading: true },
      clearPatch: { actionError: null },
      run: (app) =>
        appManagerSetStartup({
          appId: app.id,
          enabled: !app.startupEnabled,
        }),
      onSuccess: async (app, result) => {
        setAppUiStatePatch(app.id, { actionResult: result, actionError: null });
        await loadListFirstPage(getCurrentKeyword());
        if (hasApp(app.id)) {
          await loadDetailCore(app.id, true);
        }
      },
      onError: (app, message) => {
        setAppUiStatePatch(app.id, { actionError: message });
      },
    });
  }, [getCurrentKeyword, hasApp, loadDetailCore, loadListFirstPage, runSelectedAppAction, setAppUiStatePatch]);

  const runUninstall = useCallback(async () => {
    await runSelectedAppAction({
      loadingPatch: { uninstallLoading: true },
      clearPatch: { actionError: null },
      run: (app) =>
        appManagerUninstall({
          appId: app.id,
          confirmedFingerprint: app.fingerprint,
        }),
      onSuccess: async (app, result) => {
        setAppUiStatePatch(app.id, { actionResult: result, actionError: null });
        await loadListFirstPage(getCurrentKeyword());
      },
      onError: (app, message) => {
        setAppUiStatePatch(app.id, { actionError: message });
      },
    });
  }, [getCurrentKeyword, loadListFirstPage, runSelectedAppAction, setAppUiStatePatch]);

  const openUninstallHelp = useCallback(async () => {
    await runSelectedAppAction({
      loadingPatch: { openHelpLoading: true },
      clearPatch: { actionError: null },
      run: (app) => appManagerOpenUninstallHelp(app.id),
      onSuccess: (app, result) => {
        setAppUiStatePatch(app.id, { actionResult: result, actionError: null });
      },
      onError: (app, message) => {
        setAppUiStatePatch(app.id, { actionError: message });
      },
    });
  }, [runSelectedAppAction, setAppUiStatePatch]);

  const openPermissionHelp = useCallback(async () => {
    await runSelectedAppAction({
      loadingPatch: { openPermissionHelpLoading: true },
      clearPatch: { actionError: null },
      run: (app) => appManagerOpenPermissionHelp(app.id),
      onSuccess: (app, result) => {
        setAppUiStatePatch(app.id, { actionResult: result, actionError: null });
      },
      onError: (app, message) => {
        setAppUiStatePatch(app.id, { actionError: message });
      },
    });
  }, [runSelectedAppAction, setAppUiStatePatch]);

  const exportScanResult = useCallback(async () => {
    await runSelectedAppAction({
      loadingPatch: { exportLoading: true },
      clearPatch: { exportError: null },
      run: (app) => appManagerExportScanResult(app.id),
      onSuccess: (app, result: AppManagerExportScanResult) => {
        setAppUiStatePatch(app.id, {
          exportResult: result,
          exportError: null,
        });
      },
      onError: (app, message) => {
        setAppUiStatePatch(app.id, { exportError: message });
      },
    });
  }, [runSelectedAppAction, setAppUiStatePatch]);

  const openExportDirectory = useCallback(async () => {
    if (!selectedApp) {
      return;
    }

    const state = getAppUiState(selectedApp.id);
    if (!state.exportResult) {
      setAppUiStatePatch(selectedApp.id, {
        exportError: "app_manager_export_missing_result",
      });
      return;
    }

    setAppUiStatePatch(selectedApp.id, {
      openExportDirLoading: true,
      exportError: null,
    });

    try {
      await appManagerOpenDirectory(state.exportResult.directoryPath);
    } catch (error) {
      setAppUiStatePatch(selectedApp.id, {
        exportError: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setAppUiStatePatch(selectedApp.id, {
        openExportDirLoading: false,
      });
    }
  }, [getAppUiState, selectedApp, setAppUiStatePatch]);

  const revealPath = useCallback(async (path: string) => {
    await appManagerRevealPath(path);
  }, []);

  const scanAgain = useCallback(async () => {
    if (!selectedApp) {
      return;
    }
    await loadDetailHeavy(selectedApp.id, true);
  }, [loadDetailHeavy, selectedApp]);

  return {
    cleanupNow,
    retryFailed,
    toggleStartup,
    runUninstall,
    openUninstallHelp,
    openPermissionHelp,
    exportScanResult,
    openExportDirectory,
    revealPath,
    scanAgain,
  };
}

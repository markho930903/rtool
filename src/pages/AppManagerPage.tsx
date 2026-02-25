import { memo, useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  AppManagerIconKind,
  AppManagerCleanupItemResult,
  AppManagerPathType,
  AppManagerQueryCategory,
  AppManagerResidueGroup,
  AppManagerResidueItem,
  AppManagerScanWarning,
  AppReadonlyReasonCode,
  ManagedApp,
} from "@/components/app-manager/types";
import { AppEntityIcon } from "@/components/icons/AppEntityIcon";
import { resolvePathIcon } from "@/components/icons/pathIcon";
import { LoadingIndicator } from "@/components/loading";
import { Button, Dialog, Input, RadioGroup, Select, SwitchField, Tooltip } from "@/components/ui";
import { appManagerRevealPath } from "@/services/app-manager.service";
import { useAppManagerStore } from "@/stores/app-manager.store";

function formatIndexedAt(timestamp: number | null, fallback: string): string {
  if (!timestamp || !Number.isFinite(timestamp) || timestamp <= 0) {
    return fallback;
  }
  return new Date(timestamp * 1000).toLocaleString();
}

function formatBytes(value?: number | null): string {
  if (!value || !Number.isFinite(value) || value <= 0) {
    return "-";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = value;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  const fractionDigits = unitIndex === 0 ? 0 : size >= 100 ? 0 : size >= 10 ? 1 : 2;
  return `${size.toFixed(fractionDigits)} ${units[unitIndex]}`;
}

function warningMessageKey(code: AppManagerScanWarning["code"]): string {
  return `cleanup.warning.${code}`;
}

function warningDetailMessageKey(code: NonNullable<AppManagerScanWarning["detailCode"]>): string {
  return `cleanup.warningDetail.${code}`;
}

function cleanupResultReasonMessageKey(code: AppManagerCleanupItemResult["reasonCode"]): string {
  return `result.reason.${code}`;
}

interface RelatedLocationEntry {
  id: string;
  path: string;
  name: string;
  sizeBytes?: number | null;
  pathType?: AppManagerPathType;
  readonlyReasonCode?: AppReadonlyReasonCode;
  source: "main" | "scan";
}

function normalizePathKey(path: string): string {
  return path
    .trim()
    .replace(/[\\/]+/g, "/")
    .toLowerCase();
}

function isRegistryResiduePath(path: string): boolean {
  return path.includes("::");
}

function isPathInsideOwnedRoots(path: string, rootKeys: string[]): boolean {
  const pathKey = normalizePathKey(path);
  if (!pathKey) {
    return false;
  }
  return rootKeys.some((rootKey) => {
    if (!rootKey) {
      return false;
    }
    return pathKey === rootKey || pathKey.startsWith(`${rootKey}/`);
  });
}

function getPathName(path: string): string {
  const normalized = path.trim().replace(/[\\/]+/g, "/");
  if (!normalized) {
    return path;
  }
  const segments = normalized.split("/").filter(Boolean);
  return segments[segments.length - 1] ?? normalized;
}

function isAppBundlePath(path: string): boolean {
  return normalizePathKey(path).replace(/\/+$/, "").endsWith(".app");
}

const APP_MANAGER_CATEGORY_VALUES: AppManagerQueryCategory[] = ["all", "application", "startup", "rtool"];

function isAppManagerQueryCategory(value: string): value is AppManagerQueryCategory {
  return APP_MANAGER_CATEGORY_VALUES.includes(value as AppManagerQueryCategory);
}

function resolveRelatedEntryIcon(
  entry: RelatedLocationEntry,
  selectedApp: ManagedApp | null,
): {
  iconKind?: AppManagerIconKind;
  iconValue?: string;
  fallbackIcon: string;
} {
  if (entry.source === "main" || isAppBundlePath(entry.path)) {
    return {
      iconKind: selectedApp?.iconKind,
      iconValue: selectedApp?.iconValue,
      fallbackIcon: "i-noto:desktop-computer",
    };
  }

  return {
    fallbackIcon: resolvePathIcon(entry.path, entry.pathType),
  };
}

function AppIcon({
  app,
  sizeClassName = "h-8 w-8",
  iconSizeClassName = "text-[1.05rem]",
}: {
  app: ManagedApp;
  sizeClassName?: string;
  iconSizeClassName?: string;
}) {
  return (
    <AppEntityIcon
      iconKind={app.iconKind}
      iconValue={app.iconValue}
      fallbackIcon="i-noto:desktop-computer"
      imgClassName={`${sizeClassName} shrink-0 rounded-md object-cover`}
      iconClassName={`${sizeClassName} shrink-0 ${iconSizeClassName} text-text-secondary`}
    />
  );
}

const AppListItem = memo(function AppListItem({
  app,
  selected,
  actionLoading,
  startupScopeLabel,
  deepUninstallTitle,
  onSelect,
  onDeepUninstall,
}: {
  app: ManagedApp;
  selected: boolean;
  actionLoading: boolean;
  startupScopeLabel: string;
  deepUninstallTitle: string;
  onSelect: (appId: string) => void;
  onDeepUninstall: (app: ManagedApp) => void;
}) {
  return (
    <button
      type="button"
      className={`w-full rounded-lg border px-3 py-2 text-left transition-colors ${
        selected
          ? "border-accent/70 bg-accent/10"
          : "border-border-glass bg-surface-glass-soft shadow-inset-soft hover:border-accent/45"
      }`}
      onClick={() => onSelect(app.id)}
    >
      <div className="flex items-start gap-2">
        <AppIcon app={app} sizeClassName="h-9 w-9" iconSizeClassName="text-[1.15rem]" />
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-medium text-text-primary">{app.name}</div>
          <div className="mt-0.5 truncate text-xs text-text-muted">{app.path}</div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-text-secondary">
            <span>{formatBytes(app.sizeBytes)}</span>
            <span>{startupScopeLabel}</span>
          </div>
        </div>
        <Button
          size="xs"
          variant="ghost"
          iconOnly
          disabled={actionLoading}
          title={deepUninstallTitle}
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onDeepUninstall(app);
          }}
        >
          <span className="btn-icon i-noto:wastebasket text-base" aria-hidden="true" />
        </Button>
      </div>
    </button>
  );
});

function ResultRows({
  title,
  rows,
  kindClassName,
}: {
  title: string;
  rows: AppManagerCleanupItemResult[];
  kindClassName: string;
}) {
  const { t } = useTranslation("app_manager");
  if (rows.length === 0) {
    return null;
  }
  return (
    <div className="space-y-1.5">
      <h4 className="m-0 text-xs font-semibold text-text-secondary">{title}</h4>
      <div className="space-y-1.5">
        {rows.slice(0, 20).map((row) => (
          <div
            key={`${title}-${row.itemId}-${row.path}`}
            className={`rounded-md border px-2 py-1.5 text-xs ${kindClassName}`}
          >
            <div className="break-all">{row.path}</div>
            <div className="mt-0.5 text-[11px] opacity-80">
              {t(cleanupResultReasonMessageKey(row.reasonCode), {
                defaultValue: t("result.reason.unknown", { defaultValue: row.reasonCode }),
              })}{" "}
              · {row.message}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default function AppManagerPage() {
  const { t } = useTranslation("app_manager");
  const items = useAppManagerStore((state) => state.items);
  const loading = useAppManagerStore((state) => state.loading);
  const loadingMore = useAppManagerStore((state) => state.loadingMore);
  const refreshing = useAppManagerStore((state) => state.refreshing);
  const actionLoadingById = useAppManagerStore((state) => state.actionLoadingById);
  const detailLoadingById = useAppManagerStore((state) => state.detailLoadingById);
  const scanLoadingById = useAppManagerStore((state) => state.scanLoadingById);
  const cleanupLoadingById = useAppManagerStore((state) => state.cleanupLoadingById);
  const exportLoadingById = useAppManagerStore((state) => state.exportLoadingById);
  const openExportDirLoadingById = useAppManagerStore((state) => state.openExportDirLoadingById);
  const keyword = useAppManagerStore((state) => state.keyword);
  const category = useAppManagerStore((state) => state.category);
  const nextCursor = useAppManagerStore((state) => state.nextCursor);
  const indexedAt = useAppManagerStore((state) => state.indexedAt);
  const revision = useAppManagerStore((state) => state.revision);
  const indexState = useAppManagerStore((state) => state.indexState);
  const appManagerError = useAppManagerStore((state) => state.error);
  const detailError = useAppManagerStore((state) => state.detailError);
  const scanError = useAppManagerStore((state) => state.scanError);
  const cleanupError = useAppManagerStore((state) => state.cleanupError);
  const exportError = useAppManagerStore((state) => state.exportError);
  const lastActionResult = useAppManagerStore((state) => state.lastActionResult);
  const selectedAppId = useAppManagerStore((state) => state.selectedAppId);
  const detailById = useAppManagerStore((state) => state.detailById);
  const scanResultById = useAppManagerStore((state) => state.scanResultById);
  const cleanupResultById = useAppManagerStore((state) => state.cleanupResultById);
  const exportResultById = useAppManagerStore((state) => state.exportResultById);
  const selectedResidueIdsByAppId = useAppManagerStore((state) => state.selectedResidueIdsByAppId);
  const deleteModeByAppId = useAppManagerStore((state) => state.deleteModeByAppId);
  const includeMainAppByAppId = useAppManagerStore((state) => state.includeMainAppByAppId);
  const experimentalThirdPartyStartup = useAppManagerStore((state) => state.experimentalThirdPartyStartup);

  const setKeyword = useAppManagerStore((state) => state.setKeyword);
  const setCategory = useAppManagerStore((state) => state.setCategory);
  const setExperimentalThirdPartyStartup = useAppManagerStore((state) => state.setExperimentalThirdPartyStartup);
  const clearLastActionResult = useAppManagerStore((state) => state.clearLastActionResult);
  const selectApp = useAppManagerStore((state) => state.selectApp);
  const ensureSnapshotLoaded = useAppManagerStore((state) => state.ensureSnapshotLoaded);
  const loadMore = useAppManagerStore((state) => state.loadMore);
  const refreshIndex = useAppManagerStore((state) => state.refreshIndex);
  const loadDetail = useAppManagerStore((state) => state.loadDetail);
  const scanResidue = useAppManagerStore((state) => state.scanResidue);
  const toggleResidueItem = useAppManagerStore((state) => state.toggleResidueItem);
  const selectRecommendedResidues = useAppManagerStore((state) => state.selectRecommendedResidues);
  const clearResidueSelection = useAppManagerStore((state) => state.clearResidueSelection);
  const setDeleteMode = useAppManagerStore((state) => state.setDeleteMode);
  const setIncludeMainApp = useAppManagerStore((state) => state.setIncludeMainApp);
  const exportScanResultAction = useAppManagerStore((state) => state.exportScanResult);
  const openExportDirectory = useAppManagerStore((state) => state.openExportDirectory);
  const cleanupSelected = useAppManagerStore((state) => state.cleanupSelected);
  const retryFailedCleanup = useAppManagerStore((state) => state.retryFailedCleanup);
  const deepUninstall = useAppManagerStore((state) => state.deepUninstall);
  const toggleStartup = useAppManagerStore((state) => state.toggleStartup);
  const openUninstallHelp = useAppManagerStore((state) => state.openUninstallHelp);

  const [confirmTarget, setConfirmTarget] = useState<ManagedApp | null>(null);
  const [confirmingDeepUninstall, setConfirmingDeepUninstall] = useState(false);
  const [copyPathFeedback, setCopyPathFeedback] = useState<string | null>(null);
  const [highlightExportPath, setHighlightExportPath] = useState(false);
  const [revealingRelatedId, setRevealingRelatedId] = useState<string | null>(null);
  const [relatedRevealError, setRelatedRevealError] = useState<string | null>(null);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void ensureSnapshotLoaded();
    }, 150);
    return () => {
      window.clearTimeout(timer);
    };
  }, [ensureSnapshotLoaded]);

  useEffect(() => {
    setCopyPathFeedback(null);
    setHighlightExportPath(false);
    setRevealingRelatedId(null);
    setRelatedRevealError(null);
  }, [selectedAppId]);

  useEffect(() => {
    if (!highlightExportPath) {
      return;
    }
    const timer = window.setTimeout(() => {
      setHighlightExportPath(false);
    }, 1800);
    return () => {
      window.clearTimeout(timer);
    };
  }, [highlightExportPath]);

  const selectedApp = useMemo(() => items.find((item) => item.id === selectedAppId) ?? null, [items, selectedAppId]);
  const selectedDetail = selectedApp ? detailById[selectedApp.id] : undefined;
  const selectedScanResult = selectedApp ? scanResultById[selectedApp.id] : undefined;
  const selectedCleanupResult = selectedApp ? cleanupResultById[selectedApp.id] : undefined;
  const selectedExportResult = selectedApp ? exportResultById[selectedApp.id] : undefined;
  const selectedResidueIds = selectedApp ? (selectedResidueIdsByAppId[selectedApp.id] ?? []) : [];
  const selectedDeleteMode = selectedApp ? (deleteModeByAppId[selectedApp.id] ?? "trash") : "trash";
  const selectedIncludeMainApp = selectedApp ? (includeMainAppByAppId[selectedApp.id] ?? true) : true;
  const selectedDetailLoading = selectedApp ? Boolean(detailLoadingById[selectedApp.id]) : false;
  const selectedScanLoading = selectedApp ? Boolean(scanLoadingById[selectedApp.id]) : false;

  const categoryOptions = useMemo(
    () => [
      { value: "all", label: t("filters.category.all") },
      { value: "application", label: t("filters.category.application") },
      { value: "startup", label: t("filters.category.startup") },
      { value: "rtool", label: t("filters.category.rtool") },
    ],
    [t],
  );

  const deleteModeOptions = useMemo(
    () => [
      { value: "trash", label: t("cleanup.deleteModeTrash") },
      { value: "permanent", label: t("cleanup.deleteModePermanent") },
    ],
    [t],
  );

  const indexedAtText = formatIndexedAt(indexedAt, t("meta.indexedAtUnknown"));

  const resolveStartupReadonlyReason = (app: ManagedApp | null): string | null => {
    if (!app) {
      return null;
    }
    const thirdParty = app.source !== "rtool";
    if (thirdParty && !experimentalThirdPartyStartup) {
      return t("readonly.experimentalDisabled");
    }
    if (app.readonlyReasonCode === "managed_by_policy") {
      return t("readonly.systemPolicyManaged");
    }
    if (app.readonlyReasonCode === "permission_denied") {
      return t("readonly.permissionRequired");
    }
    if (!app.startupEditable) {
      return t("readonly.unknown");
    }
    return null;
  };

  const startupReadonlyReason = resolveStartupReadonlyReason(selectedApp);

  const relatedLocations = useMemo(() => {
    if (!selectedApp) {
      return [] as RelatedLocationEntry[];
    }

    const ownedRootKeys = (() => {
      const keys = new Set<string>();
      const installPath = selectedDetail?.installPath ?? selectedApp.path;
      const installKey = normalizePathKey(installPath);
      if (installKey) {
        keys.add(installKey);
      }
      selectedDetail?.relatedRoots.forEach((root) => {
        const key = normalizePathKey(root.path);
        if (key) {
          keys.add(key);
        }
      });
      return [...keys];
    })();

    const map = new Map<string, RelatedLocationEntry>();
    const upsert = (entry: RelatedLocationEntry) => {
      const key = normalizePathKey(entry.path);
      if (!key) {
        return;
      }
      const existing = map.get(key);
      if (!existing) {
        map.set(key, entry);
        return;
      }
      if (existing.source === "main") {
        return;
      }
      const existingSize = existing.sizeBytes ?? -1;
      const nextSize = entry.sizeBytes ?? -1;
      if (nextSize >= existingSize) {
        map.set(key, entry);
      }
    };

    upsert({
      id: `main-${selectedApp.id}`,
      path: selectedDetail?.installPath ?? selectedApp.path,
      name: getPathName(selectedDetail?.installPath ?? selectedApp.path),
      sizeBytes: selectedDetail?.sizeSummary.appBytes ?? selectedApp.sizeBytes,
      pathType: "directory",
      source: "main",
    });

    selectedScanResult?.groups.forEach((group) => {
      group.items.forEach((item) => {
        if (isRegistryResiduePath(item.path)) {
          return;
        }
        if (!isPathInsideOwnedRoots(item.path, ownedRootKeys)) {
          return;
        }
        upsert({
          id: item.itemId,
          path: item.path,
          name: getPathName(item.path),
          sizeBytes: item.sizeBytes,
          pathType: item.pathType,
          readonlyReasonCode: item.readonlyReasonCode,
          source: "scan",
        });
      });
    });

    return [...map.values()].sort((left, right) => {
      if (left.source !== right.source) {
        return left.source === "main" ? -1 : 1;
      }
      const sizeDiff = (right.sizeBytes ?? -1) - (left.sizeBytes ?? -1);
      if (sizeDiff !== 0) {
        return sizeDiff;
      }
      return left.path.localeCompare(right.path);
    });
  }, [selectedApp, selectedDetail, selectedScanResult]);

  useEffect(() => {
    if (!selectedApp || selectedScanResult || selectedScanLoading) {
      return;
    }
    void scanResidue(selectedApp.id);
  }, [selectedApp, selectedScanResult, selectedScanLoading, scanResidue]);

  const onConfirmDeepUninstall = async () => {
    if (!confirmTarget) {
      return;
    }
    setConfirmingDeepUninstall(true);
    try {
      await deepUninstall(confirmTarget.id);
      setConfirmTarget(null);
    } finally {
      setConfirmingDeepUninstall(false);
    }
  };

  const handleSelectListItem = useCallback(
    (appId: string) => {
      void selectApp(appId);
    },
    [selectApp],
  );

  const handleOpenDeepUninstallDialog = useCallback((app: ManagedApp) => {
    setConfirmTarget(app);
  }, []);

  const exportScanResult = async () => {
    if (!selectedApp) {
      return;
    }
    setCopyPathFeedback(null);
    await exportScanResultAction(selectedApp.id);
  };

  const copyExportPath = async () => {
    if (!selectedExportResult?.filePath) {
      return;
    }
    try {
      await navigator.clipboard.writeText(selectedExportResult.filePath);
      setCopyPathFeedback(t("cleanup.copyPathSuccess"));
      setHighlightExportPath(true);
    } catch {
      setCopyPathFeedback(t("cleanup.copyPathFailed"));
      setHighlightExportPath(false);
    }
  };

  const revealRelatedPath = async (entry: RelatedLocationEntry) => {
    if (!entry.path || revealingRelatedId) {
      return;
    }
    setRelatedRevealError(null);
    setRevealingRelatedId(entry.id);
    try {
      await appManagerRevealPath(entry.path);
    } catch (caughtError) {
      const message = caughtError instanceof Error ? caughtError.message : String(caughtError);
      setRelatedRevealError(t("detail.revealFailed", { message }));
    } finally {
      setRevealingRelatedId(null);
    }
  };

  const isRelatedEntrySelected = (entry: RelatedLocationEntry): boolean => {
    if (entry.source === "main") {
      return selectedIncludeMainApp;
    }
    return selectedResidueIds.includes(entry.id);
  };

  const isRelatedEntrySelectionDisabled = (entry: RelatedLocationEntry): boolean => {
    if (entry.source === "main") {
      return false;
    }
    return entry.readonlyReasonCode === "managed_by_policy";
  };

  const toggleRelatedEntrySelection = (entry: RelatedLocationEntry) => {
    if (!selectedApp) {
      return;
    }
    if (entry.source === "main") {
      setIncludeMainApp(selectedApp.id, !selectedIncludeMainApp);
      return;
    }
    const checked = selectedResidueIds.includes(entry.id);
    toggleResidueItem(selectedApp.id, entry.id, !checked);
  };

  const isStartupActionDisabled = (app: ManagedApp): boolean => {
    const actionLoading = Boolean(actionLoadingById[app.id]);
    const thirdParty = app.source !== "rtool";
    const startupDisabledByExperiment = thirdParty && !experimentalThirdPartyStartup;
    return actionLoading || !app.capabilities.startup || !app.startupEditable || startupDisabledByExperiment;
  };

  return (
    <section className="h-full min-h-0 p-5">
      <div className="grid h-full min-h-0 gap-4 md:grid-cols-[360px_minmax(0,1fr)]">
        <aside className="ui-glass-panel flex h-full min-h-0 flex-col">
          <div className="shrink-0 space-y-2 border-b border-border-glass px-3 py-2.5">
            <div className="flex items-start justify-between gap-2">
              <div className="space-y-0.5">
                <h1 className="m-0 text-base font-semibold text-text-primary">{t("title")}</h1>
                <p className="m-0 text-xs text-text-secondary">{t("desc")}</p>
              </div>
              <Button
                size="xs"
                variant="secondary"
                disabled={refreshing || loading}
                onClick={() => void refreshIndex()}
              >
                {refreshing ? t("actions.refreshing") : t("actions.refresh")}
              </Button>
            </div>

            <Input
              value={keyword}
              placeholder={t("filters.keywordPlaceholder")}
              onChange={(event) => setKeyword(event.currentTarget.value)}
            />
            <RadioGroup
              name="app-manager-category"
              options={categoryOptions}
              value={category}
              orientation="horizontal"
              size="sm"
              className="gap-3"
              optionClassName="items-center text-xs text-text-primary"
              onValueChange={(value) => {
                if (isAppManagerQueryCategory(value)) {
                  setCategory(value);
                }
              }}
            />
            <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-2.5 py-1.5 shadow-inset-soft">
              <SwitchField
                checked={experimentalThirdPartyStartup}
                controlPosition="end"
                onChange={(event) => setExperimentalThirdPartyStartup(event.currentTarget.checked)}
                label={
                  <span className="inline-flex items-center gap-1 text-xs text-text-primary">
                    <span>{t("experimental.title")}</span>
                    <Tooltip
                      content={<span className="leading-5">{t("experimental.desc")}</span>}
                      ariaLabel={t("experimental.desc")}
                      triggerClassName="rounded-sm text-text-muted hover:text-text-secondary"
                    >
                      <span className="i-noto:light-bulb text-sm" aria-hidden="true" />
                    </Tooltip>
                  </span>
                }
              />
            </div>

            <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-text-muted">
              <span>{t("meta.indexedAt", { value: indexedAtText })}</span>
              <span>{t("meta.count", { count: items.length })}</span>
              <span>{`rev ${revision}`}</span>
            </div>
            {indexState === "degraded" ? (
              <div className="rounded-md border border-warning/35 bg-warning/10 px-2.5 py-2 text-xs text-warning">
                {t("status.indexDegraded", { defaultValue: "索引已降级，当前展示为最近一次可用数据" })}
              </div>
            ) : null}

            {appManagerError ? (
              <div className="rounded-md border border-danger/35 bg-danger/10 px-2.5 py-2 text-xs text-danger">
                {appManagerError}
              </div>
            ) : null}
            {lastActionResult ? (
              <div
                className={`rounded-md border px-2.5 py-2 text-xs ${
                  lastActionResult.ok
                    ? "border-border-glass bg-surface-glass-soft text-text-secondary"
                    : "border-danger/35 bg-danger/10 text-danger"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="break-all">{lastActionResult.message}</span>
                  <Button size="xs" variant="ghost" onClick={() => clearLastActionResult()}>
                    {t("actions.dismiss")}
                  </Button>
                </div>
              </div>
            ) : null}
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
            <div className="mb-2 flex items-center justify-between">
              <h2 className="m-0 text-sm font-semibold text-text-primary">{t("list.title")}</h2>
            </div>
            <LoadingIndicator
              mode="overlay"
              loading={loading && items.length === 0}
              text={t("status.loading")}
              containerClassName="min-h-24"
            >
              <>
                {!loading && items.length === 0 ? (
                  <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-6 text-center text-sm text-text-muted shadow-inset-soft">
                    {t("status.empty")}
                  </div>
                ) : null}
                <div className="space-y-2">
                  {items.map((app) => {
                    const startupScopeLabel = t(`meta.startupScope.${app.startupScope}`, {
                      defaultValue: app.startupScope,
                    });
                    return (
                      <AppListItem
                        key={app.id}
                        app={app}
                        selected={app.id === selectedAppId}
                        actionLoading={Boolean(actionLoadingById[app.id])}
                        startupScopeLabel={startupScopeLabel}
                        deepUninstallTitle={t("actions.deepUninstall")}
                        onSelect={handleSelectListItem}
                        onDeepUninstall={handleOpenDeepUninstallDialog}
                      />
                    );
                  })}
                </div>

                {!loading && nextCursor ? (
                  <div className="mt-3 flex justify-center">
                    <Button size="default" variant="secondary" disabled={loadingMore} onClick={() => void loadMore()}>
                      {loadingMore ? t("actions.loadingMore") : t("actions.loadMore")}
                    </Button>
                  </div>
                ) : null}
              </>
            </LoadingIndicator>
          </div>
        </aside>

        <div className="h-full min-h-0 overflow-y-auto pr-1">
          <div className="space-y-3 pb-2">
            {!selectedApp ? (
              <div className="ui-glass-panel px-4 py-8 text-center text-sm text-text-muted">{t("detail.empty")}</div>
            ) : (
              <>
                <section className="ui-glass-panel px-4 py-4">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <AppIcon app={selectedApp} />
                        <h2 className="m-0 text-base font-semibold text-text-primary">{selectedApp.name}</h2>
                      </div>
                      <p className="m-0 break-all text-xs text-text-muted">
                        {selectedDetail?.installPath ?? selectedApp.path}
                      </p>
                      <div className="flex flex-wrap items-center gap-2 text-xs text-text-secondary">
                        <span>{t("meta.platform", { value: selectedApp.platform })}</span>
                        {selectedApp.version ? <span>{t("meta.version", { value: selectedApp.version })}</span> : null}
                        {selectedApp.publisher ? (
                          <span>{t("meta.publisher", { value: selectedApp.publisher })}</span>
                        ) : null}
                        {selectedApp.bundleOrAppId ? (
                          <span>{t("meta.bundleId", { value: selectedApp.bundleOrAppId })}</span>
                        ) : null}
                        <span>{t("meta.identity", { value: selectedApp.identity.primaryId })}</span>
                        <span>{t("meta.identitySource", { value: selectedApp.identity.identitySource })}</span>
                        <span className="inline-flex items-center gap-1">
                          {selectedDetailLoading ? (
                            <LoadingIndicator ariaLabel={t("detail.loading")} className="text-text-secondary" />
                          ) : null}
                          <span>
                            {t("detail.size", {
                              value: selectedDetailLoading
                                ? t("detail.calculating")
                                : formatBytes(selectedDetail?.sizeSummary.appBytes ?? selectedApp.sizeBytes),
                            })}
                          </span>
                        </span>
                      </div>
                      <div className="flex flex-wrap items-center gap-2 text-[11px] text-text-muted">
                        <span>
                          {selectedApp.capabilities.startup
                            ? t("meta.capability.startupEnabled")
                            : t("meta.capability.startupDisabled")}
                        </span>
                        <span>
                          {selectedApp.capabilities.uninstall
                            ? t("meta.capability.uninstallEnabled")
                            : t("meta.capability.uninstallDisabled")}
                        </span>
                        <span>
                          {selectedApp.capabilities.residueScan
                            ? t("meta.capability.scanEnabled")
                            : t("meta.capability.scanDisabled")}
                        </span>
                      </div>
                    </div>
                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        size="default"
                        variant="ghost"
                        disabled={selectedDetailLoading}
                        onClick={() => void loadDetail(selectedApp.id, true)}
                      >
                        {selectedDetailLoading
                          ? t("detail.calculating")
                          : t("actions.refreshDetail", { defaultValue: "刷新详情" })}
                      </Button>
                      <Button
                        size="default"
                        variant={selectedApp.startupEnabled ? "secondary" : "primary"}
                        disabled={isStartupActionDisabled(selectedApp)}
                        onClick={() => void toggleStartup(selectedApp, !selectedApp.startupEnabled)}
                      >
                        {actionLoadingById[selectedApp.id]
                          ? t("status.processing")
                          : selectedApp.startupEnabled
                            ? t("actions.disableStartup")
                            : t("actions.enableStartup")}
                      </Button>
                      <Button
                        size="default"
                        variant="secondary"
                        disabled={!selectedApp.capabilities.uninstall}
                        onClick={() => void openUninstallHelp(selectedApp)}
                      >
                        {t("actions.uninstallGuide")}
                      </Button>
                      <Button
                        size="default"
                        variant="danger"
                        disabled={!selectedApp.capabilities.uninstall}
                        onClick={() => setConfirmTarget(selectedApp)}
                      >
                        {t("actions.deepUninstall")}
                      </Button>
                    </div>
                  </div>

                  {startupReadonlyReason ? (
                    <div className="mt-3 rounded-md border border-info/45 bg-info/10 px-3 py-2 text-xs text-info">
                      {startupReadonlyReason}
                    </div>
                  ) : null}
                  {detailError ? (
                    <div className="mt-3 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                      {detailError}
                    </div>
                  ) : null}
                  {selectedDetailLoading ? (
                    <LoadingIndicator
                      mode="overlay"
                      text={t("detail.loading")}
                      containerClassName="mt-3 rounded-md border border-border-glass bg-surface-glass-soft shadow-inset-soft"
                      minHeightClassName="min-h-16"
                      showMask={false}
                    />
                  ) : null}
                </section>

                <section className="ui-glass-panel px-4 py-4">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <h3 className="m-0 text-sm font-semibold text-text-primary">{t("detail.relatedRoots")}</h3>
                    <span className="text-xs text-text-secondary">
                      {selectedScanLoading
                        ? t("cleanup.scanning")
                        : t("detail.relatedCount", { count: relatedLocations.length })}
                    </span>
                  </div>
                  <p className="m-0 mt-1 text-[11px] text-text-muted">{t("detail.relatedAutoScanHint")}</p>

                  {relatedRevealError ? (
                    <div className="mt-2 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                      {relatedRevealError}
                    </div>
                  ) : null}

                  <div className="mt-3 rounded-lg border border-border-glass bg-surface-glass-soft shadow-inset-soft">
                    {relatedLocations.length === 0 ? (
                      <p className="m-0 px-3 py-4 text-xs text-text-muted">
                        {selectedScanLoading ? t("cleanup.scanning") : t("detail.noRelatedRoots")}
                      </p>
                    ) : (
                      <div className="divide-y divide-border-muted">
                        {relatedLocations.map((entry) => {
                          const revealing = revealingRelatedId === entry.id;
                          const selected = isRelatedEntrySelected(entry);
                          const selectionDisabled = isRelatedEntrySelectionDisabled(entry);
                          const relatedIcon = resolveRelatedEntryIcon(entry, selectedApp);
                          return (
                            <div
                              key={entry.path}
                              className={`flex w-full items-start gap-2 px-3 py-2.5 text-left transition-colors ${
                                selected ? "bg-accent/8" : "hover:bg-surface-glass-soft"
                              }`}
                            >
                              <button
                                type="button"
                                aria-pressed={selected}
                                disabled={selectionDisabled || Boolean(revealingRelatedId)}
                                className={`mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full border transition-colors ${
                                  selected
                                    ? "border-accent bg-accent text-accent-foreground"
                                    : "border-border-glass bg-surface-glass-soft text-transparent hover:border-accent/55"
                                } ${selectionDisabled ? "cursor-not-allowed opacity-55" : "cursor-pointer"}`}
                                onClick={(event) => {
                                  event.preventDefault();
                                  event.stopPropagation();
                                  toggleRelatedEntrySelection(entry);
                                }}
                              >
                                <svg viewBox="0 0 16 16" className="h-3 w-3" aria-hidden="true">
                                  <path
                                    d="m3.5 8.25 2.5 2.5L12.5 4.5"
                                    fill="none"
                                    stroke="currentColor"
                                    strokeLinecap="round"
                                    strokeLinejoin="round"
                                    strokeWidth="1.6"
                                  />
                                </svg>
                              </button>
                              <button
                                type="button"
                                className="flex min-w-0 flex-1 items-start gap-2 text-left"
                                disabled={Boolean(revealingRelatedId)}
                                title={entry.path}
                                onClick={() => void revealRelatedPath(entry)}
                              >
                                <AppEntityIcon
                                  iconKind={relatedIcon.iconKind}
                                  iconValue={relatedIcon.iconValue}
                                  fallbackIcon={relatedIcon.fallbackIcon}
                                  imgClassName="h-8 w-8 shrink-0 rounded-md object-cover"
                                  iconClassName="h-8 w-8 shrink-0 text-[1rem] text-text-secondary"
                                />
                                <span className="min-w-0 flex-1">
                                  <span className="block truncate text-sm font-medium text-text-primary">
                                    {entry.name}
                                  </span>
                                  <span className="mt-0.5 block break-all text-xs text-text-muted">{entry.path}</span>
                                  {entry.readonlyReasonCode ? (
                                    <span className="mt-0.5 block text-[11px] text-info">
                                      {t(`readonly.${entry.readonlyReasonCode}`, {
                                        defaultValue: entry.readonlyReasonCode,
                                      })}
                                    </span>
                                  ) : null}
                                </span>
                                <span className="shrink-0 pt-0.5 text-sm text-text-primary">
                                  {revealing ? t("detail.revealing") : formatBytes(entry.sizeBytes)}
                                </span>
                              </button>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </section>

                <section className="ui-glass-panel px-4 py-4">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <h3 className="m-0 text-sm font-semibold text-text-primary">{t("cleanup.title")}</h3>
                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        size="default"
                        variant="secondary"
                        disabled={selectedScanLoading}
                        onClick={() => void scanResidue(selectedApp.id)}
                      >
                        {selectedScanLoading ? t("cleanup.scanning") : t("cleanup.scan")}
                      </Button>
                      <Button
                        size="default"
                        variant="secondary"
                        disabled={!selectedScanResult}
                        onClick={() => selectRecommendedResidues(selectedApp.id)}
                      >
                        {t("cleanup.selectRecommended")}
                      </Button>
                      <Button
                        size="default"
                        variant="ghost"
                        disabled={!selectedScanResult}
                        onClick={() => clearResidueSelection(selectedApp.id)}
                      >
                        {t("cleanup.clearSelection")}
                      </Button>
                      <Button
                        size="default"
                        variant="ghost"
                        disabled={!selectedScanResult || Boolean(exportLoadingById[selectedApp.id])}
                        onClick={() => void exportScanResult()}
                      >
                        {exportLoadingById[selectedApp.id] ? t("cleanup.exporting") : t("cleanup.exportScan")}
                      </Button>
                    </div>
                  </div>

                  <div className="mt-3 grid gap-3 md:grid-cols-[220px_auto]">
                    <Select
                      value={selectedDeleteMode}
                      options={deleteModeOptions}
                      onChange={(event) =>
                        setDeleteMode(selectedApp.id, event.currentTarget.value as "trash" | "permanent")
                      }
                    />
                    <SwitchField
                      checked={selectedIncludeMainApp}
                      controlPosition="end"
                      onChange={(event) => setIncludeMainApp(selectedApp.id, event.currentTarget.checked)}
                      label={<span className="text-sm text-text-primary">{t("cleanup.includeMainApp")}</span>}
                      description={<span className="leading-5">{t("cleanup.includeMainAppDesc")}</span>}
                    />
                  </div>

                  {scanError ? (
                    <div className="mt-3 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                      {scanError}
                    </div>
                  ) : null}
                  {cleanupError ? (
                    <div className="mt-3 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                      {cleanupError}
                    </div>
                  ) : null}
                  {exportError ? (
                    <div className="mt-3 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                      {exportError}
                    </div>
                  ) : null}

                  {selectedScanResult ? (
                    <div className="mt-3 space-y-3">
                      <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-text-secondary">
                        <span>
                          {t("cleanup.scanSummary", {
                            count: selectedScanResult.groups.flatMap((group) => group.items).length,
                          })}
                        </span>
                        <span>{t("cleanup.totalSize", { value: formatBytes(selectedScanResult.totalSizeBytes) })}</span>
                      </div>

                      {selectedScanResult.warnings.length > 0 ? (
                        <div className="space-y-1 rounded-md border border-info/45 bg-info/10 px-3 py-2 text-xs text-info">
                          {selectedScanResult.warnings.map((warning, index) => (
                            <div key={`${warning.code}-${index}`} className="space-y-0.5">
                              <div>
                                {t(warningMessageKey(warning.code), {
                                  path: warning.path ?? "-",
                                  defaultValue: t("cleanup.warning.unknown", {
                                    path: warning.path ?? "-",
                                  }),
                                })}
                              </div>
                              {warning.detailCode ? (
                                <div className="opacity-80">
                                  {t(warningDetailMessageKey(warning.detailCode), {
                                    defaultValue: t("cleanup.warningDetail.unknown"),
                                  })}
                                </div>
                              ) : null}
                            </div>
                          ))}
                        </div>
                      ) : null}

                      {selectedExportResult ? (
                        <div
                          className={`rounded-md border px-3 py-2 text-xs transition-colors ${
                            highlightExportPath
                              ? "border-accent/60 bg-accent/10 text-text-primary"
                              : "border-border-glass bg-surface-glass-soft text-text-secondary"
                          }`}
                        >
                          <p className="m-0 break-all">
                            {t("cleanup.exportedPath", { value: selectedExportResult.filePath })}
                          </p>
                          <div className="mt-2 flex flex-wrap items-center gap-2">
                            <Button
                              size="xs"
                              variant="secondary"
                              disabled={Boolean(openExportDirLoadingById[selectedApp.id])}
                              onClick={() => void openExportDirectory(selectedApp.id)}
                            >
                              {openExportDirLoadingById[selectedApp.id]
                                ? t("cleanup.openingDir")
                                : t("cleanup.openExportDir")}
                            </Button>
                            <Button size="xs" variant="secondary" onClick={() => void copyExportPath()}>
                              {t("cleanup.copyExportPath")}
                            </Button>
                            {copyPathFeedback ? (
                              <span className="text-[11px] text-text-secondary">{copyPathFeedback}</span>
                            ) : null}
                          </div>
                        </div>
                      ) : null}

                      <div className="space-y-2">
                        {selectedScanResult.groups.map((group: AppManagerResidueGroup) => (
                          <div
                            key={group.groupId}
                            className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-2 shadow-inset-soft"
                          >
                            <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                              <h4 className="m-0 text-xs font-semibold text-text-primary">{group.label}</h4>
                              <span className="text-[11px] text-text-secondary">
                                {formatBytes(group.totalSizeBytes)}
                              </span>
                            </div>
                            <div className="space-y-2">
                              {group.items.map((item: AppManagerResidueItem) => {
                                const checked = selectedResidueIds.includes(item.itemId);
                                const disabled = item.readonly && item.readonlyReasonCode === "managed_by_policy";
                                return (
                                  <button
                                    key={item.itemId}
                                    type="button"
                                    disabled={disabled}
                                    aria-pressed={checked}
                                    className={`w-full rounded-md border px-2 py-1.5 text-left transition-colors ${
                                      checked
                                        ? "border-accent/55 bg-accent/10"
                                        : "border-border-glass bg-surface-glass-soft hover:border-accent/35 hover:bg-surface-glass"
                                    } ${disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer"}`}
                                    onClick={() => toggleResidueItem(selectedApp.id, item.itemId, !checked)}
                                  >
                                    <div className="flex items-start gap-2">
                                      <span
                                        className={`mt-0.5 inline-flex h-4 w-4 shrink-0 items-center justify-center rounded border ${
                                          checked
                                            ? "border-accent bg-accent/20 text-accent"
                                            : "border-border-glass bg-surface-glass-soft text-transparent"
                                        }`}
                                        aria-hidden="true"
                                      >
                                        {checked ? (
                                          <svg viewBox="0 0 16 16" className="h-3 w-3" aria-hidden="true">
                                            <path
                                              d="m3.5 8.25 2.5 2.5L12.5 4.5"
                                              fill="none"
                                              stroke="currentColor"
                                              strokeLinecap="round"
                                              strokeLinejoin="round"
                                              strokeWidth="1.6"
                                            />
                                          </svg>
                                        ) : null}
                                      </span>
                                      <div className="min-w-0 flex-1">
                                        <p className="m-0 break-all text-xs text-text-primary">{item.path}</p>
                                        <p className="m-0 mt-0.5 text-[11px] text-text-secondary">
                                          {t("cleanup.itemMeta", {
                                            value: `${formatBytes(item.sizeBytes)} · ${item.scope} · ${item.kind} · ${t(`cleanup.confidence.${item.confidence}`, { defaultValue: item.confidence })}`,
                                          })}
                                        </p>
                                        {item.evidence.length > 0 ? (
                                          <p className="m-0 mt-0.5 break-all text-[11px] text-text-muted">
                                            {t("cleanup.evidence", { value: item.evidence.join(", ") })}
                                          </p>
                                        ) : null}
                                        {item.readonlyReasonCode ? (
                                          <p className="m-0 mt-0.5 text-[11px] text-info">
                                            {t(`readonly.${item.readonlyReasonCode}`, {
                                              defaultValue: item.readonlyReasonCode,
                                            })}
                                          </p>
                                        ) : null}
                                      </div>
                                    </div>
                                  </button>
                                );
                              })}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : (
                    <p className="mt-3 text-xs text-text-muted">{t("cleanup.scanHint")}</p>
                  )}

                  <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
                    <span className="text-xs text-text-secondary">
                      {t("cleanup.selectedCount", { count: selectedResidueIds.length })}
                    </span>
                    <Button
                      size="default"
                      variant="danger"
                      disabled={Boolean(cleanupLoadingById[selectedApp.id])}
                      onClick={() => void cleanupSelected(selectedApp.id)}
                    >
                      {cleanupLoadingById[selectedApp.id] ? t("cleanup.cleaning") : t("cleanup.cleanNow")}
                    </Button>
                  </div>
                </section>

                {selectedCleanupResult ? (
                  <section className="ui-glass-panel px-4 py-4">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <h3 className="m-0 text-sm font-semibold text-text-primary">{t("result.title")}</h3>
                      <Button
                        size="default"
                        variant="secondary"
                        disabled={
                          Boolean(cleanupLoadingById[selectedApp.id]) ||
                          selectedCleanupResult.failed.filter((row) => row.itemId !== "main-app").length === 0
                        }
                        onClick={() => void retryFailedCleanup(selectedApp.id)}
                      >
                        {t("result.retryFailed")}
                      </Button>
                    </div>
                    <div className="mt-2 flex flex-wrap items-center gap-3 text-xs text-text-secondary">
                      <span>
                        {t("result.released", { value: formatBytes(selectedCleanupResult.releasedSizeBytes) })}
                      </span>
                      <span>{t("result.deleted", { count: selectedCleanupResult.deleted.length })}</span>
                      <span>{t("result.skipped", { count: selectedCleanupResult.skipped.length })}</span>
                      <span>{t("result.failed", { count: selectedCleanupResult.failed.length })}</span>
                    </div>
                    <div className="mt-3 space-y-3">
                      <ResultRows
                        title={t("result.deletedRows")}
                        rows={selectedCleanupResult.deleted}
                        kindClassName="border-border-glass bg-surface-glass-soft text-text-secondary"
                      />
                      <ResultRows
                        title={t("result.skippedRows")}
                        rows={selectedCleanupResult.skipped}
                        kindClassName="border-info/35 bg-info/10 text-info"
                      />
                      <ResultRows
                        title={t("result.failedRows")}
                        rows={selectedCleanupResult.failed}
                        kindClassName="border-danger/35 bg-danger/10 text-danger"
                      />
                    </div>
                  </section>
                ) : null}
              </>
            )}
          </div>
        </div>
      </div>

      <Dialog
        open={Boolean(confirmTarget)}
        onClose={() => {
          if (confirmingDeepUninstall) {
            return;
          }
          setConfirmTarget(null);
        }}
        canClose={!confirmingDeepUninstall}
        className="ui-glass-panel-strong mx-auto mt-[10vh] w-[min(620px,92vw)] p-4"
        ariaLabel={t("uninstallDialog.title")}
      >
        <div className="space-y-3">
          <div className="space-y-1">
            <h3 className="m-0 text-base font-semibold text-text-primary">{t("uninstallDialog.title")}</h3>
            <p className="m-0 text-sm text-text-secondary">{t("uninstallDialog.deepDesc")}</p>
          </div>
          {confirmTarget ? (
            <div className="space-y-1 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-2 text-xs text-text-secondary shadow-inset-soft">
              <div>{t("uninstallDialog.appName", { value: confirmTarget.name })}</div>
              <div>{t("uninstallDialog.appPath", { value: confirmTarget.path })}</div>
              <div>{t("uninstallDialog.publisher", { value: confirmTarget.publisher || "-" })}</div>
              <div>{t("uninstallDialog.version", { value: confirmTarget.version || "-" })}</div>
              <div>
                {t("uninstallDialog.cleanMode", {
                  value: t(
                    `cleanup.${(deleteModeByAppId[confirmTarget.id] ?? "trash") === "trash" ? "deleteModeTrash" : "deleteModePermanent"}`,
                  ),
                })}
              </div>
            </div>
          ) : null}
          <div className="flex justify-end gap-2">
            <Button
              size="default"
              variant="secondary"
              disabled={confirmingDeepUninstall}
              onClick={() => setConfirmTarget(null)}
            >
              {t("uninstallDialog.cancel")}
            </Button>
            <Button
              size="default"
              variant="danger"
              disabled={confirmingDeepUninstall || !confirmTarget}
              onClick={() => void onConfirmDeepUninstall()}
            >
              {confirmingDeepUninstall ? t("uninstallDialog.confirming") : t("uninstallDialog.confirmDeep")}
            </Button>
          </div>
        </div>
      </Dialog>
    </section>
  );
}

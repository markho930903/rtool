import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  AppManagerCleanupItemResult,
  AppManagerResidueGroup,
  AppManagerResidueItem,
  ManagedApp,
} from "@/components/app-manager/types";
import { Button, Dialog, Input, Select, SwitchField } from "@/components/ui";
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

interface RelatedLocationEntry {
  id: string;
  path: string;
  name: string;
  sizeBytes?: number | null;
  readonlyReasonCode?: string;
  source: "main" | "scan";
}

function normalizePathKey(path: string): string {
  return path.trim().replace(/[\\/]+/g, "/").toLowerCase();
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

function isLikelyFilePath(path: string): boolean {
  const name = getPathName(path);
  if (!name) {
    return false;
  }
  if (name.toLowerCase().endsWith(".app")) {
    return false;
  }
  return /\.[^./\\]+$/.test(name);
}

function relatedEntryIconClass(entry: RelatedLocationEntry): string {
  if (entry.source === "main") {
    return "i-noto:mobile-phone-with-arrow";
  }
  return isLikelyFilePath(entry.path) ? "i-noto:document" : "i-noto:open-file-folder";
}

function AppIcon({ app }: { app: ManagedApp }) {
  if (app.iconKind === "raster" && app.iconValue) {
    return (
      <img
        src={app.iconValue}
        alt=""
        className="h-8 w-8 rounded-md border border-border-muted bg-surface object-cover"
        loading="lazy"
      />
    );
  }

  const iconClass = app.iconValue || "i-noto:desktop-computer";
  return (
    <span className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-border-muted bg-surface-soft text-text-secondary">
      <span className={`btn-icon text-[1.05rem] ${iconClass}`} aria-hidden="true" />
    </span>
  );
}

function ResultRows({
  title,
  rows,
  kindClassName,
}: {
  title: string;
  rows: AppManagerCleanupItemResult[];
  kindClassName: string;
}) {
  if (rows.length === 0) {
    return null;
  }
  return (
    <div className="space-y-1.5">
      <h4 className="m-0 text-xs font-semibold text-text-secondary">{title}</h4>
      <div className="space-y-1.5">
        {rows.slice(0, 20).map((row) => (
          <div key={`${title}-${row.itemId}-${row.path}`} className={`rounded-md border px-2 py-1.5 text-xs ${kindClassName}`}>
            <div className="break-all">{row.path}</div>
            <div className="mt-0.5 text-[11px] opacity-80">
              {row.reasonCode} · {row.message}
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
  const startupOnly = useAppManagerStore((state) => state.startupOnly);
  const category = useAppManagerStore((state) => state.category);
  const nextCursor = useAppManagerStore((state) => state.nextCursor);
  const indexedAt = useAppManagerStore((state) => state.indexedAt);
  const error = useAppManagerStore((state) => state.error);
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
  const setStartupOnly = useAppManagerStore((state) => state.setStartupOnly);
  const setCategory = useAppManagerStore((state) => state.setCategory);
  const setExperimentalThirdPartyStartup = useAppManagerStore((state) => state.setExperimentalThirdPartyStartup);
  const clearLastActionResult = useAppManagerStore((state) => state.clearLastActionResult);
  const selectApp = useAppManagerStore((state) => state.selectApp);
  const loadFirstPage = useAppManagerStore((state) => state.loadFirstPage);
  const loadMore = useAppManagerStore((state) => state.loadMore);
  const refreshIndex = useAppManagerStore((state) => state.refreshIndex);
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
      void loadFirstPage();
    }, 150);
    return () => {
      window.clearTimeout(timer);
    };
  }, [keyword, startupOnly, category, loadFirstPage]);

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

  const selectedApp = useMemo(
    () => items.find((item) => item.id === selectedAppId) ?? null,
    [items, selectedAppId],
  );
  const selectedDetail = selectedApp ? detailById[selectedApp.id] : undefined;
  const selectedScanResult = selectedApp ? scanResultById[selectedApp.id] : undefined;
  const selectedCleanupResult = selectedApp ? cleanupResultById[selectedApp.id] : undefined;
  const selectedExportResult = selectedApp ? exportResultById[selectedApp.id] : undefined;
  const selectedResidueIds = selectedApp ? selectedResidueIdsByAppId[selectedApp.id] ?? [] : [];
  const selectedDeleteMode = selectedApp ? deleteModeByAppId[selectedApp.id] ?? "trash" : "trash";
  const selectedIncludeMainApp = selectedApp ? includeMainAppByAppId[selectedApp.id] ?? true : true;
  const selectedScanLoading = selectedApp ? Boolean(scanLoadingById[selectedApp.id]) : false;

  const categoryOptions = useMemo(
    () => [
      { value: "all", label: t("filters.category.all") },
      { value: "application", label: t("filters.category.application") },
      { value: "rtool", label: t("filters.category.rtool") },
      { value: "startup", label: t("filters.category.startup") },
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
      sizeBytes: selectedApp.estimatedSizeBytes,
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
  }, [selectedApp, selectedDetail?.installPath, selectedDetail?.relatedRoots, selectedScanResult]);

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
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
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
    <section className="h-full min-h-0">
      <div className="grid h-full min-h-0 gap-4 md:grid-cols-[360px_minmax(0,1fr)]">
        <aside className="flex h-full min-h-0 flex-col rounded-xl border border-border-muted bg-surface-card shadow-surface">
          <div className="shrink-0 space-y-3 border-b border-border-muted px-3 py-3">
            <div className="flex items-start justify-between gap-2">
              <div className="space-y-0.5">
                <h1 className="m-0 text-base font-semibold text-text-primary">{t("title")}</h1>
                <p className="m-0 text-xs text-text-secondary">{t("desc")}</p>
              </div>
              <Button size="xs" variant="secondary" disabled={refreshing || loading} onClick={() => void refreshIndex()}>
                {refreshing ? t("actions.refreshing") : t("actions.refresh")}
              </Button>
            </div>

            <Input
              value={keyword}
              placeholder={t("filters.keywordPlaceholder")}
              onChange={(event) => setKeyword(event.currentTarget.value)}
            />
            <Select value={category} options={categoryOptions} onChange={(event) => setCategory(event.currentTarget.value)} />
            <SwitchField
              checked={startupOnly}
              wrapperClassName="w-auto items-center"
              onChange={(event) => setStartupOnly(event.currentTarget.checked)}
              label={<span className="text-xs text-text-primary">{t("filters.startupOnly")}</span>}
            />
            <div className="rounded-lg border border-border-muted bg-surface-soft px-2.5 py-2">
              <SwitchField
                checked={experimentalThirdPartyStartup}
                controlPosition="end"
                onChange={(event) => setExperimentalThirdPartyStartup(event.currentTarget.checked)}
                label={<span className="text-xs text-text-primary">{t("experimental.title")}</span>}
                description={<span className="leading-5">{t("experimental.desc")}</span>}
              />
            </div>

            <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-text-muted">
              <span>{t("meta.indexedAt", { value: indexedAtText })}</span>
              <span>{t("meta.count", { count: items.length })}</span>
            </div>

            {error ? <div className="rounded-md border border-danger/35 bg-danger/10 px-2.5 py-2 text-xs text-danger">{error}</div> : null}
            {lastActionResult ? (
              <div
                className={`rounded-md border px-2.5 py-2 text-xs ${
                  lastActionResult.ok
                    ? "border-border-muted bg-surface-soft text-text-secondary"
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
              {loading ? <span className="text-xs text-text-muted">{t("status.loading")}</span> : null}
            </div>
            {!loading && items.length === 0 ? (
              <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-6 text-center text-sm text-text-muted">
                {t("status.empty")}
              </div>
            ) : null}
            <div className="space-y-2">
              {items.map((app) => {
                const selected = app.id === selectedAppId;
                const actionLoading = Boolean(actionLoadingById[app.id]);
                return (
                  <button
                    key={app.id}
                    type="button"
                    className={`w-full rounded-lg border px-3 py-2 text-left transition-colors ${
                      selected
                        ? "border-accent/70 bg-accent/10"
                        : "border-border-muted bg-surface-soft hover:border-accent/45"
                    }`}
                    onClick={() => void selectApp(app.id)}
                  >
                    <div className="flex items-start gap-2">
                      <AppIcon app={app} />
                      <div className="min-w-0 flex-1">
                        <div className="truncate text-sm font-medium text-text-primary">{app.name}</div>
                        <div className="mt-0.5 truncate text-xs text-text-muted">{app.path}</div>
                        <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-text-secondary">
                          <span>{formatBytes(app.estimatedSizeBytes)}</span>
                          <span>{t(`meta.startupScope.${app.startupScope}`, { defaultValue: app.startupScope })}</span>
                        </div>
                      </div>
                      <Button
                        size="xs"
                        variant="ghost"
                        iconOnly
                        disabled={actionLoading}
                        title={t("actions.deepUninstall")}
                        onClick={(event) => {
                          event.preventDefault();
                          event.stopPropagation();
                          setConfirmTarget(app);
                        }}
                      >
                        <span className="btn-icon i-noto:wastebasket text-base" aria-hidden="true" />
                      </Button>
                    </div>
                  </button>
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
          </div>
        </aside>

        <div className="h-full min-h-0 overflow-y-auto pr-1">
          <div className="space-y-3 pb-2">
            {!selectedApp ? (
              <div className="rounded-xl border border-border-muted bg-surface-card px-4 py-8 text-center text-sm text-text-muted">
                {t("detail.empty")}
              </div>
            ) : (
              <>
                <section className="rounded-xl border border-border-muted bg-surface-card px-4 py-4 shadow-surface">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <AppIcon app={selectedApp} />
                        <h2 className="m-0 text-base font-semibold text-text-primary">{selectedApp.name}</h2>
                      </div>
                      <p className="m-0 break-all text-xs text-text-muted">{selectedDetail?.installPath ?? selectedApp.path}</p>
                      <div className="flex flex-wrap items-center gap-2 text-xs text-text-secondary">
                        <span>{t("meta.platform", { value: selectedApp.platform })}</span>
                        {selectedApp.version ? <span>{t("meta.version", { value: selectedApp.version })}</span> : null}
                        {selectedApp.publisher ? <span>{t("meta.publisher", { value: selectedApp.publisher })}</span> : null}
                        {selectedApp.bundleOrAppId ? <span>{t("meta.bundleId", { value: selectedApp.bundleOrAppId })}</span> : null}
                        <span>{t("meta.identity", { value: selectedApp.identity.primaryId })}</span>
                        <span>{t("meta.identitySource", { value: selectedApp.identity.identitySource })}</span>
                        <span>{t("detail.size", { value: formatBytes(selectedApp.estimatedSizeBytes) })}</span>
                      </div>
                      <div className="flex flex-wrap items-center gap-2 text-[11px] text-text-muted">
                        <span>{selectedApp.capabilities.startup ? t("meta.capability.startupEnabled") : t("meta.capability.startupDisabled")}</span>
                        <span>{selectedApp.capabilities.uninstall ? t("meta.capability.uninstallEnabled") : t("meta.capability.uninstallDisabled")}</span>
                        <span>{selectedApp.capabilities.residueScan ? t("meta.capability.scanEnabled") : t("meta.capability.scanDisabled")}</span>
                      </div>
                    </div>
                    <div className="flex flex-wrap items-center gap-2">
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
                  {detailLoadingById[selectedApp.id] ? (
                    <p className="mt-3 text-xs text-text-muted">{t("detail.loading")}</p>
                  ) : null}
                </section>

                <section className="rounded-xl border border-border-muted bg-surface-card px-4 py-4 shadow-surface">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <h3 className="m-0 text-sm font-semibold text-text-primary">{t("detail.relatedRoots")}</h3>
                    <span className="text-xs text-text-secondary">
                      {selectedScanLoading ? t("cleanup.scanning") : t("detail.relatedCount", { count: relatedLocations.length })}
                    </span>
                  </div>
                  <p className="m-0 mt-1 text-[11px] text-text-muted">{t("detail.relatedAutoScanHint")}</p>

                  {relatedRevealError ? (
                    <div className="mt-2 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                      {relatedRevealError}
                    </div>
                  ) : null}

                  <div className="mt-3 rounded-lg border border-border-muted bg-surface-soft">
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
                          return (
                            <div
                              key={entry.path}
                              className={`flex w-full items-start gap-2 px-3 py-2.5 text-left transition-colors ${
                                selected ? "bg-accent/8" : "hover:bg-surface"
                              }`}
                            >
                              <button
                                type="button"
                                aria-pressed={selected}
                                disabled={selectionDisabled || Boolean(revealingRelatedId)}
                                className={`mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full border transition-colors ${
                                  selected
                                    ? "border-accent bg-accent text-accent-foreground"
                                    : "border-border-strong bg-surface text-transparent hover:border-accent/55"
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
                                <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border-muted bg-surface text-text-secondary">
                                  <span className={`btn-icon text-[1rem] ${relatedEntryIconClass(entry)}`} aria-hidden="true" />
                                </span>
                                <span className="min-w-0 flex-1">
                                  <span className="block truncate text-sm font-medium text-text-primary">{entry.name}</span>
                                  <span className="mt-0.5 block break-all text-xs text-text-muted">{entry.path}</span>
                                  {entry.readonlyReasonCode ? (
                                    <span className="mt-0.5 block text-[11px] text-info">
                                      {t(`readonly.${entry.readonlyReasonCode}`, { defaultValue: entry.readonlyReasonCode })}
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

                <section className="rounded-xl border border-border-muted bg-surface-card px-4 py-4 shadow-surface">
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
                      onChange={(event) => setDeleteMode(selectedApp.id, event.currentTarget.value as "trash" | "permanent")}
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
                        <span>{t("cleanup.scanSummary", { count: selectedScanResult.groups.flatMap((group) => group.items).length })}</span>
                        <span>{t("cleanup.totalSize", { value: formatBytes(selectedScanResult.totalSizeBytes) })}</span>
                      </div>

                      {selectedScanResult.warnings.length > 0 ? (
                        <div className="space-y-1 rounded-md border border-info/45 bg-info/10 px-3 py-2 text-xs text-info">
                          {selectedScanResult.warnings.map((warning, index) => (
                            <div key={`${warning.code}-${index}`}>{warning.message}</div>
                          ))}
                        </div>
                      ) : null}

                      {selectedExportResult ? (
                        <div
                          className={`rounded-md border px-3 py-2 text-xs transition-colors ${
                            highlightExportPath
                              ? "border-accent/60 bg-accent/10 text-text-primary"
                              : "border-border-muted bg-surface text-text-secondary"
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
                          <div key={group.groupId} className="rounded-lg border border-border-muted bg-surface-soft px-3 py-2">
                            <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                              <h4 className="m-0 text-xs font-semibold text-text-primary">
                                {group.label}
                              </h4>
                              <span className="text-[11px] text-text-secondary">{formatBytes(group.totalSizeBytes)}</span>
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
                                        : "border-border-muted bg-surface hover:border-accent/35 hover:bg-surface-soft"
                                    } ${disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer"}`}
                                    onClick={() => toggleResidueItem(selectedApp.id, item.itemId, !checked)}
                                  >
                                    <div className="flex items-start gap-2">
                                      <span
                                        className={`mt-0.5 inline-flex h-4 w-4 shrink-0 items-center justify-center rounded border ${
                                          checked
                                            ? "border-accent bg-accent/20 text-accent"
                                            : "border-border-strong bg-surface-soft text-transparent"
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
                  <section className="rounded-xl border border-border-muted bg-surface-card px-4 py-4 shadow-surface">
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
                      <span>{t("result.released", { value: formatBytes(selectedCleanupResult.releasedSizeBytes) })}</span>
                      <span>{t("result.deleted", { count: selectedCleanupResult.deleted.length })}</span>
                      <span>{t("result.skipped", { count: selectedCleanupResult.skipped.length })}</span>
                      <span>{t("result.failed", { count: selectedCleanupResult.failed.length })}</span>
                    </div>
                    <div className="mt-3 space-y-3">
                      <ResultRows
                        title={t("result.deletedRows")}
                        rows={selectedCleanupResult.deleted}
                        kindClassName="border-border-muted bg-surface-soft text-text-secondary"
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
        className="mx-auto mt-[10vh] w-[min(620px,92vw)] rounded-xl border border-border-muted bg-surface-card p-4 shadow-overlay"
        ariaLabel={t("uninstallDialog.title")}
      >
        <div className="space-y-3">
          <div className="space-y-1">
            <h3 className="m-0 text-base font-semibold text-text-primary">{t("uninstallDialog.title")}</h3>
            <p className="m-0 text-sm text-text-secondary">{t("uninstallDialog.deepDesc")}</p>
          </div>
          {confirmTarget ? (
            <div className="space-y-1 rounded-lg border border-border-muted bg-surface-soft px-3 py-2 text-xs text-text-secondary">
              <div>{t("uninstallDialog.appName", { value: confirmTarget.name })}</div>
              <div>{t("uninstallDialog.appPath", { value: confirmTarget.path })}</div>
              <div>{t("uninstallDialog.publisher", { value: confirmTarget.publisher || "-" })}</div>
              <div>{t("uninstallDialog.version", { value: confirmTarget.version || "-" })}</div>
              <div>{t("uninstallDialog.cleanMode", { value: t(`cleanup.${(deleteModeByAppId[confirmTarget.id] ?? "trash") === "trash" ? "deleteModeTrash" : "deleteModePermanent"}`) })}</div>
            </div>
          ) : null}
          <div className="flex justify-end gap-2">
            <Button size="default" variant="secondary" disabled={confirmingDeepUninstall} onClick={() => setConfirmTarget(null)}>
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

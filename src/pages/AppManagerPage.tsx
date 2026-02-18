import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  AppManagerCleanupItemResult,
  AppManagerResidueGroup,
  AppManagerResidueItem,
  ManagedApp,
} from "@/components/app-manager/types";
import { Button, Checkbox, Dialog, Input, Select } from "@/components/ui";
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

  const isStartupActionDisabled = (app: ManagedApp): boolean => {
    const actionLoading = Boolean(actionLoadingById[app.id]);
    const thirdParty = app.source !== "rtool";
    const startupDisabledByExperiment = thirdParty && !experimentalThirdPartyStartup;
    return actionLoading || !app.startupEditable || startupDisabledByExperiment;
  };

  return (
    <section className="h-full min-h-0">
      <div className="space-y-4">
        <header className="rounded-xl border border-border-muted bg-surface-card px-4 py-4 shadow-surface">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="space-y-1">
              <h1 className="m-0 text-lg font-semibold text-text-primary">{t("title")}</h1>
              <p className="m-0 text-sm text-text-secondary">{t("desc")}</p>
            </div>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="secondary" disabled={refreshing || loading} onClick={() => void refreshIndex()}>
                {refreshing ? t("actions.refreshing") : t("actions.refresh")}
              </Button>
            </div>
          </div>

          <div className="mt-3 grid gap-3 md:grid-cols-[1fr_220px_auto]">
            <Input
              value={keyword}
              placeholder={t("filters.keywordPlaceholder")}
              onChange={(event) => setKeyword(event.currentTarget.value)}
            />
            <Select value={category} options={categoryOptions} onChange={(event) => setCategory(event.currentTarget.value)} />
            <Checkbox
              size="sm"
              checked={startupOnly}
              onChange={(event) => setStartupOnly(event.currentTarget.checked)}
              label={<span className="text-sm text-text-primary">{t("filters.startupOnly")}</span>}
            />
          </div>

          <div className="mt-3 rounded-lg border border-border-muted bg-surface-soft px-3 py-2">
            <Checkbox
              size="sm"
              checked={experimentalThirdPartyStartup}
              onChange={(event) => setExperimentalThirdPartyStartup(event.currentTarget.checked)}
              label={<span className="text-sm text-text-primary">{t("experimental.title")}</span>}
              description={<span className="leading-5">{t("experimental.desc")}</span>}
              wrapperClassName="items-start gap-2"
            />
          </div>

          <div className="mt-3 flex flex-wrap items-center justify-between gap-2 text-xs text-text-muted">
            <span>{t("meta.indexedAt", { value: indexedAtText })}</span>
            <span>{t("meta.count", { count: items.length })}</span>
          </div>
        </header>

        {error ? <div className="rounded-lg border border-danger/35 bg-danger/10 px-3 py-2 text-sm text-danger">{error}</div> : null}
        {lastActionResult ? (
          <div
            className={`rounded-lg border px-3 py-2 text-sm ${
              lastActionResult.ok
                ? "border-border-muted bg-surface-soft text-text-secondary"
                : "border-danger/35 bg-danger/10 text-danger"
            }`}
          >
            <div className="flex items-center justify-between gap-2">
              <span>{lastActionResult.message}</span>
              <Button size="xs" variant="ghost" onClick={() => clearLastActionResult()}>
                {t("actions.dismiss")}
              </Button>
            </div>
          </div>
        ) : null}

        <div className="grid min-h-[58vh] gap-4 md:grid-cols-[360px_minmax(0,1fr)]">
          <aside className="rounded-xl border border-border-muted bg-surface-card p-3 shadow-surface">
            <div className="mb-2 flex items-center justify-between">
              <h2 className="m-0 text-sm font-semibold text-text-primary">{t("list.title")}</h2>
              {loading ? <span className="text-xs text-text-muted">{t("status.loading")}</span> : null}
            </div>
            {!loading && items.length === 0 ? (
              <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-6 text-center text-sm text-text-muted">
                {t("status.empty")}
              </div>
            ) : null}
            <div className="max-h-[64vh] space-y-2 overflow-y-auto pr-1">
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
                <Button size="sm" variant="secondary" disabled={loadingMore} onClick={() => void loadMore()}>
                  {loadingMore ? t("actions.loadingMore") : t("actions.loadMore")}
                </Button>
              </div>
            ) : null}
          </aside>

          <div className="min-w-0 space-y-3">
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
                        <span>{t("detail.size", { value: formatBytes(selectedApp.estimatedSizeBytes) })}</span>
                      </div>
                    </div>
                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        size="sm"
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
                      <Button size="sm" variant="secondary" onClick={() => void openUninstallHelp(selectedApp)}>
                        {t("actions.uninstallGuide")}
                      </Button>
                      <Button size="sm" variant="danger" onClick={() => setConfirmTarget(selectedApp)}>
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
                  <h3 className="m-0 text-sm font-semibold text-text-primary">{t("detail.relatedRoots")}</h3>
                  <div className="mt-2 space-y-2">
                    {(selectedDetail?.relatedRoots ?? []).map((root) => (
                      <div key={root.id} className="rounded-lg border border-border-muted bg-surface-soft px-3 py-2">
                        <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-text-secondary">
                          <span className="font-medium text-text-primary">{root.label}</span>
                          <span>{t(`detail.scope.${root.scope}`, { defaultValue: root.scope })}</span>
                        </div>
                        <p className="m-0 mt-1 break-all text-[11px] text-text-muted">{root.path}</p>
                        {root.readonlyReasonCode ? (
                          <p className="m-0 mt-1 text-[11px] text-info">{t(`readonly.${root.readonlyReasonCode}`, { defaultValue: root.readonlyReasonCode })}</p>
                        ) : null}
                      </div>
                    ))}
                    {!selectedDetail?.relatedRoots.length ? (
                      <p className="m-0 text-xs text-text-muted">{t("detail.noRelatedRoots")}</p>
                    ) : null}
                  </div>
                </section>

                <section className="rounded-xl border border-border-muted bg-surface-card px-4 py-4 shadow-surface">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <h3 className="m-0 text-sm font-semibold text-text-primary">{t("cleanup.title")}</h3>
                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        size="sm"
                        variant="secondary"
                        disabled={Boolean(scanLoadingById[selectedApp.id])}
                        onClick={() => void scanResidue(selectedApp.id)}
                      >
                        {scanLoadingById[selectedApp.id] ? t("cleanup.scanning") : t("cleanup.scan")}
                      </Button>
                      <Button
                        size="sm"
                        variant="secondary"
                        disabled={!selectedScanResult}
                        onClick={() => selectRecommendedResidues(selectedApp.id)}
                      >
                        {t("cleanup.selectRecommended")}
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={!selectedScanResult}
                        onClick={() => clearResidueSelection(selectedApp.id)}
                      >
                        {t("cleanup.clearSelection")}
                      </Button>
                      <Button
                        size="sm"
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
                    <Checkbox
                      size="sm"
                      checked={selectedIncludeMainApp}
                      onChange={(event) => setIncludeMainApp(selectedApp.id, event.currentTarget.checked)}
                      label={<span className="text-sm text-text-primary">{t("cleanup.includeMainApp")}</span>}
                      description={<span className="leading-5">{t("cleanup.includeMainAppDesc")}</span>}
                      wrapperClassName="items-start gap-2"
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
                                  <div key={item.itemId} className="rounded-md border border-border-muted bg-surface px-2 py-1.5">
                                    <div className="flex items-start gap-2">
                                      <Checkbox
                                        size="sm"
                                        checked={checked}
                                        disabled={disabled}
                                        onChange={(event) =>
                                          toggleResidueItem(selectedApp.id, item.itemId, event.currentTarget.checked)
                                        }
                                      />
                                      <div className="min-w-0 flex-1">
                                        <p className="m-0 break-all text-xs text-text-primary">{item.path}</p>
                                        <p className="m-0 mt-0.5 text-[11px] text-text-secondary">
                                          {t("cleanup.itemMeta", {
                                            value: `${formatBytes(item.sizeBytes)} · ${item.scope} · ${item.kind}`,
                                          })}
                                        </p>
                                        {item.readonlyReasonCode ? (
                                          <p className="m-0 mt-0.5 text-[11px] text-info">
                                            {t(`readonly.${item.readonlyReasonCode}`, {
                                              defaultValue: item.readonlyReasonCode,
                                            })}
                                          </p>
                                        ) : null}
                                      </div>
                                    </div>
                                  </div>
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
                      size="sm"
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
                        size="sm"
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
            <Button size="sm" variant="secondary" disabled={confirmingDeepUninstall} onClick={() => setConfirmTarget(null)}>
              {t("uninstallDialog.cancel")}
            </Button>
            <Button
              size="sm"
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

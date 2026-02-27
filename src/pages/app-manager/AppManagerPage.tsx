import { type ReactElement, useMemo } from "react";

import { AppDetailPane } from "@/pages/app-manager/AppDetailPane";
import { AppListPane } from "@/pages/app-manager/AppListPane";
import { useAppManagerController } from "@/pages/app-manager/useAppManagerController";

function formatIndexedAt(timestamp: number | null): string {
  if (!timestamp || !Number.isFinite(timestamp) || timestamp <= 0) {
    return "-";
  }
  return new Date(timestamp * 1000).toLocaleString();
}

export default function AppManagerPage(): ReactElement {
  const controller = useAppManagerController();
  const { list, detail, actions } = controller;
  const indexedAtText = useMemo(() => formatIndexedAt(list.indexedAt), [list.indexedAt]);

  return (
    <section className="h-full min-h-0">
      <div className="grid h-full min-h-0 gap-4 md:grid-cols-[380px_minmax(0,1fr)]">
        <AppListPane
          items={list.items}
          selectedAppId={list.selectedAppId}
          loading={list.loading}
          loadingMore={list.loadingMore}
          hasMore={list.hasMore}
          keyword={list.keyword}
          indexedAtText={indexedAtText}
          indexState={list.indexState}
          totalCount={list.totalCount}
          onKeywordChange={actions.setKeyword}
          onSelect={actions.setSelectedAppId}
          onRefresh={actions.refreshList}
          onLoadMore={actions.onLoadMore}
        />

        <div className="h-full min-h-0 overflow-hidden">
          <AppDetailPane
            selectedApp={detail.selectedApp}
            coreDetail={detail.coreDetail}
            heavyDetail={detail.heavyDetail}
            coreLoading={detail.coreLoading}
            heavyLoading={detail.heavyLoading}
            deepCompleting={detail.deepCompleting}
            detailError={detail.detailError}
            selectedResidueIds={detail.selectedResidueIds}
            selectedIncludeMain={detail.selectedIncludeMain}
            selectedDeleteMode={detail.selectedDeleteMode}
            cleanupLoading={detail.cleanupLoading}
            cleanupResult={detail.cleanupResult}
            cleanupError={detail.cleanupError}
            startupLoading={detail.startupLoading}
            uninstallLoading={detail.uninstallLoading}
            openHelpLoading={detail.openHelpLoading}
            exportLoading={detail.exportLoading}
            openExportDirLoading={detail.openExportDirLoading}
            exportResult={detail.exportResult}
            exportError={detail.exportError}
            actionResult={detail.actionResult}
            actionError={detail.actionError}
            onToggleResidue={actions.onToggleResidue}
            onSelectAllResidues={actions.onSelectAllResidues}
            onToggleIncludeMain={actions.onToggleIncludeMain}
            onSetDeleteMode={actions.onSetDeleteMode}
            onCleanupNow={actions.onCleanupNow}
            onRetryFailed={actions.onRetryFailed}
            onRevealPath={actions.onRevealPath}
            onScanAgain={actions.onScanAgain}
            onToggleStartup={actions.onToggleStartup}
            onOpenUninstallHelp={actions.onOpenUninstallHelp}
            onUninstall={actions.onUninstall}
            onExportScanResult={actions.onExportScanResult}
            onOpenExportDirectory={actions.onOpenExportDirectory}
          />
        </div>
      </div>
    </section>
  );
}

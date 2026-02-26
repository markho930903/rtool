import { useMemo } from "react";

import { AppDetailPane } from "@/features/app-manager/AppDetailPane";
import { AppListPane } from "@/features/app-manager/AppListPane";
import { useAppManagerController } from "@/features/app-manager/useAppManagerController";

function formatIndexedAt(timestamp: number | null): string {
  if (!timestamp || !Number.isFinite(timestamp) || timestamp <= 0) {
    return "-";
  }
  return new Date(timestamp * 1000).toLocaleString();
}

export function AppManagerShell() {
  const controller = useAppManagerController();
  const indexedAtText = useMemo(() => formatIndexedAt(controller.indexedAt), [controller.indexedAt]);

  return (
    <section className="h-full min-h-0">
      <div className="grid h-full min-h-0 gap-4 md:grid-cols-[380px_minmax(0,1fr)]">
        <AppListPane
          items={controller.items}
          selectedAppId={controller.selectedAppId}
          loading={controller.loading}
          loadingMore={controller.loadingMore}
          hasMore={controller.hasMore}
          keyword={controller.keyword}
          indexedAtText={indexedAtText}
          indexState={controller.indexState}
          totalCount={controller.totalCount}
          onKeywordChange={controller.setKeyword}
          onSelect={controller.setSelectedAppId}
          onRefresh={controller.refreshList}
          onLoadMore={controller.onLoadMore}
        />

        <div className="h-full min-h-0 overflow-hidden">
          <AppDetailPane
            selectedApp={controller.selectedApp}
            coreDetail={controller.selectedCore}
            heavyDetail={controller.selectedHeavy}
            coreLoading={controller.selectedCoreLoading}
            heavyLoading={controller.selectedHeavyLoading}
            deepCompleting={controller.selectedDeepCompleting}
            detailError={controller.detailError ?? controller.listError}
            selectedResidueIds={controller.selectedResidueIds}
            selectedIncludeMain={controller.selectedIncludeMain}
            selectedDeleteMode={controller.selectedDeleteMode}
            cleanupLoading={controller.selectedCleanupLoading}
            cleanupResult={controller.selectedCleanupResult}
            cleanupError={controller.cleanupError}
            onToggleResidue={(itemId, checked) => {
              if (!controller.selectedApp) {
                return;
              }
              controller.toggleResidue(controller.selectedApp.id, itemId, checked);
            }}
            onToggleIncludeMain={(checked) => {
              if (!controller.selectedApp) {
                return;
              }
              controller.setIncludeMain(controller.selectedApp.id, checked);
            }}
            onSetDeleteMode={(mode) => {
              if (!controller.selectedApp) {
                return;
              }
              controller.setDeleteMode(controller.selectedApp.id, mode);
            }}
            onCleanupNow={controller.cleanupNow}
            onRetryFailed={controller.retryFailed}
            onRevealPath={controller.revealPath}
            onScanAgain={controller.scanAgain}
          />
        </div>
      </div>
    </section>
  );
}

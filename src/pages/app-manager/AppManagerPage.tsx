import { type ReactElement } from "react";

import { AppDetailPane } from "@/pages/app-manager/AppDetailPane";
import { AppListPane } from "@/pages/app-manager/AppListPane";
import { useAppManagerScreen } from "@/pages/app-manager/useAppManagerScreen";

export default function AppManagerPage(): ReactElement {
  const controller = useAppManagerScreen();
  const { listPaneModel, detailPaneModel } = controller;

  return (
    <section className="h-full min-h-0">
      <div className="grid h-full min-h-0 gap-4 md:grid-cols-[380px_minmax(0,1fr)]">
        <AppListPane model={listPaneModel} />

        <div className="h-full min-h-0 overflow-hidden">
          <AppDetailPane model={detailPaneModel} />
        </div>
      </div>
    </section>
  );
}

import type { RefObject } from "react";
import { useTranslation } from "react-i18next";

import type { ResourceAnalysisRow } from "@/pages/resource-monitor/hooks/useResourceMonitorData";

interface ResourceAttributionSectionProps {
  crateChartRef: RefObject<HTMLDivElement | null>;
  hasCrateChartData: boolean;
  analysisRows: ResourceAnalysisRow[];
}

export default function ResourceAttributionSection(props: ResourceAttributionSectionProps) {
  const { t } = useTranslation(["resource_monitor"]);

  return (
    <section className="grid grid-cols-1 gap-3 xl:grid-cols-[1.15fr_1fr]">
      <article className="ui-glass-panel p-4">
        <header className="mb-3">
          <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
            {t("panel.crates.title")}
          </div>
          <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.crates.subtitle")}</h2>
        </header>
        {props.hasCrateChartData ? (
          <div ref={props.crateChartRef} className="h-[240px] w-full" />
        ) : (
          <div className="py-10 text-center text-sm text-text-muted">{t("chart.noData")}</div>
        )}
      </article>

      <article className="ui-glass-panel p-4">
        <header className="mb-3">
          <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
            {t("panel.analysis.title")}
          </div>
          <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.analysis.subtitle")}</h2>
        </header>

        <div className="space-y-2">
          {props.analysisRows.map((item) => (
            <div
              key={item.moduleId}
              className="flex items-start justify-between gap-3 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-2 shadow-inset-soft"
            >
              <div>
                <div className="text-sm font-medium text-text-primary">{item.moduleLabel}</div>
                <div className="mt-0.5 text-xs text-text-muted">{item.detailText}</div>
              </div>

              <div className="text-right">
                <div className="font-mono text-xs text-accent">{item.cpuText}</div>
                <div className="mt-0.5 font-mono text-xs text-text-secondary">{item.memoryText}</div>
              </div>
            </div>
          ))}
        </div>
      </article>
    </section>
  );
}

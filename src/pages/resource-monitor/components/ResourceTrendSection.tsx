import type { RefObject } from "react";
import { useTranslation } from "react-i18next";

interface ResourceTrendSectionProps {
  historyChartRef: RefObject<HTMLDivElement | null>;
  moduleChartRef: RefObject<HTMLDivElement | null>;
  hasHistoryData: boolean;
  hasModuleChartData: boolean;
}

export default function ResourceTrendSection(props: ResourceTrendSectionProps) {
  const { t } = useTranslation(["resource_monitor"]);
  const historyLegend = [
    { key: "cpu", label: t("chart.series.cpu"), color: "var(--color-chart-series-1)" },
    { key: "memory", label: t("chart.series.memoryMb"), color: "var(--color-chart-series-2)" },
  ] as const;

  return (
    <section className="grid grid-cols-1 gap-3 xl:grid-cols-2">
      <article className="ui-glass-panel p-4">
        <header className="mb-3">
          <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">{t("panel.timeline.title")}</div>
          <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.timeline.subtitle")}</h2>
        </header>
        <div className="mb-2 flex flex-wrap items-center gap-x-4 gap-y-1">
          {historyLegend.map((item) => (
            <span key={item.key} className="inline-flex items-center gap-2 whitespace-nowrap ui-text-caption text-text-secondary">
              <span className="inline-block h-[2px] w-4 rounded-full" style={{ background: item.color }} />
              <span>{item.label}</span>
            </span>
          ))}
        </div>
        {props.hasHistoryData ? (
          <div ref={props.historyChartRef} className="h-[280px] w-full" />
        ) : (
          <div className="py-10 text-center text-sm text-text-muted">{t("chart.noData")}</div>
        )}
      </article>

      <article className="ui-glass-panel p-4">
        <header className="mb-3">
          <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">{t("panel.modules.title")}</div>
          <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.modules.subtitle")}</h2>
        </header>
        {props.hasModuleChartData ? (
          <div ref={props.moduleChartRef} className="h-[280px] w-full" />
        ) : (
          <div className="py-10 text-center text-sm text-text-muted">{t("chart.noData")}</div>
        )}
      </article>
    </section>
  );
}

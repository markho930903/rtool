import { Chart } from "@antv/g2";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

import { Button, Select } from "@/components/ui";
import type { ResourceCrateStatsDto, ResourceModuleStatsDto, ResourcePointDto } from "@/contracts";
import { useResourceMonitorStore, type ResourceSortMetric } from "@/stores/resource-monitor.store";
import { useThemeStore } from "@/theme/store";

function formatBytes(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes) || bytes < 0) {
    return "--";
  }
  if (bytes === 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"] as const;
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** exponent;
  const digits = exponent <= 1 ? 0 : value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${units[exponent]}`;
}

function formatPercent(value: number | null): string {
  if (value === null || !Number.isFinite(value)) {
    return "--";
  }
  return `${value.toFixed(2)}%`;
}

function formatTime(value: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(value));
}

function readColorToken(name: string): string {
  if (typeof window === "undefined") {
    return "#60a5fa";
  }
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || "#60a5fa";
}

function pickSortValue(metric: ResourceSortMetric, item: ResourceModuleStatsDto | ResourceCrateStatsDto): number {
  if (metric === "cpu") {
    return item.estimatedCpuPercent ?? -1;
  }
  if (metric === "memory") {
    return item.estimatedMemoryBytes ?? -1;
  }
  return item.calls;
}

interface MetricCardProps {
  title: string;
  value: string;
  hint: string;
}

function MetricCard(props: MetricCardProps) {
  return (
    <article className="rounded-xl border border-border-muted bg-surface px-4 py-3">
      <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">{props.title}</div>
      <div className="mt-2 text-lg leading-none font-semibold text-text-primary">{props.value}</div>
      <div className="mt-1.5 text-xs text-text-secondary">{props.hint}</div>
    </article>
  );
}

function createHistoryChart(
  element: HTMLDivElement,
  points: ResourcePointDto[],
  locale: string,
  seriesLabel: { cpu: string; memory: string },
) {
  const accent = readColorToken("--color-accent");
  const info = readColorToken("--color-info");
  const data = points.flatMap((point) => {
    const time = formatTime(point.sampledAt, locale);
    const next = [];
    if (point.processCpuPercent !== null) {
      next.push({
        time,
        value: point.processCpuPercent,
        metric: seriesLabel.cpu,
      });
    }
    if (point.processMemoryBytes !== null) {
      next.push({
        time,
        value: point.processMemoryBytes / 1024 / 1024,
        metric: seriesLabel.memory,
      });
    }
    return next;
  });

  const chart = new Chart({
    container: element,
    autoFit: true,
    height: 280,
  });

  chart
    .line()
    .data(data)
    .encode("x", "time")
    .encode("y", "value")
    .encode("color", "metric")
    .scale("color", {
      range: [accent, info],
    })
    .style("lineWidth", 2);

  chart.render();
  return chart;
}

function createTopChart(
  element: HTMLDivElement,
  data: Array<{ name: string; value: number }>,
  color: string,
  height = 280,
) {
  const chart = new Chart({
    container: element,
    autoFit: true,
    height,
  });

  chart.interval().data(data).encode("x", "name").encode("y", "value").style("fill", color);

  chart.render();
  return chart;
}

export default function ResourceMonitorPage() {
  const { t, i18n } = useTranslation(["resource_monitor"]);
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const resolvedTheme = useThemeStore((state) => state.resolved);

  const initialized = useResourceMonitorStore((state) => state.initialized);
  const loading = useResourceMonitorStore((state) => state.loading);
  const error = useResourceMonitorStore((state) => state.error);
  const snapshot = useResourceMonitorStore((state) => state.snapshot);
  const history = useResourceMonitorStore((state) => state.history);
  const lastUpdatedAt = useResourceMonitorStore((state) => state.lastUpdatedAt);
  const historyWindowMinutes = useResourceMonitorStore((state) => state.historyWindowMinutes);
  const sortMetric = useResourceMonitorStore((state) => state.sortMetric);
  const initialize = useResourceMonitorStore((state) => state.initialize);
  const refreshAll = useResourceMonitorStore((state) => state.refreshAll);
  const startPolling = useResourceMonitorStore((state) => state.startPolling);
  const stopPolling = useResourceMonitorStore((state) => state.stopPolling);
  const setHistoryWindowMinutes = useResourceMonitorStore((state) => state.setHistoryWindowMinutes);
  const setSortMetric = useResourceMonitorStore((state) => state.setSortMetric);
  const resetSession = useResourceMonitorStore((state) => state.resetSession);

  const historyChartRef = useRef<HTMLDivElement | null>(null);
  const moduleChartRef = useRef<HTMLDivElement | null>(null);
  const crateChartRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    void initialize();
    startPolling();
    return () => {
      stopPolling();
    };
  }, [initialize, startPolling, stopPolling]);

  const visibleHistory = useMemo(() => {
    const cutoff = Date.now() - historyWindowMinutes * 60 * 1000;
    return history.filter((point) => point.sampledAt >= cutoff);
  }, [history, historyWindowMinutes]);

  const moduleStats = useMemo(() => {
    const modules = snapshot?.modules ?? [];
    return [...modules].sort((left, right) => pickSortValue(sortMetric, right) - pickSortValue(sortMetric, left));
  }, [snapshot?.modules, sortMetric]);

  const crateStats = useMemo(() => {
    const crates = snapshot?.crates ?? [];
    return [...crates].sort((left, right) => pickSortValue(sortMetric, right) - pickSortValue(sortMetric, left));
  }, [snapshot?.crates, sortMetric]);

  useEffect(() => {
    const container = historyChartRef.current;
    if (!container) {
      return;
    }
    if (visibleHistory.length === 0) {
      container.innerHTML = "";
      return;
    }
    const chart = createHistoryChart(container, visibleHistory, locale, {
      cpu: t("chart.series.cpu"),
      memory: t("chart.series.memoryMb"),
    });
    return () => {
      chart.destroy();
    };
  }, [locale, resolvedTheme, t, visibleHistory]);

  useEffect(() => {
    const container = moduleChartRef.current;
    if (!container) {
      return;
    }
    const data = moduleStats
      .slice(0, 8)
      .map((item) => ({ name: t(`module.${item.moduleId}`), value: pickSortValue(sortMetric, item) }))
      .filter((item) => item.value >= 0);
    if (data.length === 0) {
      container.innerHTML = "";
      return;
    }
    const color = readColorToken("--color-accent");
    const chart = createTopChart(container, data, color);
    return () => {
      chart.destroy();
    };
  }, [moduleStats, resolvedTheme, sortMetric, t]);

  useEffect(() => {
    const container = crateChartRef.current;
    if (!container) {
      return;
    }
    const data = crateStats
      .slice(0, 8)
      .map((item) => ({ name: t(`crate.${item.crateId}`), value: pickSortValue(sortMetric, item) }))
      .filter((item) => item.value >= 0);
    if (data.length === 0) {
      container.innerHTML = "";
      return;
    }
    const color = readColorToken("--color-info");
    const chart = createTopChart(container, data, color, 240);
    return () => {
      chart.destroy();
    };
  }, [crateStats, resolvedTheme, sortMetric, t]);

  const overview = snapshot?.overview;
  const memoryUsageHint =
    overview?.systemUsedMemoryBytes !== null && overview?.systemTotalMemoryBytes !== null
      ? `${formatBytes(overview?.systemUsedMemoryBytes ?? null)} / ${formatBytes(overview?.systemTotalMemoryBytes ?? null)}`
      : "--";
  const activeModuleCount = moduleStats.filter((item) => item.calls > 0).length;

  return (
    <div className="space-y-3 pb-2">
      <section className="rounded-2xl border border-border-strong bg-surface p-4">
        <div className="font-mono ui-text-micro uppercase tracking-ui-wider text-text-muted">
          rtool / resource monitor
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <h1 className="m-0 text-xl font-semibold tracking-tight text-text-primary">{t("header.title")}</h1>
          <span className="rounded-full border border-border-muted bg-app px-2 py-0.5 font-mono ui-text-micro uppercase tracking-ui-wide text-accent">
            {loading && !initialized ? t("status.booting") : error ? t("status.degraded") : t("status.online")}
          </span>
        </div>
        <p className="mt-2 text-sm text-text-secondary">{t("header.subtitle")}</p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Button size="default" variant="secondary" onClick={() => void refreshAll()}>
            <span
              className="btn-icon i-noto:anticlockwise-downwards-and-upwards-open-circle-arrows"
              aria-hidden="true"
            />
            <span>{t("action.refreshNow")}</span>
          </Button>
          <Button size="default" variant="danger" onClick={() => void resetSession()}>
            <span className="btn-icon i-noto:broom" aria-hidden="true" />
            <span>{t("action.resetSession")}</span>
          </Button>
          <div className="w-[150px]">
            <Select
              value={`${historyWindowMinutes}`}
              options={[
                { value: "5", label: t("filter.window.5m") },
                { value: "15", label: t("filter.window.15m") },
                { value: "30", label: t("filter.window.30m") },
              ]}
              onChange={(event) => {
                const value = Number(event.target.value);
                if (value === 5 || value === 15 || value === 30) {
                  setHistoryWindowMinutes(value);
                }
              }}
            />
          </div>
          <div className="w-[180px]">
            <Select
              value={sortMetric}
              options={[
                { value: "cpu", label: t("filter.sort.cpu") },
                { value: "memory", label: t("filter.sort.memory") },
                { value: "calls", label: t("filter.sort.calls") },
              ]}
              onChange={(event) => {
                const value = event.target.value;
                if (value === "cpu" || value === "memory" || value === "calls") {
                  setSortMetric(value);
                }
              }}
            />
          </div>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-text-muted">
          <span>
            {t("meta.sampledAt", { value: overview?.sampledAt ? formatTime(overview.sampledAt, locale) : "--" })}
          </span>
          <span>{t("meta.lastUpdated", { value: lastUpdatedAt ? formatTime(lastUpdatedAt, locale) : "--" })}</span>
          <span>{t("meta.historyPoints", { value: visibleHistory.length })}</span>
        </div>
        {error ? (
          <div className="mt-2 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
            {t("error.sampleFailed", { message: error })}
          </div>
        ) : null}
      </section>

      <section className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          title={t("metric.processCpu.title")}
          value={formatPercent(overview?.processCpuPercent ?? null)}
          hint={t("metric.processCpu.hint")}
        />
        <MetricCard
          title={t("metric.processMemory.title")}
          value={formatBytes(overview?.processMemoryBytes ?? null)}
          hint={t("metric.processMemory.hint")}
        />
        <MetricCard
          title={t("metric.systemMemory.title")}
          value={memoryUsageHint}
          hint={t("metric.systemMemory.hint")}
        />
        <MetricCard
          title={t("metric.activeModules.title")}
          value={`${activeModuleCount}`}
          hint={t("metric.activeModules.hint")}
        />
      </section>

      <section className="grid grid-cols-1 gap-3 xl:grid-cols-2">
        <article className="rounded-xl border border-border-muted bg-surface p-4">
          <header className="mb-3">
            <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
              {t("panel.timeline.title")}
            </div>
            <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.timeline.subtitle")}</h2>
          </header>
          {visibleHistory.length === 0 ? (
            <div className="py-10 text-center text-sm text-text-muted">{t("chart.noData")}</div>
          ) : (
            <div ref={historyChartRef} className="h-[280px] w-full" />
          )}
        </article>

        <article className="rounded-xl border border-border-muted bg-surface p-4">
          <header className="mb-3">
            <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
              {t("panel.modules.title")}
            </div>
            <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.modules.subtitle")}</h2>
          </header>
          {moduleStats.length === 0 ? (
            <div className="py-10 text-center text-sm text-text-muted">{t("chart.noData")}</div>
          ) : (
            <div ref={moduleChartRef} className="h-[280px] w-full" />
          )}
        </article>
      </section>

      <section className="grid grid-cols-1 gap-3 xl:grid-cols-[1.15fr_1fr]">
        <article className="rounded-xl border border-border-muted bg-surface p-4">
          <header className="mb-3">
            <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
              {t("panel.crates.title")}
            </div>
            <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.crates.subtitle")}</h2>
          </header>
          {crateStats.length === 0 ? (
            <div className="py-10 text-center text-sm text-text-muted">{t("chart.noData")}</div>
          ) : (
            <div ref={crateChartRef} className="h-[240px] w-full" />
          )}
        </article>

        <article className="rounded-xl border border-border-muted bg-surface p-4">
          <header className="mb-3">
            <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
              {t("panel.analysis.title")}
            </div>
            <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.analysis.subtitle")}</h2>
          </header>
          <div className="space-y-2">
            {moduleStats.slice(0, 6).map((item) => (
              <div
                key={item.moduleId}
                className="flex items-start justify-between gap-3 rounded-lg border border-border-muted/70 bg-app/55 px-3 py-2"
              >
                <div>
                  <div className="text-sm font-medium text-text-primary">{t(`module.${item.moduleId}`)}</div>
                  <div className="mt-0.5 text-xs text-text-muted">
                    {t("analysis.row", {
                      calls: item.calls,
                      errors: item.errorCalls,
                      avg: item.avgDurationMs ?? 0,
                    })}
                  </div>
                </div>
                <div className="text-right">
                  <div className="font-mono text-xs text-accent">{formatPercent(item.estimatedCpuPercent)}</div>
                  <div className="mt-0.5 font-mono text-xs text-text-secondary">
                    {formatBytes(item.estimatedMemoryBytes)}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </article>
      </section>
    </div>
  );
}

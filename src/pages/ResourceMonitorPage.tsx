import { Chart } from "@antv/g2";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button, Select } from "@/components/ui";
import type { ResourceCrateStatsDto, ResourceModuleStatsDto, ResourcePointDto, ResourceSnapshotDto } from "@/contracts";
import { useResourceMonitorStore, type ResourceSortMetric } from "@/stores/resource-monitor.store";
import { useThemeStore } from "@/theme/store";

const MAX_ATTRIBUTION_SNAPSHOTS = 1800;
const ATTRIBUTION_GROUP_LIMIT = 4;

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

function readCssToken(name: string, fallback: string): string {
  if (typeof window === "undefined") {
    return fallback;
  }
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
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

function formatSortMetricValue(metric: ResourceSortMetric, value: number): string {
  if (!Number.isFinite(value) || value < 0) {
    return "--";
  }
  if (metric === "cpu") {
    return `${value.toFixed(2)}%`;
  }
  if (metric === "memory") {
    return formatBytes(value);
  }
  return `${Math.round(value)}`;
}

function appendSnapshotSeries(history: ResourceSnapshotDto[], snapshot: ResourceSnapshotDto): ResourceSnapshotDto[] {
  const deduped = history.filter((item) => item.sampledAt !== snapshot.sampledAt);
  const next = [...deduped, snapshot].sort((left, right) => left.sampledAt - right.sampledAt);
  return next.slice(-MAX_ATTRIBUTION_SNAPSHOTS);
}

interface MetricCardProps {
  title: string;
  value: string;
  hint: string;
}

function MetricCard(props: MetricCardProps) {
  return (
    <article className="ui-glass-panel px-4 py-3">
      <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">{props.title}</div>
      <div className="mt-2 text-lg leading-none font-semibold text-text-primary">{props.value}</div>
      <div className="mt-1.5 text-xs text-text-secondary">{props.hint}</div>
    </article>
  );
}

interface HistoryChartDatum {
  time: string;
  value: number;
  metric: string;
  kind: "cpu" | "memory";
}

interface GroupedBarChartDatum {
  time: string;
  group: string;
  value: number;
}

interface ChartController<T> {
  update: (data: T[]) => void;
  destroy: () => void;
}

interface TooltipItem {
  label: string;
  value: string;
}

interface TooltipOptions<T> {
  series: boolean;
  shared: boolean;
  getTitle: (rows: T[]) => string;
  getItems: (rows: T[]) => TooltipItem[];
}

interface TooltipController {
  refresh: () => void;
  destroy: () => void;
}

function createTooltipController<T extends object>(
  chart: Chart,
  element: HTMLDivElement,
  options: TooltipOptions<T>,
): TooltipController {
  if (typeof window === "undefined") {
    return {
      refresh() {},
      destroy() {},
    };
  }

  element.style.position = "relative";
  const tooltip = document.createElement("div");
  tooltip.style.position = "absolute";
  tooltip.style.left = "0";
  tooltip.style.top = "0";
  tooltip.style.transform = "translate(-9999px, -9999px)";
  tooltip.style.zIndex = "10";
  tooltip.style.pointerEvents = "none";
  tooltip.style.border = `1px solid ${readColorToken("--color-border-glass")}`;
  tooltip.style.background = readColorToken("--color-surface-glass-strong");
  tooltip.style.color = readColorToken("--color-text-primary");
  tooltip.style.borderRadius = "8px";
  tooltip.style.padding = "8px 10px";
  tooltip.style.fontSize = "12px";
  tooltip.style.lineHeight = "1.4";
  tooltip.style.whiteSpace = "nowrap";
  tooltip.style.boxShadow = readCssToken("--shadow-overlay", "var(--shadow-overlay)");
  tooltip.style.display = "none";
  element.appendChild(tooltip);

  let lastPoint: { x: number; y: number } | null = null;

  const hide = () => {
    tooltip.style.display = "none";
    tooltip.style.transform = "translate(-9999px, -9999px)";
  };

  const updateTooltip = () => {
    if (!lastPoint) {
      hide();
      return;
    }
    const rows = chart.getDataByXY(
      { x: lastPoint.x, y: lastPoint.y },
      { series: options.series, shared: options.shared },
    ) as T[];
    if (!Array.isArray(rows) || rows.length === 0) {
      hide();
      return;
    }
    const items = options.getItems(rows);
    if (items.length === 0) {
      hide();
      return;
    }

    tooltip.replaceChildren();

    const titleEl = document.createElement("div");
    titleEl.style.fontWeight = "600";
    titleEl.style.marginBottom = "6px";
    titleEl.textContent = options.getTitle(rows);
    tooltip.appendChild(titleEl);

    for (const item of items) {
      const row = document.createElement("div");
      row.style.display = "flex";
      row.style.alignItems = "center";
      row.style.justifyContent = "space-between";
      row.style.gap = "10px";

      const labelEl = document.createElement("span");
      labelEl.style.opacity = "0.85";
      labelEl.textContent = item.label;
      row.appendChild(labelEl);

      const valueEl = document.createElement("span");
      valueEl.style.fontFamily = "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace";
      valueEl.textContent = item.value;
      row.appendChild(valueEl);

      tooltip.appendChild(row);
    }

    tooltip.style.display = "block";
    const width = tooltip.offsetWidth;
    const height = tooltip.offsetHeight;
    const maxX = Math.max(0, element.clientWidth - width - 6);
    const maxY = Math.max(0, element.clientHeight - height - 6);
    const nextX = Math.min(Math.max(6, lastPoint.x + 12), maxX);
    const nextY = Math.min(Math.max(6, lastPoint.y + 12), maxY);
    tooltip.style.transform = `translate(${nextX}px, ${nextY}px)`;
  };

  const onPointerMove = (event: PointerEvent) => {
    lastPoint = { x: event.offsetX, y: event.offsetY };
    updateTooltip();
  };

  const onPointerLeave = () => {
    lastPoint = null;
    hide();
  };

  element.addEventListener("pointermove", onPointerMove);
  element.addEventListener("pointerleave", onPointerLeave);

  return {
    refresh() {
      updateTooltip();
    },
    destroy() {
      element.removeEventListener("pointermove", onPointerMove);
      element.removeEventListener("pointerleave", onPointerLeave);
      tooltip.remove();
    },
  };
}

function buildHistoryChartData(
  points: ResourcePointDto[],
  locale: string,
  seriesLabel: { cpu: string; memory: string },
): HistoryChartDatum[] {
  return points.flatMap((point) => {
    const time = formatTime(point.sampledAt, locale);
    const next: HistoryChartDatum[] = [];
    if (point.processCpuPercent !== null) {
      next.push({
        time,
        value: point.processCpuPercent,
        metric: seriesLabel.cpu,
        kind: "cpu",
      });
    }
    if (point.processMemoryBytes !== null) {
      next.push({
        time,
        value: point.processMemoryBytes / 1024 / 1024,
        metric: seriesLabel.memory,
        kind: "memory",
      });
    }
    return next;
  });
}

function createHistoryChart(element: HTMLDivElement, data: HistoryChartDatum[]): ChartController<HistoryChartDatum> {
  const accent = readColorToken("--color-accent");
  const info = readColorToken("--color-info");

  const chart = new Chart({
    container: element,
    autoFit: true,
    height: 280,
  });
  chart.animate(false);
  chart.interaction("tooltip", false);

  const line = chart
    .line()
    .data(data)
    .encode("x", "time")
    .encode("y", "value")
    .encode("color", "metric")
    .scale("color", {
      range: [accent, info],
    })
    .style("lineWidth", 2)
    .animate(false)
    .tooltip(false);

  chart.render();
  const tooltip = createTooltipController<HistoryChartDatum>(chart, element, {
    series: true,
    shared: true,
    getTitle(rows) {
      return rows[0]?.time ?? "";
    },
    getItems(rows) {
      return rows.map((row) => ({
        label: row.metric,
        value: row.kind === "cpu" ? `${row.value.toFixed(2)}%` : `${row.value.toFixed(2)} MB`,
      }));
    },
  });

  return {
    update(nextData) {
      void line.changeData(nextData).then(() => {
        tooltip.refresh();
      });
    },
    destroy() {
      tooltip.destroy();
      chart.destroy();
    },
  };
}

function createGroupedBarChart(
  element: HTMLDivElement,
  data: GroupedBarChartDatum[],
  valueFormatter: (value: number) => string,
  height = 280,
): ChartController<GroupedBarChartDatum> {
  const palette = [
    readColorToken("--color-accent"),
    readColorToken("--color-info"),
    readColorToken("--color-success"),
    readColorToken("--color-warning"),
    readColorToken("--color-danger"),
  ];

  const chart = new Chart({
    container: element,
    autoFit: true,
    height,
  });
  chart.animate(false);
  chart.interaction("tooltip", false);

  const interval = chart
    .interval()
    .data(data)
    .encode("x", "time")
    .encode("y", "value")
    .encode("color", "group")
    .transform({ type: "dodgeX" })
    .scale("color", {
      range: palette,
    })
    .style("maxWidth", 32)
    .animate(false)
    .tooltip(false);

  chart.render();
  const tooltip = createTooltipController<GroupedBarChartDatum>(chart, element, {
    series: false,
    shared: true,
    getTitle(rows) {
      return rows[0]?.time ?? "";
    },
    getItems(rows) {
      return rows.map((row) => ({
        label: row.group,
        value: valueFormatter(row.value),
      }));
    },
  });

  return {
    update(nextData) {
      void interval.changeData(nextData).then(() => {
        tooltip.refresh();
      });
    },
    destroy() {
      tooltip.destroy();
      chart.destroy();
    },
  };
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
  const historyControllerRef = useRef<ChartController<HistoryChartDatum> | null>(null);
  const moduleControllerRef = useRef<ChartController<GroupedBarChartDatum> | null>(null);
  const crateControllerRef = useRef<ChartController<GroupedBarChartDatum> | null>(null);
  const [attributionSnapshots, setAttributionSnapshots] = useState<ResourceSnapshotDto[]>([]);

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

  useEffect(() => {
    if (!snapshot) {
      return;
    }
    setAttributionSnapshots((current) => appendSnapshotSeries(current, snapshot));
  }, [snapshot]);

  const visibleAttributionSnapshots = useMemo(() => {
    const cutoff = Date.now() - historyWindowMinutes * 60 * 1000;
    return attributionSnapshots.filter((entry) => entry.sampledAt >= cutoff);
  }, [attributionSnapshots, historyWindowMinutes]);

  const moduleStats = useMemo(() => {
    const modules = snapshot?.modules ?? [];
    return [...modules].sort((left, right) => pickSortValue(sortMetric, right) - pickSortValue(sortMetric, left));
  }, [snapshot?.modules, sortMetric]);

  const historyChartData = useMemo(
    () =>
      buildHistoryChartData(visibleHistory, locale, {
        cpu: t("chart.series.cpu"),
        memory: t("chart.series.memoryMb"),
      }),
    [locale, t, visibleHistory],
  );

  const moduleTopIds = useMemo(() => {
    const latest = visibleAttributionSnapshots[visibleAttributionSnapshots.length - 1];
    if (!latest) {
      return [];
    }
    return [...latest.modules]
      .filter((item) => pickSortValue(sortMetric, item) >= 0)
      .sort((left, right) => pickSortValue(sortMetric, right) - pickSortValue(sortMetric, left))
      .slice(0, ATTRIBUTION_GROUP_LIMIT)
      .map((item) => item.moduleId);
  }, [sortMetric, visibleAttributionSnapshots]);

  const moduleChartData = useMemo(() => {
    if (moduleTopIds.length === 0 || visibleAttributionSnapshots.length === 0) {
      return [];
    }
    return visibleAttributionSnapshots.flatMap((entry) => {
      const byId = new Map(entry.modules.map((item) => [item.moduleId, item]));
      const time = formatTime(entry.sampledAt, locale);
      return moduleTopIds.map((moduleId) => {
        const current = byId.get(moduleId);
        const value = current ? pickSortValue(sortMetric, current) : 0;
        return {
          time,
          group: t(`module.${moduleId}`),
          value: value < 0 ? 0 : value,
        };
      });
    });
  }, [locale, moduleTopIds, sortMetric, t, visibleAttributionSnapshots]);

  const crateTopIds = useMemo(() => {
    const latest = visibleAttributionSnapshots[visibleAttributionSnapshots.length - 1];
    if (!latest) {
      return [];
    }
    return [...latest.crates]
      .filter((item) => pickSortValue(sortMetric, item) >= 0)
      .sort((left, right) => pickSortValue(sortMetric, right) - pickSortValue(sortMetric, left))
      .slice(0, ATTRIBUTION_GROUP_LIMIT)
      .map((item) => item.crateId);
  }, [sortMetric, visibleAttributionSnapshots]);

  const crateChartData = useMemo(() => {
    if (crateTopIds.length === 0 || visibleAttributionSnapshots.length === 0) {
      return [];
    }
    return visibleAttributionSnapshots.flatMap((entry) => {
      const byId = new Map(entry.crates.map((item) => [item.crateId, item]));
      const time = formatTime(entry.sampledAt, locale);
      return crateTopIds.map((crateId) => {
        const current = byId.get(crateId);
        const value = current ? pickSortValue(sortMetric, current) : 0;
        return {
          time,
          group: t(`crate.${crateId}`),
          value: value < 0 ? 0 : value,
        };
      });
    });
  }, [crateTopIds, locale, sortMetric, t, visibleAttributionSnapshots]);

  useEffect(() => {
    historyControllerRef.current?.destroy();
    historyControllerRef.current = null;
    moduleControllerRef.current?.destroy();
    moduleControllerRef.current = null;
    crateControllerRef.current?.destroy();
    crateControllerRef.current = null;
  }, [resolvedTheme]);

  useEffect(() => {
    moduleControllerRef.current?.destroy();
    moduleControllerRef.current = null;
    crateControllerRef.current?.destroy();
    crateControllerRef.current = null;
  }, [sortMetric]);

  useEffect(
    () => () => {
      historyControllerRef.current?.destroy();
      historyControllerRef.current = null;
      moduleControllerRef.current?.destroy();
      moduleControllerRef.current = null;
      crateControllerRef.current?.destroy();
      crateControllerRef.current = null;
    },
    [],
  );

  useEffect(() => {
    const container = historyChartRef.current;
    if (!container) {
      return;
    }
    if (historyChartData.length === 0) {
      historyControllerRef.current?.destroy();
      historyControllerRef.current = null;
      container.innerHTML = "";
      return;
    }
    if (!historyControllerRef.current) {
      historyControllerRef.current = createHistoryChart(container, historyChartData);
      return;
    }
    historyControllerRef.current.update(historyChartData);
  }, [historyChartData]);

  useEffect(() => {
    const container = moduleChartRef.current;
    if (!container) {
      return;
    }
    if (moduleChartData.length === 0) {
      moduleControllerRef.current?.destroy();
      moduleControllerRef.current = null;
      container.innerHTML = "";
      return;
    }
    if (!moduleControllerRef.current) {
      moduleControllerRef.current = createGroupedBarChart(container, moduleChartData, (value) =>
        formatSortMetricValue(sortMetric, value),
      );
      return;
    }
    moduleControllerRef.current.update(moduleChartData);
  }, [moduleChartData, sortMetric]);

  useEffect(() => {
    const container = crateChartRef.current;
    if (!container) {
      return;
    }
    if (crateChartData.length === 0) {
      crateControllerRef.current?.destroy();
      crateControllerRef.current = null;
      container.innerHTML = "";
      return;
    }
    if (!crateControllerRef.current) {
      crateControllerRef.current = createGroupedBarChart(
        container,
        crateChartData,
        (value) => formatSortMetricValue(sortMetric, value),
        240,
      );
      return;
    }
    crateControllerRef.current.update(crateChartData);
  }, [crateChartData, sortMetric]);

  const overview = snapshot?.overview;
  const memoryUsageHint =
    overview?.systemUsedMemoryBytes !== null && overview?.systemTotalMemoryBytes !== null
      ? `${formatBytes(overview?.systemUsedMemoryBytes ?? null)} / ${formatBytes(overview?.systemTotalMemoryBytes ?? null)}`
      : "--";
  const activeModuleCount = moduleStats.filter((item) => item.calls > 0).length;

  return (
    <div className="space-y-3 pb-2">
      <section className="ui-glass-panel-strong rounded-2xl p-4">
        <div className="font-mono ui-text-micro uppercase tracking-ui-wider text-text-muted">
          rtool / resource monitor
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <h1 className="m-0 text-xl font-semibold tracking-tight text-text-primary">{t("header.title")}</h1>
          <span className="ui-glass-chip px-2 py-0.5 font-mono ui-text-micro uppercase tracking-ui-wide text-accent">
            {loading && !initialized ? t("status.booting") : error ? t("status.degraded") : t("status.online")}
          </span>
        </div>
        <p className="mt-2 text-sm text-text-secondary">{t("header.subtitle")}</p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Button size="default" variant="secondary" onClick={() => void refreshAll()}>
            <span className="btn-icon i-noto:counterclockwise-arrows-button" aria-hidden="true" />
            <span>{t("action.refreshNow")}</span>
          </Button>
          <Button
            size="default"
            variant="danger"
            onClick={() => {
              setAttributionSnapshots([]);
              void resetSession();
            }}
          >
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
        <article className="ui-glass-panel p-4">
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

        <article className="ui-glass-panel p-4">
          <header className="mb-3">
            <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
              {t("panel.modules.title")}
            </div>
            <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.modules.subtitle")}</h2>
          </header>
          {moduleChartData.length === 0 ? (
            <div className="py-10 text-center text-sm text-text-muted">{t("chart.noData")}</div>
          ) : (
            <div ref={moduleChartRef} className="h-[280px] w-full" />
          )}
        </article>
      </section>

      <section className="grid grid-cols-1 gap-3 xl:grid-cols-[1.15fr_1fr]">
        <article className="ui-glass-panel p-4">
          <header className="mb-3">
            <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
              {t("panel.crates.title")}
            </div>
            <h2 className="mt-1 text-sm font-semibold text-text-primary">{t("panel.crates.subtitle")}</h2>
          </header>
          {crateChartData.length === 0 ? (
            <div className="py-10 text-center text-sm text-text-muted">{t("chart.noData")}</div>
          ) : (
            <div ref={crateChartRef} className="h-[240px] w-full" />
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
            {moduleStats.slice(0, 6).map((item) => (
              <div
                key={item.moduleId}
                className="flex items-start justify-between gap-3 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-2 shadow-inset-soft"
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

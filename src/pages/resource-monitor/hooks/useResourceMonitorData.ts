import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  ResourceCrateIdDto,
  ResourceCrateStatsDto,
  ResourceModuleIdDto,
  ResourceModuleStatsDto,
  ResourceOverviewDto,
  ResourcePointDto,
  ResourceSnapshotDto,
} from "@/contracts";
import { useResourceMonitorStore, type ResourceSortMetric } from "@/stores/resource-monitor.store";

import type { GroupedBarChartDatum, HistoryChartDatum } from "../charts";

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

function buildModuleChartData(
  snapshots: ResourceSnapshotDto[],
  moduleTopIds: ResourceModuleIdDto[],
  sortMetric: ResourceSortMetric,
  locale: string,
  getLabel: (moduleId: ResourceModuleIdDto) => string,
): GroupedBarChartDatum[] {
  if (moduleTopIds.length === 0 || snapshots.length === 0) {
    return [];
  }

  return snapshots.flatMap((entry) => {
    const byId = new Map(entry.modules.map((item) => [item.moduleId, item]));
    const time = formatTime(entry.sampledAt, locale);
    return moduleTopIds.map((moduleId) => {
      const current = byId.get(moduleId);
      const value = current ? pickSortValue(sortMetric, current) : 0;
      return {
        time,
        group: getLabel(moduleId),
        value: value < 0 ? 0 : value,
      };
    });
  });
}

function buildCrateChartData(
  snapshots: ResourceSnapshotDto[],
  crateTopIds: ResourceCrateIdDto[],
  sortMetric: ResourceSortMetric,
  locale: string,
  getLabel: (crateId: ResourceCrateIdDto) => string,
): GroupedBarChartDatum[] {
  if (crateTopIds.length === 0 || snapshots.length === 0) {
    return [];
  }

  return snapshots.flatMap((entry) => {
    const byId = new Map(entry.crates.map((item) => [item.crateId, item]));
    const time = formatTime(entry.sampledAt, locale);
    return crateTopIds.map((crateId) => {
      const current = byId.get(crateId);
      const value = current ? pickSortValue(sortMetric, current) : 0;
      return {
        time,
        group: getLabel(crateId),
        value: value < 0 ? 0 : value,
      };
    });
  });
}

export interface ResourceMetricCard {
  title: string;
  value: string;
  hint: string;
}

export interface ResourceAnalysisRow {
  moduleId: ResourceModuleIdDto;
  moduleLabel: string;
  detailText: string;
  cpuText: string;
  memoryText: string;
}

export interface ResourceMetaText {
  sampledAtText: string;
  lastUpdatedText: string;
  historyPointsText: string;
}

export interface UseResourceMonitorDataResult {
  initialized: boolean;
  loading: boolean;
  error: string | null;
  overview: ResourceOverviewDto | null;
  historyWindowMinutes: 5 | 15 | 30;
  sortMetric: ResourceSortMetric;
  visibleHistory: ResourcePointDto[];
  historyChartData: HistoryChartDatum[];
  moduleChartData: GroupedBarChartDatum[];
  crateChartData: GroupedBarChartDatum[];
  metricCards: ResourceMetricCard[];
  analysisRows: ResourceAnalysisRow[];
  meta: ResourceMetaText;
  refreshAll: () => Promise<void>;
  setHistoryWindowMinutes: (value: 5 | 15 | 30) => void;
  setSortMetric: (value: ResourceSortMetric) => void;
  resetSessionWithLocalClear: () => Promise<void>;
  formatCurrentSortMetricValue: (value: number) => string;
}

export function useResourceMonitorData(): UseResourceMonitorDataResult {
  const { t, i18n } = useTranslation(["resource_monitor"]);
  const locale = i18n.resolvedLanguage ?? i18n.language;

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
      return [] as ResourceModuleIdDto[];
    }
    return [...latest.modules]
      .filter((item) => pickSortValue(sortMetric, item) >= 0)
      .sort((left, right) => pickSortValue(sortMetric, right) - pickSortValue(sortMetric, left))
      .slice(0, ATTRIBUTION_GROUP_LIMIT)
      .map((item) => item.moduleId);
  }, [sortMetric, visibleAttributionSnapshots]);

  const moduleChartData = useMemo(
    () =>
      buildModuleChartData(visibleAttributionSnapshots, moduleTopIds, sortMetric, locale, (moduleId) =>
        t(`module.${moduleId}`),
      ),
    [locale, moduleTopIds, sortMetric, t, visibleAttributionSnapshots],
  );

  const crateTopIds = useMemo(() => {
    const latest = visibleAttributionSnapshots[visibleAttributionSnapshots.length - 1];
    if (!latest) {
      return [] as ResourceCrateIdDto[];
    }
    return [...latest.crates]
      .filter((item) => pickSortValue(sortMetric, item) >= 0)
      .sort((left, right) => pickSortValue(sortMetric, right) - pickSortValue(sortMetric, left))
      .slice(0, ATTRIBUTION_GROUP_LIMIT)
      .map((item) => item.crateId);
  }, [sortMetric, visibleAttributionSnapshots]);

  const crateChartData = useMemo(
    () =>
      buildCrateChartData(visibleAttributionSnapshots, crateTopIds, sortMetric, locale, (crateId) => t(`crate.${crateId}`)),
    [crateTopIds, locale, sortMetric, t, visibleAttributionSnapshots],
  );

  const overview = snapshot?.overview ?? null;

  const metricCards = useMemo<ResourceMetricCard[]>(() => {
    const memoryUsageHint =
      overview?.systemUsedMemoryBytes !== null && overview?.systemTotalMemoryBytes !== null
        ? `${formatBytes(overview?.systemUsedMemoryBytes ?? null)} / ${formatBytes(overview?.systemTotalMemoryBytes ?? null)}`
        : "--";

    const activeModuleCount = moduleStats.filter((item) => item.calls > 0).length;
    const launcherCalls = moduleStats.find((item) => item.moduleId === "launcher")?.calls ?? 0;
    const launcherIndexCalls = moduleStats.find((item) => item.moduleId === "launcher_index")?.calls ?? 0;
    const launcherFallbackCalls = moduleStats.find((item) => item.moduleId === "launcher_fallback")?.calls ?? 0;
    const indexHitRate = launcherCalls > 0 ? (launcherIndexCalls / launcherCalls) * 100 : null;
    const fallbackRate = launcherCalls > 0 ? (launcherFallbackCalls / launcherCalls) * 100 : null;

    return [
      {
        title: t("metric.processCpu.title"),
        value: formatPercent(overview?.processCpuPercent ?? null),
        hint: t("metric.processCpu.hint"),
      },
      {
        title: t("metric.processMemory.title"),
        value: formatBytes(overview?.processMemoryBytes ?? null),
        hint: t("metric.processMemory.hint"),
      },
      {
        title: t("metric.systemMemory.title"),
        value: memoryUsageHint,
        hint: t("metric.systemMemory.hint"),
      },
      {
        title: t("metric.indexHitRate.title"),
        value: formatPercent(indexHitRate),
        hint: t("metric.indexHitRate.hint"),
      },
      {
        title: t("metric.fallbackRate.title"),
        value: formatPercent(fallbackRate),
        hint: t("metric.fallbackRate.hint"),
      },
      {
        title: t("metric.activeModules.title"),
        value: `${activeModuleCount}`,
        hint: t("metric.activeModules.hint"),
      },
    ];
  }, [moduleStats, overview?.processCpuPercent, overview?.processMemoryBytes, overview?.systemTotalMemoryBytes, overview?.systemUsedMemoryBytes, t]);

  const analysisRows = useMemo<ResourceAnalysisRow[]>(
    () =>
      moduleStats.slice(0, 6).map((item) => ({
        moduleId: item.moduleId,
        moduleLabel: t(`module.${item.moduleId}`),
        detailText: t("analysis.row", {
          calls: item.calls,
          errors: item.errorCalls,
          avg: item.avgDurationMs ?? 0,
        }),
        cpuText: formatPercent(item.estimatedCpuPercent),
        memoryText: formatBytes(item.estimatedMemoryBytes),
      })),
    [moduleStats, t],
  );

  const meta = useMemo<ResourceMetaText>(
    () => ({
      sampledAtText: t("meta.sampledAt", {
        value: overview?.sampledAt ? formatTime(overview.sampledAt, locale) : "--",
      }),
      lastUpdatedText: t("meta.lastUpdated", {
        value: lastUpdatedAt ? formatTime(lastUpdatedAt, locale) : "--",
      }),
      historyPointsText: t("meta.historyPoints", { value: visibleHistory.length }),
    }),
    [lastUpdatedAt, locale, overview?.sampledAt, t, visibleHistory.length],
  );

  const resetSessionWithLocalClear = useCallback(async () => {
    setAttributionSnapshots([]);
    await resetSession();
  }, [resetSession]);

  const formatCurrentSortMetricValue = useCallback(
    (value: number) => formatSortMetricValue(sortMetric, value),
    [sortMetric],
  );

  return {
    initialized,
    loading,
    error,
    overview,
    historyWindowMinutes,
    sortMetric,
    visibleHistory,
    historyChartData,
    moduleChartData,
    crateChartData,
    metricCards,
    analysisRows,
    meta,
    refreshAll,
    setHistoryWindowMinutes,
    setSortMetric,
    resetSessionWithLocalClear,
    formatCurrentSortMetricValue,
  };
}

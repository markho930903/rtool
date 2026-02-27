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
import { useResourceMonitorStore } from "@/stores/resource-monitor.store";

import type { GroupedBarChartDatum, HistoryChartDatum } from "../charts";

const MAX_ATTRIBUTION_SNAPSHOTS = 1800;
const ATTRIBUTION_GROUP_LIMIT = 4;
const MAX_HISTORY_SERIES_POINTS = 600;
const MAX_ATTRIBUTION_CHART_BUCKETS = 120;

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

function pickMemorySortValue(item: ResourceModuleStatsDto | ResourceCrateStatsDto): number {
  return item.estimatedMemoryBytes ?? -1;
}

function formatMemorySortValue(value: number): string {
  if (!Number.isFinite(value) || value < 0) {
    return "--";
  }
  return formatBytes(value);
}

function appendSnapshotSeries(history: ResourceSnapshotDto[], snapshot: ResourceSnapshotDto): ResourceSnapshotDto[] {
  const deduped = history.filter((item) => item.sampledAt !== snapshot.sampledAt);
  const next = [...deduped, snapshot].sort((left, right) => left.sampledAt - right.sampledAt);
  return next.slice(-MAX_ATTRIBUTION_SNAPSHOTS);
}

interface NumericSeriesPoint {
  sampledAt: number;
  value: number;
}

function downsampleSeriesLttb(points: NumericSeriesPoint[], threshold: number): NumericSeriesPoint[] {
  if (threshold <= 2 || points.length <= threshold) {
    return points;
  }

  const sampled: NumericSeriesPoint[] = [points[0]];
  const every = (points.length - 2) / (threshold - 2);
  let a = 0;

  for (let i = 0; i < threshold - 2; i += 1) {
    const avgRangeStart = Math.floor((i + 1) * every) + 1;
    const avgRangeEnd = Math.min(Math.floor((i + 2) * every) + 1, points.length);

    let avgX: number;
    let avgY: number;
    if (avgRangeEnd <= avgRangeStart) {
      const fallback = points[Math.min(avgRangeStart, points.length - 1)];
      avgX = fallback.sampledAt;
      avgY = fallback.value;
    } else {
      avgX = 0;
      avgY = 0;
      const avgRangeLength = avgRangeEnd - avgRangeStart;
      for (let index = avgRangeStart; index < avgRangeEnd; index += 1) {
        avgX += points[index].sampledAt;
        avgY += points[index].value;
      }
      avgX /= avgRangeLength;
      avgY /= avgRangeLength;
    }

    const rangeStart = Math.floor(i * every) + 1;
    const rangeEnd = Math.min(Math.floor((i + 1) * every) + 1, points.length - 1);
    if (rangeStart >= rangeEnd) {
      sampled.push(points[Math.min(rangeStart, points.length - 1)]);
      a = Math.min(rangeStart, points.length - 1);
      continue;
    }

    let nextIndex = rangeStart;
    let maxArea = -1;
    for (let index = rangeStart; index < rangeEnd; index += 1) {
      const area =
        Math.abs(
          (points[a].sampledAt - avgX) * (points[index].value - points[a].value) -
            (points[a].sampledAt - points[index].sampledAt) * (avgY - points[a].value),
        ) * 0.5;

      if (area > maxArea) {
        maxArea = area;
        nextIndex = index;
      }
    }

    sampled.push(points[nextIndex]);
    a = nextIndex;
  }

  sampled.push(points[points.length - 1]);
  return sampled;
}

function splitIntoBuckets<T>(items: T[], maxBuckets: number): T[][] {
  if (items.length === 0) {
    return [];
  }
  if (items.length <= maxBuckets) {
    return items.map((item) => [item]);
  }

  const bucketSize = Math.ceil(items.length / maxBuckets);
  const buckets: T[][] = [];
  for (let index = 0; index < items.length; index += bucketSize) {
    buckets.push(items.slice(index, index + bucketSize));
  }
  return buckets;
}

function buildHistoryChartData(
  points: ResourcePointDto[],
  formatTimeLabel: (value: number) => string,
  seriesLabel: { cpu: string; memory: string },
): HistoryChartDatum[] {
  const cpuSeries = points
    .filter((point) => point.processCpuPercent !== null)
    .map((point) => ({
      sampledAt: point.sampledAt,
      value: point.processCpuPercent ?? 0,
    }));
  const memorySeries = points
    .filter((point) => point.processMemoryBytes !== null)
    .map((point) => ({
      sampledAt: point.sampledAt,
      value: (point.processMemoryBytes ?? 0) / 1024 / 1024,
    }));

  const sampledCpuSeries = downsampleSeriesLttb(cpuSeries, MAX_HISTORY_SERIES_POINTS);
  const sampledMemorySeries = downsampleSeriesLttb(memorySeries, MAX_HISTORY_SERIES_POINTS);
  const timeLabelCache = new Map<number, string>();
  const resolveTimeLabel = (value: number): string => {
    const cached = timeLabelCache.get(value);
    if (cached) {
      return cached;
    }
    const next = formatTimeLabel(value);
    timeLabelCache.set(value, next);
    return next;
  };

  return [
    ...sampledCpuSeries.map((point) => ({
      time: resolveTimeLabel(point.sampledAt),
      value: point.value,
      metric: seriesLabel.cpu,
      kind: "cpu" as const,
    })),
    ...sampledMemorySeries.map((point) => ({
      time: resolveTimeLabel(point.sampledAt),
      value: point.value,
      metric: seriesLabel.memory,
      kind: "memory" as const,
    })),
  ];
}

function buildModuleChartData(
  snapshots: ResourceSnapshotDto[],
  moduleTopIds: ResourceModuleIdDto[],
  formatTimeLabel: (value: number) => string,
  getLabel: (moduleId: ResourceModuleIdDto) => string,
): GroupedBarChartDatum[] {
  if (moduleTopIds.length === 0 || snapshots.length === 0) {
    return [];
  }

  const buckets = splitIntoBuckets(snapshots, MAX_ATTRIBUTION_CHART_BUCKETS);
  return buckets.flatMap((bucket) => {
    const maxById = new Map<ResourceModuleIdDto, number>(moduleTopIds.map((moduleId) => [moduleId, 0]));

    for (const entry of bucket) {
      const byId = new Map(entry.modules.map((item) => [item.moduleId, item]));
      for (const moduleId of moduleTopIds) {
        const current = byId.get(moduleId);
        const value = current ? pickMemorySortValue(current) : 0;
        const normalized = value < 0 ? 0 : value;
        if (normalized > (maxById.get(moduleId) ?? 0)) {
          maxById.set(moduleId, normalized);
        }
      }
    }

    const time = formatTimeLabel(bucket[bucket.length - 1].sampledAt);
    return moduleTopIds.map((moduleId) => ({
      time,
      group: getLabel(moduleId),
      value: maxById.get(moduleId) ?? 0,
    }));
  });
}

function buildCrateChartData(
  snapshots: ResourceSnapshotDto[],
  crateTopIds: ResourceCrateIdDto[],
  formatTimeLabel: (value: number) => string,
  getLabel: (crateId: ResourceCrateIdDto) => string,
): GroupedBarChartDatum[] {
  if (crateTopIds.length === 0 || snapshots.length === 0) {
    return [];
  }

  const buckets = splitIntoBuckets(snapshots, MAX_ATTRIBUTION_CHART_BUCKETS);
  return buckets.flatMap((bucket) => {
    const maxById = new Map<ResourceCrateIdDto, number>(crateTopIds.map((crateId) => [crateId, 0]));

    for (const entry of bucket) {
      const byId = new Map(entry.crates.map((item) => [item.crateId, item]));
      for (const crateId of crateTopIds) {
        const current = byId.get(crateId);
        const value = current ? pickMemorySortValue(current) : 0;
        const normalized = value < 0 ? 0 : value;
        if (normalized > (maxById.get(crateId) ?? 0)) {
          maxById.set(crateId, normalized);
        }
      }
    }

    const time = formatTimeLabel(bucket[bucket.length - 1].sampledAt);
    return crateTopIds.map((crateId) => ({
      time,
      group: getLabel(crateId),
      value: maxById.get(crateId) ?? 0,
    }));
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
  visibleHistory: ResourcePointDto[];
  historyChartData: HistoryChartDatum[];
  moduleChartData: GroupedBarChartDatum[];
  crateChartData: GroupedBarChartDatum[];
  metricCards: ResourceMetricCard[];
  analysisRows: ResourceAnalysisRow[];
  meta: ResourceMetaText;
  refreshAll: () => Promise<void>;
  setHistoryWindowMinutes: (value: 5 | 15 | 30) => void;
  resetSessionWithLocalClear: () => Promise<void>;
  formatCurrentSortMetricValue: (value: number) => string;
}

export function useResourceMonitorData(): UseResourceMonitorDataResult {
  const { t, i18n } = useTranslation(["resource_monitor"]);
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const timeFormatter = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      }),
    [locale],
  );
  const formatTimeLabel = useCallback((value: number) => timeFormatter.format(new Date(value)), [timeFormatter]);

  const initialized = useResourceMonitorStore((state) => state.initialized);
  const loading = useResourceMonitorStore((state) => state.loading);
  const error = useResourceMonitorStore((state) => state.error);
  const snapshot = useResourceMonitorStore((state) => state.snapshot);
  const history = useResourceMonitorStore((state) => state.history);
  const lastUpdatedAt = useResourceMonitorStore((state) => state.lastUpdatedAt);
  const historyWindowMinutes = useResourceMonitorStore((state) => state.historyWindowMinutes);
  const initialize = useResourceMonitorStore((state) => state.initialize);
  const refreshAll = useResourceMonitorStore((state) => state.refreshAll);
  const startPolling = useResourceMonitorStore((state) => state.startPolling);
  const stopPolling = useResourceMonitorStore((state) => state.stopPolling);
  const setHistoryWindowMinutes = useResourceMonitorStore((state) => state.setHistoryWindowMinutes);
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
    return [...modules].sort((left, right) => pickMemorySortValue(right) - pickMemorySortValue(left));
  }, [snapshot?.modules]);

  const historyChartData = useMemo(
    () =>
      buildHistoryChartData(visibleHistory, formatTimeLabel, {
        cpu: t("chart.series.cpu"),
        memory: t("chart.series.memoryMb"),
      }),
    [formatTimeLabel, t, visibleHistory],
  );

  const moduleTopIds = useMemo(() => {
    const latest = visibleAttributionSnapshots[visibleAttributionSnapshots.length - 1];
    if (!latest) {
      return [] as ResourceModuleIdDto[];
    }
    return [...latest.modules]
      .filter((item) => pickMemorySortValue(item) >= 0)
      .sort((left, right) => pickMemorySortValue(right) - pickMemorySortValue(left))
      .slice(0, ATTRIBUTION_GROUP_LIMIT)
      .map((item) => item.moduleId);
  }, [visibleAttributionSnapshots]);

  const moduleChartData = useMemo(
    () =>
      buildModuleChartData(visibleAttributionSnapshots, moduleTopIds, formatTimeLabel, (moduleId) =>
        t(`module.${moduleId}`),
      ),
    [formatTimeLabel, moduleTopIds, t, visibleAttributionSnapshots],
  );

  const crateTopIds = useMemo(() => {
    const latest = visibleAttributionSnapshots[visibleAttributionSnapshots.length - 1];
    if (!latest) {
      return [] as ResourceCrateIdDto[];
    }
    return [...latest.crates]
      .filter((item) => pickMemorySortValue(item) >= 0)
      .sort((left, right) => pickMemorySortValue(right) - pickMemorySortValue(left))
      .slice(0, ATTRIBUTION_GROUP_LIMIT)
      .map((item) => item.crateId);
  }, [visibleAttributionSnapshots]);

  const crateChartData = useMemo(
    () =>
      buildCrateChartData(visibleAttributionSnapshots, crateTopIds, formatTimeLabel, (crateId) =>
        t(`crate.${crateId}`),
      ),
    [crateTopIds, formatTimeLabel, t, visibleAttributionSnapshots],
  );

  const overview = snapshot?.overview ?? null;

  const metricCards = useMemo<ResourceMetricCard[]>(() => {
    const moduleCalls = (moduleId: ResourceModuleIdDto): number =>
      moduleStats.find((item) => item.moduleId === moduleId)?.calls ?? 0;

    const memoryUsageHint =
      overview?.systemUsedMemoryBytes !== null && overview?.systemTotalMemoryBytes !== null
        ? `${formatBytes(overview?.systemUsedMemoryBytes ?? null)} / ${formatBytes(overview?.systemTotalMemoryBytes ?? null)}`
        : "--";

    const activeModuleCount = moduleStats.filter((item) => item.calls > 0).length;
    const launcherCalls = moduleCalls("launcher");
    const launcherIndexCalls = moduleCalls("launcher_index");
    const indexHitRate = launcherCalls > 0 ? (launcherIndexCalls / launcherCalls) * 100 : null;

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
        title: t("metric.activeModules.title"),
        value: `${activeModuleCount}`,
        hint: t("metric.activeModules.hint"),
      },
    ];
  }, [
    moduleStats,
    overview?.processCpuPercent,
    overview?.processMemoryBytes,
    overview?.systemTotalMemoryBytes,
    overview?.systemUsedMemoryBytes,
    t,
  ]);

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
        value: overview?.sampledAt ? formatTimeLabel(overview.sampledAt) : "--",
      }),
      lastUpdatedText: t("meta.lastUpdated", {
        value: lastUpdatedAt ? formatTimeLabel(lastUpdatedAt) : "--",
      }),
      historyPointsText: t("meta.historyPoints", { value: visibleHistory.length }),
    }),
    [formatTimeLabel, lastUpdatedAt, overview?.sampledAt, t, visibleHistory.length],
  );

  const resetSessionWithLocalClear = useCallback(async () => {
    setAttributionSnapshots([]);
    await resetSession();
  }, [resetSession]);

  const formatCurrentSortMetricValue = useCallback((value: number) => formatMemorySortValue(value), []);

  return {
    initialized,
    loading,
    error,
    overview,
    historyWindowMinutes,
    visibleHistory,
    historyChartData,
    moduleChartData,
    crateChartData,
    metricCards,
    analysisRows,
    meta,
    refreshAll,
    setHistoryWindowMinutes,
    resetSessionWithLocalClear,
    formatCurrentSortMetricValue,
  };
}

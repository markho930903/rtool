import { useEffect, useRef } from "react";

import {
  createGroupedBarChart,
  createHistoryChart,
  type ChartController,
  type GroupedBarChartDatum,
  type HistoryChartDatum,
} from "@/pages/resource-monitor/charts";
import ResourceAttributionSection from "@/pages/resource-monitor/components/ResourceAttributionSection";
import ResourceMetricsSection from "@/pages/resource-monitor/components/ResourceMetricsSection";
import ResourceMonitorHeader from "@/pages/resource-monitor/components/ResourceMonitorHeader";
import ResourceTrendSection from "@/pages/resource-monitor/components/ResourceTrendSection";
import { useResourceMonitorData } from "@/pages/resource-monitor/hooks/useResourceMonitorData";
import { useThemeStore } from "@/theme/store";

export default function ResourceMonitorPage() {
  const resolvedTheme = useThemeStore((state) => state.resolved);
  const monitor = useResourceMonitorData();

  const historyChartRef = useRef<HTMLDivElement | null>(null);
  const moduleChartRef = useRef<HTMLDivElement | null>(null);
  const crateChartRef = useRef<HTMLDivElement | null>(null);

  const historyControllerRef = useRef<ChartController<HistoryChartDatum> | null>(null);
  const moduleControllerRef = useRef<ChartController<GroupedBarChartDatum> | null>(null);
  const crateControllerRef = useRef<ChartController<GroupedBarChartDatum> | null>(null);

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
  }, [monitor.sortMetric]);

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

    if (monitor.historyChartData.length === 0) {
      historyControllerRef.current?.destroy();
      historyControllerRef.current = null;
      container.innerHTML = "";
      return;
    }

    if (!historyControllerRef.current) {
      historyControllerRef.current = createHistoryChart(container, monitor.historyChartData);
      return;
    }

    historyControllerRef.current.update(monitor.historyChartData);
  }, [monitor.historyChartData]);

  useEffect(() => {
    const container = moduleChartRef.current;
    if (!container) {
      return;
    }

    if (monitor.moduleChartData.length === 0) {
      moduleControllerRef.current?.destroy();
      moduleControllerRef.current = null;
      container.innerHTML = "";
      return;
    }

    if (!moduleControllerRef.current) {
      moduleControllerRef.current = createGroupedBarChart(
        container,
        monitor.moduleChartData,
        monitor.formatCurrentSortMetricValue,
      );
      return;
    }

    moduleControllerRef.current.update(monitor.moduleChartData);
  }, [monitor.formatCurrentSortMetricValue, monitor.moduleChartData]);

  useEffect(() => {
    const container = crateChartRef.current;
    if (!container) {
      return;
    }

    if (monitor.crateChartData.length === 0) {
      crateControllerRef.current?.destroy();
      crateControllerRef.current = null;
      container.innerHTML = "";
      return;
    }

    if (!crateControllerRef.current) {
      crateControllerRef.current = createGroupedBarChart(
        container,
        monitor.crateChartData,
        monitor.formatCurrentSortMetricValue,
        240,
      );
      return;
    }

    crateControllerRef.current.update(monitor.crateChartData);
  }, [monitor.crateChartData, monitor.formatCurrentSortMetricValue]);

  return (
    <div className="space-y-3 pb-2">
      <ResourceMonitorHeader
        initialized={monitor.initialized}
        loading={monitor.loading}
        error={monitor.error}
        historyWindowMinutes={monitor.historyWindowMinutes}
        sortMetric={monitor.sortMetric}
        sampledAtText={monitor.meta.sampledAtText}
        lastUpdatedText={monitor.meta.lastUpdatedText}
        historyPointsText={monitor.meta.historyPointsText}
        onRefresh={() => {
          void monitor.refreshAll();
        }}
        onResetSession={() => {
          void monitor.resetSessionWithLocalClear();
        }}
        onHistoryWindowChange={monitor.setHistoryWindowMinutes}
        onSortMetricChange={monitor.setSortMetric}
      />

      <ResourceMetricsSection metricCards={monitor.metricCards} />

      <ResourceTrendSection
        historyChartRef={historyChartRef}
        moduleChartRef={moduleChartRef}
        hasHistoryData={monitor.visibleHistory.length > 0}
        hasModuleChartData={monitor.moduleChartData.length > 0}
      />

      <ResourceAttributionSection
        crateChartRef={crateChartRef}
        hasCrateChartData={monitor.crateChartData.length > 0}
        analysisRows={monitor.analysisRows}
      />
    </div>
  );
}

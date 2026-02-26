import { Chart } from "@antv/g2";

import { getChartThemeConfig, type ChartTooltipTheme } from "@/theme/chartTheme";

const HISTORY_PIXELS_PER_POINT = 8;
const HISTORY_MIN_VISIBLE_POINTS = 20;
const HISTORY_MAX_VISIBLE_POINTS = 120;
const GROUPED_PIXELS_PER_BUCKET = 24;
const GROUPED_MIN_VISIBLE_BUCKETS = 4;
const GROUPED_MAX_VISIBLE_BUCKETS = 40;
const MIN_SLIDER_SPAN = 0.01;
const SLIDER_FILTER_EVENT = "sliderX:filter";
const DEFAULT_MIN_WIDTH = 320;

export interface HistoryChartDatum {
  time: string;
  value: number;
  metric: string;
  kind: "cpu" | "memory";
}

export interface GroupedBarChartDatum {
  time: string;
  group: string;
  value: number;
}

export interface ChartController<T> {
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
  tooltipTheme: ChartTooltipTheme;
  getTitle: (rows: T[]) => string;
  getItems: (rows: T[]) => TooltipItem[];
}

interface TooltipController {
  refresh: () => void;
  destroy: () => void;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function buildTimeDomain<T extends { time: string }>(data: T[]): string[] {
  return [...new Set(data.map((item) => item.time))];
}

function countUniqueXLabels<T extends { time: string }>(data: T[]): number {
  return buildTimeDomain(data).length;
}

function normalizeRange(value: [number, number]): [number, number] {
  const start = clamp(value[0], 0, 1);
  const end = clamp(value[1], 0, 1);
  return start <= end ? [start, end] : [end, start];
}

function clampRangeWidth(range: [number, number], maxSpan: number): [number, number] {
  const [start, end] = normalizeRange(range);
  const span = end - start;
  if (span <= maxSpan) {
    return [start, end];
  }

  const center = (start + end) / 2;
  let nextStart = center - maxSpan / 2;
  let nextEnd = center + maxSpan / 2;
  if (nextStart < 0) {
    nextEnd -= nextStart;
    nextStart = 0;
  }
  if (nextEnd > 1) {
    nextStart -= nextEnd - 1;
    nextEnd = 1;
  }

  return normalizeRange([nextStart, nextEnd]);
}

function resolveVisibleCount(width: number, pixelsPerUnit: number, minCount: number, maxCount: number): number {
  return clamp(Math.floor(width / pixelsPerUnit), minCount, maxCount);
}

function resolveMaxSpan(
  totalCount: number,
  width: number,
  pixelsPerUnit: number,
  minCount: number,
  maxCount: number,
): number {
  if (totalCount <= 0) {
    return 1;
  }
  const visibleCount = resolveVisibleCount(width, pixelsPerUnit, minCount, maxCount);
  return clamp(visibleCount / totalCount, MIN_SLIDER_SPAN, 1);
}

function ratioFromDomainSelection(selection: unknown, domain: string[]): [number, number] | null {
  if (!Array.isArray(selection) || selection.length < 2 || domain.length <= 1) {
    return null;
  }
  const [startDomain, endDomain] = selection;
  const startIndex = domain.indexOf(String(startDomain));
  const endIndex = domain.indexOf(String(endDomain));
  if (startIndex < 0 || endIndex < 0) {
    return null;
  }

  const divisor = domain.length - 1;
  return normalizeRange([startIndex / divisor, endIndex / divisor]);
}

function buildSliderConfig(values: [number, number]) {
  return {
    x: {
      showHandle: true,
      showLabel: false,
      showLabelOnInteraction: true,
      values,
    },
  };
}

function resolveLatestRange(maxSpan: number): [number, number] {
  return normalizeRange([Math.max(0, 1 - maxSpan), 1]);
}

function alignRangeToLatest(range: [number, number], maxSpan: number): [number, number] {
  const normalized = normalizeRange(range);
  const span = clamp(normalized[1] - normalized[0], MIN_SLIDER_SPAN, maxSpan);
  return resolveLatestRange(span);
}

export function createTooltipController<T extends object>(
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
  tooltip.style.border = `1px solid ${options.tooltipTheme.border}`;
  tooltip.style.background = options.tooltipTheme.background;
  tooltip.style.color = options.tooltipTheme.text;
  tooltip.style.borderRadius = "8px";
  tooltip.style.padding = "8px 10px";
  tooltip.style.fontSize = "12px";
  tooltip.style.lineHeight = "1.4";
  tooltip.style.whiteSpace = "nowrap";
  tooltip.style.boxShadow = options.tooltipTheme.shadow;
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

export function createHistoryChart(
  element: HTMLDivElement,
  data: HistoryChartDatum[],
): ChartController<HistoryChartDatum> {
  const themeConfig = getChartThemeConfig();
  const getMaxSpan = (nextData: HistoryChartDatum[]) =>
    resolveMaxSpan(
      countUniqueXLabels(nextData),
      Math.max(element.clientWidth, DEFAULT_MIN_WIDTH),
      HISTORY_PIXELS_PER_POINT,
      HISTORY_MIN_VISIBLE_POINTS,
      HISTORY_MAX_VISIBLE_POINTS,
    );
  let domain = buildTimeDomain(data);
  let maxSpan = getMaxSpan(data);
  let currentRange = resolveLatestRange(maxSpan);

  const chart = new Chart({
    container: element,
    autoFit: true,
    height: 280,
  });
  chart.theme(themeConfig.g2Theme);
  chart.animate(false);
  chart.interaction("tooltip", false);

  const line = chart
    .line()
    .data(data)
    .encode("x", "time")
    .encode("y", "value")
    .encode("color", "metric")
    .legend("color", false)
    .scale("color", {
      range: themeConfig.seriesPalette.slice(0, 2),
    })
    .slider(buildSliderConfig(currentRange))
    .axis({
      x: {
        title: false,
        labelAutoRotate: false,
        labelAutoHide: true,
        labelAutoEllipsis: true,
      },
      y: { title: false },
    })
    .style("lineWidth", 2)
    .animate(false)
    .tooltip(false);

  const onSliderFilter = (event: unknown) => {
    const selection = (event as { data?: { selection?: unknown[] } })?.data?.selection?.[0];
    const nextRange = ratioFromDomainSelection(selection, domain);
    if (!nextRange) {
      return;
    }
    currentRange = clampRangeWidth(nextRange, maxSpan);
  };

  chart.on(SLIDER_FILTER_EVENT, onSliderFilter);
  chart.render();

  const tooltip = createTooltipController<HistoryChartDatum>(chart, element, {
    series: true,
    shared: true,
    tooltipTheme: themeConfig.tooltip,
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
      domain = buildTimeDomain(nextData);
      maxSpan = getMaxSpan(nextData);
      currentRange = alignRangeToLatest(currentRange, maxSpan);
      line.slider(buildSliderConfig(currentRange));
      void line.changeData(nextData).then(() => {
        tooltip.refresh();
      });
    },
    destroy() {
      chart.off(SLIDER_FILTER_EVENT, onSliderFilter);
      tooltip.destroy();
      chart.destroy();
    },
  };
}

export function createGroupedBarChart(
  element: HTMLDivElement,
  data: GroupedBarChartDatum[],
  valueFormatter: (value: number) => string,
  height = 280,
): ChartController<GroupedBarChartDatum> {
  const themeConfig = getChartThemeConfig();
  const getMaxSpan = (nextData: GroupedBarChartDatum[]) =>
    resolveMaxSpan(
      countUniqueXLabels(nextData),
      Math.max(element.clientWidth, DEFAULT_MIN_WIDTH),
      GROUPED_PIXELS_PER_BUCKET,
      GROUPED_MIN_VISIBLE_BUCKETS,
      GROUPED_MAX_VISIBLE_BUCKETS,
    );
  let domain = buildTimeDomain(data);
  let maxSpan = getMaxSpan(data);
  let currentRange = resolveLatestRange(maxSpan);

  const chart = new Chart({
    container: element,
    autoFit: true,
    height,
  });
  chart.theme(themeConfig.g2Theme);
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
      range: themeConfig.seriesPalette,
    })
    .slider(buildSliderConfig(currentRange))
    .axis({
      x: {
        title: false,
        labelAutoRotate: false,
        labelAutoHide: true,
        labelAutoEllipsis: true,
      },
      y: { title: false },
    })
    .style("maxWidth", 32)
    .animate(false)
    .tooltip(false);

  const onSliderFilter = (event: unknown) => {
    const selection = (event as { data?: { selection?: unknown[] } })?.data?.selection?.[0];
    const nextRange = ratioFromDomainSelection(selection, domain);
    if (!nextRange) {
      return;
    }
    currentRange = clampRangeWidth(nextRange, maxSpan);
  };

  chart.on(SLIDER_FILTER_EVENT, onSliderFilter);
  chart.render();

  const tooltip = createTooltipController<GroupedBarChartDatum>(chart, element, {
    series: false,
    shared: true,
    tooltipTheme: themeConfig.tooltip,
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
      domain = buildTimeDomain(nextData);
      maxSpan = getMaxSpan(nextData);
      currentRange = alignRangeToLatest(currentRange, maxSpan);
      interval.slider(buildSliderConfig(currentRange));
      void interval.changeData(nextData).then(() => {
        tooltip.refresh();
      });
    },
    destroy() {
      chart.off(SLIDER_FILTER_EVENT, onSliderFilter);
      tooltip.destroy();
      chart.destroy();
    },
  };
}

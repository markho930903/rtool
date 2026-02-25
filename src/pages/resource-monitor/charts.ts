import { Chart } from "@antv/g2";

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
  getTitle: (rows: T[]) => string;
  getItems: (rows: T[]) => TooltipItem[];
}

interface TooltipController {
  refresh: () => void;
  destroy: () => void;
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

export function createHistoryChart(
  element: HTMLDivElement,
  data: HistoryChartDatum[],
): ChartController<HistoryChartDatum> {
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

export function createGroupedBarChart(
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

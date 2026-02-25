type ChartColorToken =
  | "--color-chart-axis-label"
  | "--color-chart-axis-title"
  | "--color-chart-axis-line"
  | "--color-chart-grid-line"
  | "--color-chart-legend-label"
  | "--color-chart-legend-value"
  | "--color-chart-legend-nav"
  | "--color-chart-series-1"
  | "--color-chart-series-2"
  | "--color-chart-series-3"
  | "--color-chart-series-4"
  | "--color-chart-series-5"
  | "--color-chart-tooltip-border"
  | "--color-chart-tooltip-bg"
  | "--color-chart-tooltip-text";

interface ChartTokenFallbackMap {
  [key: string]: string;
}

const CHART_TOKEN_FALLBACKS: ChartTokenFallbackMap = {
  "--color-chart-axis-label": "#d4d4d8",
  "--color-chart-axis-title": "#f4f4f5",
  "--color-chart-axis-line": "#52525b",
  "--color-chart-grid-line": "#3f3f46",
  "--color-chart-legend-label": "#f4f4f5",
  "--color-chart-legend-value": "#d4d4d8",
  "--color-chart-legend-nav": "#d4d4d8",
  "--color-chart-series-1": "#7dd3fc",
  "--color-chart-series-2": "#38bdf8",
  "--color-chart-series-3": "#cbd5e1",
  "--color-chart-series-4": "#fda4af",
  "--color-chart-series-5": "#fb7185",
  "--color-chart-tooltip-border": "#52525b",
  "--color-chart-tooltip-bg": "#27272e",
  "--color-chart-tooltip-text": "#f4f4f5",
};

function readCssToken(name: ChartColorToken, fallback: string): string {
  if (typeof window === "undefined") {
    return fallback;
  }
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function readShadowToken(fallback: string): string {
  if (typeof window === "undefined") {
    return fallback;
  }
  const value = getComputedStyle(document.documentElement).getPropertyValue("--shadow-overlay").trim();
  return value || fallback;
}

export interface ChartTooltipTheme {
  border: string;
  background: string;
  text: string;
  shadow: string;
}

export interface ChartThemeConfig {
  g2Theme: Record<string, unknown>;
  seriesPalette: [string, string, string, string, string];
  tooltip: ChartTooltipTheme;
}

export function getChartThemeConfig(): ChartThemeConfig {
  const axisLabel = readCssToken("--color-chart-axis-label", CHART_TOKEN_FALLBACKS["--color-chart-axis-label"]);
  const axisTitle = readCssToken("--color-chart-axis-title", CHART_TOKEN_FALLBACKS["--color-chart-axis-title"]);
  const axisLine = readCssToken("--color-chart-axis-line", CHART_TOKEN_FALLBACKS["--color-chart-axis-line"]);
  const gridLine = readCssToken("--color-chart-grid-line", CHART_TOKEN_FALLBACKS["--color-chart-grid-line"]);
  const legendLabel = readCssToken("--color-chart-legend-label", CHART_TOKEN_FALLBACKS["--color-chart-legend-label"]);
  const legendValue = readCssToken("--color-chart-legend-value", CHART_TOKEN_FALLBACKS["--color-chart-legend-value"]);
  const legendNav = readCssToken("--color-chart-legend-nav", CHART_TOKEN_FALLBACKS["--color-chart-legend-nav"]);

  const seriesPalette: [string, string, string, string, string] = [
    readCssToken("--color-chart-series-1", CHART_TOKEN_FALLBACKS["--color-chart-series-1"]),
    readCssToken("--color-chart-series-2", CHART_TOKEN_FALLBACKS["--color-chart-series-2"]),
    readCssToken("--color-chart-series-3", CHART_TOKEN_FALLBACKS["--color-chart-series-3"]),
    readCssToken("--color-chart-series-4", CHART_TOKEN_FALLBACKS["--color-chart-series-4"]),
    readCssToken("--color-chart-series-5", CHART_TOKEN_FALLBACKS["--color-chart-series-5"]),
  ];

  const tooltip: ChartTooltipTheme = {
    border: readCssToken("--color-chart-tooltip-border", CHART_TOKEN_FALLBACKS["--color-chart-tooltip-border"]),
    background: readCssToken("--color-chart-tooltip-bg", CHART_TOKEN_FALLBACKS["--color-chart-tooltip-bg"]),
    text: readCssToken("--color-chart-tooltip-text", CHART_TOKEN_FALLBACKS["--color-chart-tooltip-text"]),
    shadow: readShadowToken("var(--shadow-overlay)"),
  };

  return {
    g2Theme: {
      axis: {
        labelFill: axisLabel,
        labelOpacity: 1,
        titleFill: axisTitle,
        titleOpacity: 1,
        lineStroke: axisLine,
        lineStrokeOpacity: 1,
        tickStroke: axisLine,
        tickOpacity: 1,
        gridStroke: gridLine,
        gridStrokeOpacity: 1,
      },
      axisBottom: {
        labelFill: axisLabel,
        titleFill: axisTitle,
        lineStroke: axisLine,
        tickStroke: axisLine,
        gridStroke: gridLine,
      },
      axisLeft: {
        labelFill: axisLabel,
        titleFill: axisTitle,
        lineStroke: axisLine,
        tickStroke: axisLine,
        gridStroke: gridLine,
      },
      axisRight: {
        labelFill: axisLabel,
        titleFill: axisTitle,
        lineStroke: axisLine,
        tickStroke: axisLine,
        gridStroke: gridLine,
      },
      legendCategory: {
        itemLabelFill: legendLabel,
        itemLabelFillOpacity: 1,
        itemValueFill: legendValue,
        itemValueFillOpacity: 1,
        navButtonFill: legendNav,
        navButtonFillOpacity: 1,
        navPageNumFill: legendNav,
        navPageNumFillOpacity: 0.9,
        tickStroke: axisLine,
        tickStrokeOpacity: 1,
        titleFill: axisTitle,
        titleFillOpacity: 1,
      },
      legendContinuous: {
        labelFill: legendLabel,
        labelFillOpacity: 1,
        handleLabelFill: legendValue,
        handleLabelFillOpacity: 1,
        tickStroke: axisLine,
        tickStrokeOpacity: 1,
        titleFill: axisTitle,
        titleFillOpacity: 1,
      },
      htmlLabel: {
        color: legendLabel,
        opacity: 1,
      },
    },
    seriesPalette,
    tooltip,
  };
}

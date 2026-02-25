import { DEFAULT_GLASS_SETTINGS, DEFAULT_THEME_PREFERENCE, GLASS_RANGES, THEME_MEDIA_QUERY } from "@/theme/constants";
import type { LiquidGlassProfile, LiquidGlassSettings, ResolvedTheme, ThemePreference } from "@/theme/types";

const ALLOWED_PREFERENCES: ThemePreference[] = ["light", "dark", "system"];

interface GlassAlphaStops {
  strong: [number, number];
  surface: [number, number];
  soft: [number, number];
  overlay: [number, number];
}

const GLASS_ALPHA_STOPS: Record<ResolvedTheme, GlassAlphaStops> = {
  dark: {
    strong: [20, 88],
    surface: [16, 82],
    soft: [4, 28],
    overlay: [18, 90],
  },
  light: {
    strong: [28, 88],
    surface: [22, 80],
    soft: [10, 58],
    overlay: [30, 88],
  },
};

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function cloneGlassSettings(settings: LiquidGlassSettings): LiquidGlassSettings {
  return {
    light: { ...settings.light },
    dark: { ...settings.dark },
  };
}

function clampNumber(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function interpolate(min: number, max: number, progress: number): number {
  return min + (max - min) * progress;
}

function formatPercent(value: number): string {
  return `${clampNumber(value, 0, 100).toFixed(2)}%`;
}

export function normalizeThemePreference(value: string | null | undefined): ThemePreference {
  if (value && ALLOWED_PREFERENCES.includes(value as ThemePreference)) {
    return value as ThemePreference;
  }
  return DEFAULT_THEME_PREFERENCE;
}

function normalizeGlassProfile(
  input: Partial<LiquidGlassProfile> | undefined,
  fallback: LiquidGlassProfile,
): LiquidGlassProfile {
  return {
    opacity: clampNumber(
      isFiniteNumber(input?.opacity) ? Math.round(input.opacity) : fallback.opacity,
      GLASS_RANGES.opacity.min,
      GLASS_RANGES.opacity.max,
    ),
    blur: clampNumber(
      isFiniteNumber(input?.blur) ? Math.round(input.blur) : fallback.blur,
      GLASS_RANGES.blur.min,
      GLASS_RANGES.blur.max,
    ),
    saturate: clampNumber(
      isFiniteNumber(input?.saturate) ? Math.round(input.saturate) : fallback.saturate,
      GLASS_RANGES.saturate.min,
      GLASS_RANGES.saturate.max,
    ),
    brightness: clampNumber(
      isFiniteNumber(input?.brightness) ? Math.round(input.brightness) : fallback.brightness,
      GLASS_RANGES.brightness.min,
      GLASS_RANGES.brightness.max,
    ),
  };
}

export function normalizeLiquidGlassSettings(
  input: Partial<LiquidGlassSettings> | null | undefined,
): LiquidGlassSettings {
  return {
    light: normalizeGlassProfile(input?.light, DEFAULT_GLASS_SETTINGS.light),
    dark: normalizeGlassProfile(input?.dark, DEFAULT_GLASS_SETTINGS.dark),
  };
}

export function getDefaultLiquidGlassSettings(): LiquidGlassSettings {
  return cloneGlassSettings(DEFAULT_GLASS_SETTINGS);
}

export function getSystemPrefersDark(): boolean {
  if (typeof window === "undefined") {
    return true;
  }
  return window.matchMedia(THEME_MEDIA_QUERY).matches;
}

export function resolveTheme(preference: ThemePreference, systemPrefersDark: boolean): ResolvedTheme {
  if (preference === "system") {
    return systemPrefersDark ? "dark" : "light";
  }
  return preference;
}

export function applyThemeToDocument(preference: ThemePreference, resolved: ResolvedTheme) {
  if (typeof document === "undefined") {
    return;
  }

  const root = document.documentElement;
  root.setAttribute("data-theme", resolved);
  root.setAttribute("data-theme-preference", preference);
  root.style.colorScheme = resolved;
}

export function applyLiquidGlassToDocument(resolved: ResolvedTheme, settings: LiquidGlassSettings) {
  if (typeof document === "undefined") {
    return;
  }

  const root = document.documentElement;
  const profile = resolved === "dark" ? settings.dark : settings.light;
  const normalizedOpacity = clampNumber(profile.opacity, GLASS_RANGES.opacity.min, GLASS_RANGES.opacity.max);
  const opacitySpan = GLASS_RANGES.opacity.max - GLASS_RANGES.opacity.min;
  const opacityProgress = opacitySpan <= 0 ? 1 : (normalizedOpacity - GLASS_RANGES.opacity.min) / opacitySpan;
  const alphaStops = GLASS_ALPHA_STOPS[resolved];

  const strongAlpha = interpolate(alphaStops.strong[0], alphaStops.strong[1], opacityProgress);
  const surfaceAlpha = interpolate(alphaStops.surface[0], alphaStops.surface[1], opacityProgress);
  const softAlpha = interpolate(alphaStops.soft[0], alphaStops.soft[1], opacityProgress);
  const overlayAlpha = interpolate(alphaStops.overlay[0], alphaStops.overlay[1], opacityProgress);

  root.style.setProperty("--glass-blur", `${profile.blur}px`);
  root.style.setProperty("--glass-saturate", `${profile.saturate}%`);
  root.style.setProperty("--glass-brightness", `${profile.brightness}%`);
  root.style.setProperty("--glass-alpha-strong", formatPercent(strongAlpha));
  root.style.setProperty("--glass-alpha-surface", formatPercent(surfaceAlpha));
  root.style.setProperty("--glass-alpha-soft", formatPercent(softAlpha));
  root.style.setProperty("--glass-alpha-overlay", formatPercent(overlayAlpha));
}

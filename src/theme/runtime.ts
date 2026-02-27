import { DEFAULT_GLASS_SETTINGS, DEFAULT_THEME_PREFERENCE, GLASS_RANGES, THEME_MEDIA_QUERY } from "@/theme/constants";
import type { LiquidGlassProfile, LiquidGlassSettings, ResolvedTheme, ThemePreference } from "@/theme/types";

const ALLOWED_PREFERENCES: ThemePreference[] = ["light", "dark", "system"];

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
  const normalizedBlur = clampNumber(profile.blur, GLASS_RANGES.blur.min, GLASS_RANGES.blur.max);
  const normalizedSaturate = clampNumber(profile.saturate, GLASS_RANGES.saturate.min, GLASS_RANGES.saturate.max);
  const normalizedBrightness = clampNumber(profile.brightness, GLASS_RANGES.brightness.min, GLASS_RANGES.brightness.max);
  const opacityPercent = formatPercent(normalizedOpacity);

  root.style.setProperty("--glass-alpha", opacityPercent);
  root.style.setProperty("--glass-blur", `${normalizedBlur}px`);
  root.style.setProperty("--glass-saturate", `${normalizedSaturate}%`);
  root.style.setProperty("--glass-brightness", `${normalizedBrightness}%`);
}

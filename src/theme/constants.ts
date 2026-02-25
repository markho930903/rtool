import type { LiquidGlassSettings, ThemePreference } from "@/theme/types";

export const DEFAULT_THEME_PREFERENCE: ThemePreference = "system";

export const THEME_MEDIA_QUERY = "(prefers-color-scheme: dark)";

export const DEFAULT_GLASS_SETTINGS: LiquidGlassSettings = {
  light: {
    opacity: 100,
    blur: 20,
    saturate: 135,
    brightness: 100,
  },
  dark: {
    opacity: 100,
    blur: 24,
    saturate: 150,
    brightness: 100,
  },
};

export const GLASS_RANGES = {
  opacity: { min: 0, max: 100 },
  blur: { min: 8, max: 40 },
  saturate: { min: 100, max: 220 },
  brightness: { min: 85, max: 130 },
} as const;

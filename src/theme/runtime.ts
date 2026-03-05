import { DEFAULT_THEME_PREFERENCE, THEME_MEDIA_QUERY } from "@/theme/constants";
import type { ResolvedTheme, ThemePreference } from "@/theme/types";

const ALLOWED_PREFERENCES: ThemePreference[] = ["light", "dark", "system"];

export function normalizeThemePreference(value: string | null | undefined): ThemePreference {
  if (value && ALLOWED_PREFERENCES.includes(value as ThemePreference)) {
    return value as ThemePreference;
  }
  return DEFAULT_THEME_PREFERENCE;
}

export function normalizeTransparentWindowBackground(value: unknown): boolean {
  return value === true;
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

export function applyThemeToDocument(
  preference: ThemePreference,
  resolved: ResolvedTheme,
  transparentWindowBackground: boolean,
) {
  if (typeof document === "undefined") {
    return;
  }

  const root = document.documentElement;
  root.setAttribute("data-theme", resolved);
  root.setAttribute("data-theme-preference", preference);
  root.setAttribute("data-window-transparency", transparentWindowBackground ? "on" : "off");
  root.style.colorScheme = resolved;
}

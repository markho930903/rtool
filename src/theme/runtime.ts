import { DEFAULT_THEME_PREFERENCE, THEME_MEDIA_QUERY, THEME_STORAGE_KEY } from "@/theme/constants";
import type { ResolvedTheme, ThemePreference } from "@/theme/types";

const ALLOWED_PREFERENCES: ThemePreference[] = ["light", "dark", "system"];

function isThemePreference(value: string | null): value is ThemePreference {
  return value !== null && ALLOWED_PREFERENCES.includes(value as ThemePreference);
}

export function getStoredThemePreference(): ThemePreference {
  if (typeof window === "undefined") {
    return DEFAULT_THEME_PREFERENCE;
  }

  let stored: string | null = null;
  try {
    stored = window.localStorage.getItem(THEME_STORAGE_KEY);
  } catch {
    stored = null;
  }

  return isThemePreference(stored) ? stored : DEFAULT_THEME_PREFERENCE;
}

export function setStoredThemePreference(preference: ThemePreference) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, preference);
  } catch {}
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

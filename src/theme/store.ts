import { create } from "zustand";

import { logWarn } from "@/services/logger";
import { getUserSettings, patchUserSettings } from "@/services/user-settings.service";
import { THEME_MEDIA_QUERY } from "@/theme/constants";
import {
  applyLiquidGlassToDocument,
  applyThemeToDocument,
  getDefaultLiquidGlassSettings,
  getSystemPrefersDark,
  normalizeLiquidGlassSettings,
  normalizeThemePreference,
  resolveTheme,
} from "@/theme/runtime";
import type {
  LiquidGlassProfile,
  LiquidGlassSettings,
  ResolvedTheme,
  ThemePreference,
  ThemeState,
} from "@/theme/types";

interface ThemeActions {
  init: () => Promise<void>;
  syncFromStorage: () => Promise<void>;
  setPreference: (preference: ThemePreference) => Promise<void>;
  previewGlassProfile: (theme: ResolvedTheme, patch: Partial<LiquidGlassProfile>) => void;
  commitGlassProfile: (theme: ResolvedTheme, patch: Partial<LiquidGlassProfile>) => Promise<void>;
  // Backward-compatible alias for existing callers.
  setGlassProfile: (theme: ResolvedTheme, patch: Partial<LiquidGlassProfile>) => Promise<void>;
  resetGlassProfile: (theme: ResolvedTheme) => Promise<void>;
  toggle: () => void;
}

type ThemeStore = ThemeState & ThemeActions;

let mediaQuery: MediaQueryList | null = null;
let mediaQueryListener: ((event: MediaQueryListEvent) => void) | null = null;
let latestGlassCommitId = 0;

function isSameGlass(left: LiquidGlassSettings, right: LiquidGlassSettings): boolean {
  return (
    left.light.opacity === right.light.opacity &&
    left.light.blur === right.light.blur &&
    left.light.saturate === right.light.saturate &&
    left.light.brightness === right.light.brightness &&
    left.dark.opacity === right.dark.opacity &&
    left.dark.blur === right.dark.blur &&
    left.dark.saturate === right.dark.saturate &&
    left.dark.brightness === right.dark.brightness
  );
}

function applyTheme(preference: ThemePreference, resolved: ResolvedTheme, glassSettings: LiquidGlassSettings) {
  applyThemeToDocument(preference, resolved);
  applyLiquidGlassToDocument(resolved, glassSettings);
}

function teardownSystemListener() {
  if (!mediaQuery || !mediaQueryListener) {
    return;
  }

  mediaQuery.removeEventListener("change", mediaQueryListener);
  mediaQuery = null;
  mediaQueryListener = null;
}

function setupSystemListener() {
  if (typeof window === "undefined" || useThemeStore.getState().preference !== "system") {
    teardownSystemListener();
    return;
  }

  if (mediaQuery && mediaQueryListener) {
    return;
  }

  mediaQuery = window.matchMedia(THEME_MEDIA_QUERY);
  mediaQueryListener = () => {
    const current = useThemeStore.getState();
    if (current.preference !== "system") {
      return;
    }

    const resolved = resolveTheme("system", getSystemPrefersDark());
    if (resolved === current.resolved) {
      return;
    }

    useThemeStore.setState({ resolved });
    applyTheme("system", resolved, current.glassSettings);
  };

  mediaQuery.addEventListener("change", mediaQueryListener);
}

function buildPatchedGlassSettings(
  current: LiquidGlassSettings,
  theme: ResolvedTheme,
  patch: Partial<LiquidGlassProfile>,
): LiquidGlassSettings {
  const next = {
    light: { ...current.light },
    dark: { ...current.dark },
  };
  if (theme === "light") {
    next.light = { ...next.light, ...patch };
  } else {
    next.dark = { ...next.dark, ...patch };
  }
  return normalizeLiquidGlassSettings(next);
}

function toThemePatch(theme: ResolvedTheme, patch: Partial<LiquidGlassProfile>) {
  return theme === "light"
    ? {
        theme: {
          glass: {
            light: patch,
          },
        },
      }
    : {
        theme: {
          glass: {
            dark: patch,
          },
        },
      };
}

export const useThemeStore = create<ThemeStore>((set, get) => ({
  preference: "system",
  resolved: "dark",
  glassSettings: getDefaultLiquidGlassSettings(),
  initialized: false,
  async init() {
    if (get().initialized) {
      return;
    }

    await get().syncFromStorage();
  },
  async syncFromStorage() {
    try {
      const settings = await getUserSettings();
      const preference = normalizeThemePreference(settings.theme.preference);
      const glassSettings = normalizeLiquidGlassSettings(settings.theme.glass);
      const resolved = resolveTheme(preference, getSystemPrefersDark());
      const current = get();

      if (
        current.initialized &&
        current.preference === preference &&
        current.resolved === resolved &&
        isSameGlass(current.glassSettings, glassSettings)
      ) {
        setupSystemListener();
        return;
      }

      set({
        preference,
        resolved,
        glassSettings,
        initialized: true,
      });
      applyTheme(preference, resolved, glassSettings);
      setupSystemListener();
    } catch (error) {
      logWarn("theme", "sync_from_backend_failed", {
        error: error instanceof Error ? error.message : String(error),
      });
      const current = get();
      if (!current.initialized) {
        const fallbackPreference: ThemePreference = "system";
        const fallbackGlass = getDefaultLiquidGlassSettings();
        const resolved = resolveTheme(fallbackPreference, getSystemPrefersDark());
        set({
          preference: fallbackPreference,
          resolved,
          glassSettings: fallbackGlass,
          initialized: true,
        });
        applyTheme(fallbackPreference, resolved, fallbackGlass);
      }
      setupSystemListener();
    }
  },
  async setPreference(preference) {
    const canonical = normalizeThemePreference(preference);
    const optimisticResolved = resolveTheme(canonical, getSystemPrefersDark());
    const currentGlass = get().glassSettings;

    set({
      preference: canonical,
      resolved: optimisticResolved,
      initialized: true,
    });
    applyTheme(canonical, optimisticResolved, currentGlass);
    setupSystemListener();

    try {
      const settings = await patchUserSettings({
        theme: {
          preference: canonical,
        },
      });
      const nextPreference = normalizeThemePreference(settings.theme.preference);
      const nextGlass = normalizeLiquidGlassSettings(settings.theme.glass);
      const nextResolved = resolveTheme(nextPreference, getSystemPrefersDark());
      set({
        preference: nextPreference,
        resolved: nextResolved,
        glassSettings: nextGlass,
        initialized: true,
      });
      applyTheme(nextPreference, nextResolved, nextGlass);
      setupSystemListener();
    } catch (error) {
      logWarn("theme", "set_preference_failed", {
        preference: canonical,
        error: error instanceof Error ? error.message : String(error),
      });
      await get().syncFromStorage();
    }
  },
  previewGlassProfile(theme, patch) {
    const current = get();
    const nextGlassSettings = buildPatchedGlassSettings(current.glassSettings, theme, patch);
    set({
      glassSettings: nextGlassSettings,
      initialized: true,
    });
    applyTheme(current.preference, current.resolved, nextGlassSettings);
  },
  async commitGlassProfile(theme, patch) {
    const commitId = ++latestGlassCommitId;
    const current = get();
    const nextGlassSettings = buildPatchedGlassSettings(current.glassSettings, theme, patch);
    set({
      glassSettings: nextGlassSettings,
      initialized: true,
    });
    applyTheme(current.preference, current.resolved, nextGlassSettings);

    try {
      const settings = await patchUserSettings(toThemePatch(theme, patch));
      if (commitId !== latestGlassCommitId) {
        return;
      }
      const normalizedPreference = normalizeThemePreference(settings.theme.preference);
      const normalizedGlass = normalizeLiquidGlassSettings(settings.theme.glass);
      const resolved = resolveTheme(normalizedPreference, getSystemPrefersDark());
      set({
        preference: normalizedPreference,
        resolved,
        glassSettings: normalizedGlass,
        initialized: true,
      });
      applyTheme(normalizedPreference, resolved, normalizedGlass);
      setupSystemListener();
    } catch (error) {
      if (commitId !== latestGlassCommitId) {
        return;
      }
      logWarn("theme", "set_glass_profile_failed", {
        theme,
        patch,
        error: error instanceof Error ? error.message : String(error),
      });
      await get().syncFromStorage();
    }
  },
  async setGlassProfile(theme, patch) {
    await get().commitGlassProfile(theme, patch);
  },
  async resetGlassProfile(theme) {
    const defaults = getDefaultLiquidGlassSettings();
    const profile = theme === "light" ? defaults.light : defaults.dark;
    await get().commitGlassProfile(theme, profile);
  },
  toggle() {
    const nextPreference: ThemePreference = get().resolved === "dark" ? "light" : "dark";
    void get().setPreference(nextPreference);
  },
}));

// Backward-compatible export for stale module graphs during hot reload.
export const themeActions: ThemeActions = {
  init: () => useThemeStore.getState().init(),
  syncFromStorage: () => useThemeStore.getState().syncFromStorage(),
  setPreference: (preference) => useThemeStore.getState().setPreference(preference),
  previewGlassProfile: (theme, patch) => useThemeStore.getState().previewGlassProfile(theme, patch),
  commitGlassProfile: (theme, patch) => useThemeStore.getState().commitGlassProfile(theme, patch),
  setGlassProfile: (theme, patch) => useThemeStore.getState().setGlassProfile(theme, patch),
  resetGlassProfile: (theme) => useThemeStore.getState().resetGlassProfile(theme),
  toggle: () => useThemeStore.getState().toggle(),
};

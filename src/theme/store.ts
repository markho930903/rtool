import { create } from "zustand";

import type { SettingsDto, ThemeSettingsDto } from "@/contracts";
import { logWarn } from "@/services/logger";
import { getSettings, patchSettings } from "@/services/settings.service";
import { DEFAULT_TRANSPARENT_WINDOW_BACKGROUND, THEME_MEDIA_QUERY } from "@/theme/constants";
import {
  applyThemeToDocument,
  getSystemPrefersDark,
  normalizeThemePreference,
  normalizeTransparentWindowBackground,
  resolveTheme,
} from "@/theme/runtime";
import type { ResolvedTheme, ThemePreference, ThemeState } from "@/theme/types";

interface ThemeActions {
  init: () => Promise<void>;
  syncFromStorage: () => Promise<void>;
  hydrateFromSettings: (settings: SettingsDto) => void;
  hydrateFromThemeSettings: (themeSettings: ThemeSettingsDto) => void;
  setPreference: (preference: ThemePreference) => Promise<void>;
  setTransparentWindowBackground: (enabled: boolean) => Promise<void>;
  toggle: () => void;
}

type ThemeStore = ThemeState & ThemeActions;

let mediaQuery: MediaQueryList | null = null;
let mediaQueryListener: ((event: MediaQueryListEvent) => void) | null = null;
let latestTransparencyCommitId = 0;

function applyTheme(preference: ThemePreference, resolved: ResolvedTheme, transparentWindowBackground: boolean) {
  applyThemeToDocument(preference, resolved, transparentWindowBackground);
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
    applyTheme("system", resolved, current.transparentWindowBackground);
  };

  mediaQuery.addEventListener("change", mediaQueryListener);
}

export const useThemeStore = create<ThemeStore>((set, get) => ({
  preference: "system",
  resolved: "dark",
  transparentWindowBackground: DEFAULT_TRANSPARENT_WINDOW_BACKGROUND,
  initialized: false,
  async init() {
    if (get().initialized) {
      return;
    }

    await get().syncFromStorage();
  },
  async syncFromStorage() {
    try {
      const settings = await getSettings();
      get().hydrateFromSettings(settings);
    } catch (error) {
      logWarn("theme", "sync_from_backend_failed", {
        error: error instanceof Error ? error.message : String(error),
      });
      const current = get();
      if (!current.initialized) {
        const fallbackPreference: ThemePreference = "system";
        const resolved = resolveTheme(fallbackPreference, getSystemPrefersDark());
        const transparentWindowBackground = DEFAULT_TRANSPARENT_WINDOW_BACKGROUND;
        set({
          preference: fallbackPreference,
          resolved,
          transparentWindowBackground,
          initialized: true,
        });
        applyTheme(fallbackPreference, resolved, transparentWindowBackground);
      }
      setupSystemListener();
    }
  },
  hydrateFromSettings(settings) {
    get().hydrateFromThemeSettings(settings.theme);
  },
  hydrateFromThemeSettings(themeSettings) {
    const preference = normalizeThemePreference(themeSettings.preference);
    const transparentWindowBackground = normalizeTransparentWindowBackground(themeSettings.transparentWindowBackground);
    const resolved = resolveTheme(preference, getSystemPrefersDark());
    const current = get();

    if (
      current.initialized &&
      current.preference === preference &&
      current.resolved === resolved &&
      current.transparentWindowBackground === transparentWindowBackground
    ) {
      setupSystemListener();
      return;
    }

    set({
      preference,
      resolved,
      transparentWindowBackground,
      initialized: true,
    });
    applyTheme(preference, resolved, transparentWindowBackground);
    setupSystemListener();
  },
  async setPreference(preference) {
    const canonical = normalizeThemePreference(preference);
    const optimisticResolved = resolveTheme(canonical, getSystemPrefersDark());
    const currentTransparentWindowBackground = get().transparentWindowBackground;

    set({
      preference: canonical,
      resolved: optimisticResolved,
      initialized: true,
    });
    applyTheme(canonical, optimisticResolved, currentTransparentWindowBackground);
    setupSystemListener();

    try {
      const settings = await patchSettings({
        theme: {
          preference: canonical,
        },
      });
      get().hydrateFromSettings(settings);
    } catch (error) {
      logWarn("theme", "set_preference_failed", {
        preference: canonical,
        error: error instanceof Error ? error.message : String(error),
      });
      await get().syncFromStorage();
    }
  },
  async setTransparentWindowBackground(enabled) {
    const commitId = ++latestTransparencyCommitId;
    const current = get();
    const normalizedEnabled = normalizeTransparentWindowBackground(enabled);
    set({
      transparentWindowBackground: normalizedEnabled,
      initialized: true,
    });
    applyTheme(current.preference, current.resolved, normalizedEnabled);

    try {
      const settings = await patchSettings({
        theme: {
          transparentWindowBackground: normalizedEnabled,
        },
      });
      if (commitId !== latestTransparencyCommitId) {
        return;
      }
      get().hydrateFromSettings(settings);
    } catch (error) {
      if (commitId !== latestTransparencyCommitId) {
        return;
      }
      logWarn("theme", "set_transparent_window_background_failed", {
        enabled: normalizedEnabled,
        error: error instanceof Error ? error.message : String(error),
      });
      await get().syncFromStorage();
    }
  },
  toggle() {
    const nextPreference: ThemePreference = get().resolved === "dark" ? "light" : "dark";
    void get().setPreference(nextPreference);
  },
}));

import { create } from "zustand";

import { THEME_MEDIA_QUERY, THEME_STORAGE_KEY } from "@/theme/constants";
import {
  applyThemeToDocument,
  getStoredThemePreference,
  getSystemPrefersDark,
  resolveTheme,
  setStoredThemePreference,
} from "@/theme/runtime";
import type { ThemePreference, ThemeState } from "@/theme/types";

interface ThemeActions {
  init: () => void;
  syncFromStorage: () => void;
  setPreference: (preference: ThemePreference) => void;
  toggle: () => void;
}

type ThemeStore = ThemeState & ThemeActions;

let mediaQuery: MediaQueryList | null = null;
let mediaQueryListener: ((event: MediaQueryListEvent) => void) | null = null;
let storageListener: ((event: StorageEvent) => void) | null = null;

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
    applyThemeToDocument("system", resolved);
  };

  mediaQuery.addEventListener("change", mediaQueryListener);
}

function setupStorageListener() {
  if (typeof window === "undefined" || storageListener) {
    return;
  }

  storageListener = (event) => {
    if (event.storageArea !== window.localStorage) {
      return;
    }

    if (event.key !== null && event.key !== THEME_STORAGE_KEY) {
      return;
    }

    useThemeStore.getState().syncFromStorage();
  };

  window.addEventListener("storage", storageListener);
}

export const useThemeStore = create<ThemeStore>((set, get) => ({
  preference: "system",
  resolved: "dark",
  initialized: false,
  init() {
    if (get().initialized) {
      return;
    }

    get().syncFromStorage();
    setupStorageListener();
  },
  syncFromStorage() {
    const preference = getStoredThemePreference();
    const resolved = resolveTheme(preference, getSystemPrefersDark());
    const current = get();

    if (current.initialized && current.preference === preference && current.resolved === resolved) {
      return;
    }

    set({
      preference,
      resolved,
      initialized: true,
    });
    applyThemeToDocument(preference, resolved);
    setupSystemListener();
  },
  setPreference(preference) {
    setStoredThemePreference(preference);
    set({
      preference,
      resolved: resolveTheme(preference, getSystemPrefersDark()),
      initialized: true,
    });
    applyThemeToDocument(preference, get().resolved);
    setupSystemListener();
  },
  toggle() {
    const nextPreference: ThemePreference = get().resolved === "dark" ? "light" : "dark";
    get().setPreference(nextPreference);
  },
}));

// Backward-compatible export for stale module graphs during hot reload.
export const themeActions: ThemeActions = {
  init: () => useThemeStore.getState().init(),
  syncFromStorage: () => useThemeStore.getState().syncFromStorage(),
  setPreference: (preference) => useThemeStore.getState().setPreference(preference),
  toggle: () => useThemeStore.getState().toggle(),
};

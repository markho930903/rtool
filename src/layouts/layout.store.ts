import { create } from "zustand";

import type { LayoutPreference, LayoutState } from "@/layouts/layout.types";

const LAYOUT_STORAGE_KEY = "rtool.layout.preference";
const DEFAULT_LAYOUT_PREFERENCE: LayoutPreference = "topbar";
const ALLOWED_PREFERENCES: LayoutPreference[] = ["topbar", "sidebar"];

function isLayoutPreference(value: string | null): value is LayoutPreference {
  return value !== null && ALLOWED_PREFERENCES.includes(value as LayoutPreference);
}

function getStoredLayoutPreference(): LayoutPreference {
  if (typeof window === "undefined") {
    return DEFAULT_LAYOUT_PREFERENCE;
  }

  let stored: string | null = null;
  try {
    stored = window.localStorage.getItem(LAYOUT_STORAGE_KEY);
  } catch {
    stored = null;
  }

  return isLayoutPreference(stored) ? stored : DEFAULT_LAYOUT_PREFERENCE;
}

function setStoredLayoutPreference(preference: LayoutPreference) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(LAYOUT_STORAGE_KEY, preference);
  } catch {}
}

interface LayoutActions {
  init: () => void;
  syncFromStorage: () => void;
  setPreference: (preference: LayoutPreference) => void;
}

type LayoutStore = LayoutState & LayoutActions;

let storageListener: ((event: StorageEvent) => void) | null = null;

function setupStorageListener() {
  if (typeof window === "undefined" || storageListener) {
    return;
  }

  storageListener = (event) => {
    if (event.storageArea !== window.localStorage) {
      return;
    }

    if (event.key !== null && event.key !== LAYOUT_STORAGE_KEY) {
      return;
    }

    useLayoutStore.getState().syncFromStorage();
  };

  window.addEventListener("storage", storageListener);
}

export const useLayoutStore = create<LayoutStore>((set, get) => ({
  preference: DEFAULT_LAYOUT_PREFERENCE,
  initialized: false,
  init() {
    if (get().initialized) {
      return;
    }

    get().syncFromStorage();
    setupStorageListener();
  },
  syncFromStorage() {
    const preference = getStoredLayoutPreference();
    const current = get();

    if (current.initialized && current.preference === preference) {
      return;
    }

    set({
      preference,
      initialized: true,
    });
  },
  setPreference(preference) {
    setStoredLayoutPreference(preference);
    set({
      preference,
      initialized: true,
    });
  },
}));

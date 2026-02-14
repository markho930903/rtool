import { create } from "zustand";

import i18n from "@/i18n";
import { LOCALE_STORAGE_KEY } from "@/i18n/constants";
import {
  applyLocaleToDocument,
  getStoredLocalePreference,
  resolveLocale,
  setStoredLocalePreference,
} from "@/i18n/runtime";
import type { AppLocale, LocalePreference, LocaleState } from "@/i18n/types";
import { logWarn } from "@/services/logger";
import { fetchBackendLocaleState, saveBackendLocalePreference } from "@/services/locale.service";

interface LocaleActions {
  init: () => void;
  syncFromStorage: () => void;
  syncFromBackend: () => Promise<void>;
  setPreference: (preference: LocalePreference) => void;
  setLocale: (locale: AppLocale) => void;
}

type LocaleStore = LocaleState & LocaleActions;

let storageListener: ((event: StorageEvent) => void) | null = null;

function applyLanguage(locale: AppLocale) {
  applyLocaleToDocument(locale);
  void i18n.changeLanguage(locale);
}

function setupStorageListener() {
  if (typeof window === "undefined" || storageListener) {
    return;
  }

  storageListener = (event) => {
    if (event.storageArea !== window.localStorage) {
      return;
    }

    if (event.key !== null && event.key !== LOCALE_STORAGE_KEY) {
      return;
    }

    useLocaleStore.getState().syncFromStorage();
  };

  window.addEventListener("storage", storageListener);
}

function applyLocaleState(
  set: (partial: Pick<LocaleState, "preference" | "resolved" | "initialized">) => void,
  preference: LocalePreference,
  resolved: AppLocale,
) {
  set({
    preference,
    resolved,
    initialized: true,
  });
  applyLanguage(resolved);
}

export const useLocaleStore = create<LocaleStore>((set, get) => ({
  preference: "system",
  resolved: resolveLocale("system"),
  initialized: false,
  init() {
    if (get().initialized) {
      return;
    }

    get().syncFromStorage();
    setupStorageListener();
    void get().syncFromBackend();
  },
  syncFromStorage() {
    const preference = getStoredLocalePreference();
    const resolved = resolveLocale(preference);
    const current = get();

    if (current.initialized && current.preference === preference && current.resolved === resolved) {
      return;
    }

    applyLocaleState(set, preference, resolved);
  },
  async syncFromBackend() {
    try {
      const state = await fetchBackendLocaleState();
      setStoredLocalePreference(state.preference);
      const current = get();
      if (current.initialized && current.preference === state.preference && current.resolved === state.resolved) {
        return;
      }

      applyLocaleState(set, state.preference, state.resolved);
    } catch (error) {
      logWarn("locale", "backend_sync_failed", {
        error: error instanceof Error ? error.message : String(error),
      });
    }
  },
  setPreference(preference) {
    setStoredLocalePreference(preference);
    const resolved = resolveLocale(preference);
    applyLocaleState(set, preference, resolved);

    void (async () => {
      try {
        const backendState = await saveBackendLocalePreference(preference);
        setStoredLocalePreference(backendState.preference);

        const current = get();
        if (current.preference === backendState.preference && current.resolved === backendState.resolved) {
          return;
        }

        applyLocaleState(set, backendState.preference, backendState.resolved);
      } catch (error) {
        logWarn("locale", "backend_set_failed", {
          preference,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    })();
  },
  setLocale(locale) {
    get().setPreference(locale);
  },
}));

export const localeActions: LocaleActions = {
  init: () => useLocaleStore.getState().init(),
  syncFromStorage: () => useLocaleStore.getState().syncFromStorage(),
  syncFromBackend: () => useLocaleStore.getState().syncFromBackend(),
  setPreference: (preference) => useLocaleStore.getState().setPreference(preference),
  setLocale: (locale) => useLocaleStore.getState().setLocale(locale),
};

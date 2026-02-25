import { create } from "zustand";

import i18n from "@/i18n";
import { resolveLocale } from "@/i18n/runtime";
import type { AppLocale, LocalePreference, LocaleState } from "@/i18n/types";
import { fetchBackendLocaleState, saveBackendLocalePreference } from "@/services/locale.service";
import { logWarn } from "@/services/logger";

interface LocaleActions {
  init: () => Promise<void>;
  syncFromStorage: () => Promise<void>;
  syncFromBackend: () => Promise<void>;
  setPreference: (preference: LocalePreference) => Promise<void>;
  setLocale: (locale: AppLocale) => Promise<void>;
}

type LocaleStore = LocaleState & LocaleActions;

function applyLocaleToDocument(locale: AppLocale) {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.lang = locale;
  document.documentElement.setAttribute("data-locale", locale);
}

function applyLanguage(locale: AppLocale) {
  applyLocaleToDocument(locale);
  void i18n.changeLanguage(locale);
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
  async init() {
    if (get().initialized) {
      return;
    }

    await get().syncFromBackend();
  },
  async syncFromStorage() {
    await get().syncFromBackend();
  },
  async syncFromBackend() {
    try {
      const state = await fetchBackendLocaleState();
      const current = get();
      if (current.initialized && current.preference === state.preference && current.resolved === state.resolved) {
        return;
      }

      applyLocaleState(set, state.preference, state.resolved);
    } catch (error) {
      logWarn("locale", "backend_sync_failed", {
        error: error instanceof Error ? error.message : String(error),
      });
      if (!get().initialized) {
        const fallbackPreference: LocalePreference = "system";
        applyLocaleState(set, fallbackPreference, resolveLocale(fallbackPreference));
      }
    }
  },
  async setPreference(preference) {
    const optimisticResolved = resolveLocale(preference);
    applyLocaleState(set, preference, optimisticResolved);

    try {
      const backendState = await saveBackendLocalePreference(preference);
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
      await get().syncFromBackend();
    }
  },
  async setLocale(locale) {
    await get().setPreference(locale);
  },
}));

export const localeActions: LocaleActions = {
  init: () => useLocaleStore.getState().init(),
  syncFromStorage: () => useLocaleStore.getState().syncFromStorage(),
  syncFromBackend: () => useLocaleStore.getState().syncFromBackend(),
  setPreference: (preference) => useLocaleStore.getState().setPreference(preference),
  setLocale: (locale) => useLocaleStore.getState().setLocale(locale),
};

import { create } from "zustand";

import type { SettingsDto } from "@/contracts";
import i18n from "@/i18n";
import { resolveLocale } from "@/i18n/runtime";
import type { AppLocale, LocalePreference, LocaleState } from "@/i18n/types";
import {
  type BackendLocaleState,
  fetchBackendLocaleState,
  saveBackendLocalePreference,
} from "@/services/locale.service";
import { logWarn } from "@/services/logger";
import { getSettings } from "@/services/settings.service";

interface LocaleActions {
  init: () => Promise<void>;
  syncFromStorage: () => Promise<void>;
  syncFromBackend: () => Promise<void>;
  hydrateFromSettings: (settings: SettingsDto) => void;
  hydrateFromBackendState: (state: BackendLocaleState) => void;
  setPreference: (preference: LocalePreference) => Promise<void>;
  setLocale: (locale: AppLocale) => Promise<void>;
}

type LocaleStore = LocaleState & LocaleActions;
const BACKEND_SYNC_MIN_INTERVAL_MS = 2_000;
let lastBackendSyncAt = 0;

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

function normalizeLocalePreference(preference: string | undefined): LocalePreference {
  const normalized = preference?.trim();
  if (!normalized) {
    return "system";
  }
  return normalized as LocalePreference;
}

export const useLocaleStore = create<LocaleStore>((set, get) => ({
  preference: "system",
  resolved: resolveLocale("system"),
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
      logWarn("locale", "storage_sync_failed", {
        error: error instanceof Error ? error.message : String(error),
      });
      if (!get().initialized) {
        const fallbackPreference: LocalePreference = "system";
        applyLocaleState(set, fallbackPreference, resolveLocale(fallbackPreference));
      }
    }
  },
  async syncFromBackend() {
    const now = Date.now();
    if (get().initialized && now - lastBackendSyncAt < BACKEND_SYNC_MIN_INTERVAL_MS) {
      return;
    }

    try {
      const state = await fetchBackendLocaleState();
      get().hydrateFromBackendState(state);
      lastBackendSyncAt = now;
    } catch (error) {
      lastBackendSyncAt = now;
      logWarn("locale", "backend_sync_failed", {
        error: error instanceof Error ? error.message : String(error),
      });
      if (!get().initialized) {
        const fallbackPreference: LocalePreference = "system";
        applyLocaleState(set, fallbackPreference, resolveLocale(fallbackPreference));
      }
    }
  },
  hydrateFromSettings(settings) {
    const preference = normalizeLocalePreference(settings.locale.preference);
    const resolved = resolveLocale(preference);
    const current = get();
    if (current.initialized && current.preference === preference && current.resolved === resolved) {
      return;
    }
    applyLocaleState(set, preference, resolved);
  },
  hydrateFromBackendState(state) {
    const preference = normalizeLocalePreference(state.preference);
    const resolved = resolveLocale(state.resolved);
    const current = get();
    if (current.initialized && current.preference === preference && current.resolved === resolved) {
      lastBackendSyncAt = Date.now();
      return;
    }
    lastBackendSyncAt = Date.now();
    applyLocaleState(set, preference, resolved);
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

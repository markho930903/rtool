import { create } from "zustand";

import type { SettingsDto } from "@/contracts";
import type { LayoutPreference, LayoutState } from "@/layouts/layout.types";
import { logWarn } from "@/services/logger";
import { getSettings, patchSettings } from "@/services/settings.service";

const DEFAULT_LAYOUT_PREFERENCE: LayoutPreference = "topbar";

function normalizeLayoutPreference(value: string | undefined): LayoutPreference {
  return value === "sidebar" ? "sidebar" : "topbar";
}

interface LayoutActions {
  init: () => Promise<void>;
  syncFromStorage: () => Promise<void>;
  hydrateFromSettings: (settings: SettingsDto) => void;
  setPreference: (preference: LayoutPreference) => Promise<void>;
}

type LayoutStore = LayoutState & LayoutActions;

export const useLayoutStore = create<LayoutStore>((set, get) => ({
  preference: DEFAULT_LAYOUT_PREFERENCE,
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
      logWarn("layout", "sync_from_backend_failed", {
        error: error instanceof Error ? error.message : String(error),
      });
      if (!get().initialized) {
        set({
          preference: DEFAULT_LAYOUT_PREFERENCE,
          initialized: true,
        });
      }
    }
  },
  hydrateFromSettings(settings) {
    const preference = normalizeLayoutPreference(settings.layout.preference);
    const current = get();
    if (current.initialized && current.preference === preference) {
      return;
    }
    set({
      preference,
      initialized: true,
    });
  },
  async setPreference(preference) {
    const canonical = normalizeLayoutPreference(preference);
    set({
      preference: canonical,
      initialized: true,
    });
    try {
      const settings = await patchSettings({
        layout: {
          preference: canonical,
        },
      });
      get().hydrateFromSettings(settings);
    } catch (error) {
      logWarn("layout", "set_preference_failed", {
        preference: canonical,
        error: error instanceof Error ? error.message : String(error),
      });
      await get().syncFromStorage();
    }
  },
}));

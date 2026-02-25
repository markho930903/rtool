import { create } from "zustand";

import type { LayoutPreference, LayoutState } from "@/layouts/layout.types";
import { logWarn } from "@/services/logger";
import { getUserSettings, patchUserSettings } from "@/services/user-settings.service";

const DEFAULT_LAYOUT_PREFERENCE: LayoutPreference = "topbar";

function normalizeLayoutPreference(value: string | undefined): LayoutPreference {
  return value === "sidebar" ? "sidebar" : "topbar";
}

interface LayoutActions {
  init: () => Promise<void>;
  syncFromStorage: () => Promise<void>;
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
      const settings = await getUserSettings();
      const preference = normalizeLayoutPreference(settings.layout.preference);
      set({
        preference,
        initialized: true,
      });
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
  async setPreference(preference) {
    const canonical = normalizeLayoutPreference(preference);
    set({
      preference: canonical,
      initialized: true,
    });
    try {
      const settings = await patchUserSettings({
        layout: {
          preference: canonical,
        },
      });
      set({
        preference: normalizeLayoutPreference(settings.layout.preference),
        initialized: true,
      });
    } catch (error) {
      logWarn("layout", "set_preference_failed", {
        preference: canonical,
        error: error instanceof Error ? error.message : String(error),
      });
      await get().syncFromStorage();
    }
  },
}));

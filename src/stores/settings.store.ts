import { create } from "zustand";

import {
  getFreshStartupSettings,
  getPendingStartupSettingsRequest,
} from "@/services/startup-settings-cache";
import { getSettings, patchSettings } from "@/services/settings.service";

interface ClipboardSettings {
  maxItems: number;
  sizeCleanupEnabled: boolean;
  maxTotalSizeMb: number;
}

interface ClipboardSettingsUpdateInput {
  maxItems: number;
  sizeCleanupEnabled?: boolean;
  maxTotalSizeMb?: number;
}

interface SettingsState {
  clipboardSettings: ClipboardSettings | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
}

interface SettingsActions {
  fetchClipboardSettings: () => Promise<void>;
  updateClipboardSettings: (input: ClipboardSettingsUpdateInput) => Promise<void>;
}

type SettingsStore = SettingsState & SettingsActions;
const STARTUP_SETTINGS_MAX_AGE_MS = 15_000;

export const useSettingsStore = create<SettingsStore>((set) => ({
  clipboardSettings: null,
  loading: false,
  saving: false,
  error: null,

  async fetchClipboardSettings() {
    set({ loading: true, error: null });
    try {
      const freshSettings = getFreshStartupSettings(STARTUP_SETTINGS_MAX_AGE_MS);
      const pendingSettingsRequest = getPendingStartupSettingsRequest();
      const settings =
        freshSettings ??
        (pendingSettingsRequest ? await pendingSettingsRequest : await getSettings());
      set({ clipboardSettings: settings.clipboard, loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ loading: false, error: message });
    }
  },

  async updateClipboardSettings(input) {
    set({ saving: true, error: null });
    try {
      const settings = await patchSettings({
        clipboard: {
          maxItems: input.maxItems,
          sizeCleanupEnabled: input.sizeCleanupEnabled,
          maxTotalSizeMb: input.maxTotalSizeMb,
        },
      });
      set({ clipboardSettings: settings.clipboard, saving: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ saving: false, error: message });
      throw error;
    }
  },
}));

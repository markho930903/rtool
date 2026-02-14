import { create } from "zustand";

import { invokeWithLog } from "@/services/invoke";

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

export const useSettingsStore = create<SettingsStore>((set) => ({
  clipboardSettings: null,
  loading: false,
  saving: false,
  error: null,

  async fetchClipboardSettings() {
    set({ loading: true, error: null });
    try {
      const settings = await invokeWithLog<ClipboardSettings>("clipboard_get_settings");
      set({ clipboardSettings: settings, loading: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ loading: false, error: message });
    }
  },

  async updateClipboardSettings(input) {
    set({ saving: true, error: null });
    try {
      const settings = await invokeWithLog<ClipboardSettings>("clipboard_update_settings", {
        maxItems: input.maxItems,
        sizeCleanupEnabled: input.sizeCleanupEnabled,
        maxTotalSizeMb: input.maxTotalSizeMb,
      });
      set({ clipboardSettings: settings, saving: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ saving: false, error: message });
      throw error;
    }
  },
}));

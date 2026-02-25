import { create } from "zustand";

import type { LauncherAction, PaletteActionResult, PaletteItem } from "@/components/palette/types";
import { invokeWithLog } from "@/services/invoke";

interface LauncherState {
  query: string;
  items: PaletteItem[];
  selectedIndex: number;
  loading: boolean;
  error: string | null;
  lastAction: PaletteActionResult | null;
}

interface LauncherActions {
  reset: () => void;
  setQuery: (query: string) => void;
  moveSelection: (delta: number) => void;
  setSelectedIndex: (index: number) => void;
  search: (limit?: number) => Promise<void>;
  executeSelected: () => Promise<PaletteActionResult | null>;
}

type LauncherStore = LauncherState & LauncherActions;
let latestSearchRequestVersion = 0;

export const useLauncherStore = create<LauncherStore>((set, get) => ({
  query: "",
  items: [],
  selectedIndex: 0,
  loading: false,
  error: null,
  lastAction: null,
  reset() {
    latestSearchRequestVersion += 1;
    set({
      query: "",
      items: [],
      selectedIndex: 0,
      loading: false,
      error: null,
      lastAction: null,
    });
  },
  setQuery(query) {
    set({ query, selectedIndex: 0 });
  },
  moveSelection(delta) {
    const count = get().items.length;
    if (count <= 0) {
      return;
    }

    const next = (get().selectedIndex + delta + count) % count;
    set({ selectedIndex: next });
  },
  setSelectedIndex(index) {
    if (index < 0 || index >= get().items.length) {
      return;
    }
    set({ selectedIndex: index });
  },
  async search(limit?: number) {
    const requestVersion = ++latestSearchRequestVersion;
    const query = get().query;
    set({ loading: true, error: null });

    try {
      const items = await invokeWithLog<PaletteItem[]>("launcher_search", {
        query,
        limit,
      });

      if (requestVersion !== latestSearchRequestVersion || query !== get().query) {
        return;
      }

      set((prev) => ({
        items,
        selectedIndex: items.length === 0 ? 0 : Math.min(prev.selectedIndex, items.length - 1),
        loading: false,
      }));
    } catch (error) {
      if (requestVersion !== latestSearchRequestVersion) {
        return;
      }
      const message = error instanceof Error ? error.message : String(error);
      set({ loading: false, error: message });
    }
  },
  async executeSelected(): Promise<PaletteActionResult | null> {
    const state = get();
    const selected = state.items[state.selectedIndex];
    if (!selected?.action) {
      return null;
    }
    try {
      const result = await invokeWithLog<PaletteActionResult>("launcher_execute", {
        action: selected.action as LauncherAction,
      });

      set({
        lastAction: result,
        error: result.ok ? null : result.message,
      });

      return result;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ error: message });
      return null;
    }
  },
}));

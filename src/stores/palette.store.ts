import { create } from "zustand";

import type { PaletteActionResult, PaletteItem } from "@/components/palette/types";
import { invokeWithLog } from "@/services/invoke";

interface PaletteState {
  isOpen: boolean;
  query: string;
  items: PaletteItem[];
  selectedIndex: number;
  loading: boolean;
  error: string | null;
  lastAction: PaletteActionResult | null;
}

interface PaletteActions {
  open: () => void;
  close: () => void;
  toggle: (forceState?: boolean) => void;
  setQuery: (query: string) => void;
  moveSelection: (delta: number) => void;
  setSelectedIndex: (index: number) => void;
  search: () => Promise<void>;
  executeSelected: () => Promise<PaletteActionResult | null>;
}

type PaletteStore = PaletteState & PaletteActions;

export const usePaletteStore = create<PaletteStore>((set, get) => ({
  isOpen: false,
  query: "",
  items: [],
  selectedIndex: 0,
  loading: false,
  error: null,
  lastAction: null,
  open() {
    set({ isOpen: true, error: null });
  },
  close() {
    set({
      isOpen: false,
      query: "",
      items: [],
      selectedIndex: 0,
      loading: false,
      error: null,
    });
  },
  toggle(forceState?: boolean) {
    if (typeof forceState === "boolean") {
      if (forceState) {
        get().open();
      } else {
        get().close();
      }
      return;
    }

    if (get().isOpen) {
      get().close();
    } else {
      get().open();
    }
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
  async search() {
    const query = get().query;
    set({ loading: true, error: null });

    try {
      const items = await invokeWithLog<PaletteItem[]>("palette_search", {
        query,
      });

      if (query !== get().query) {
        return;
      }

      set((prev) => ({
        items,
        selectedIndex: items.length === 0 ? 0 : Math.min(prev.selectedIndex, items.length - 1),
        loading: false,
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ loading: false, error: message });
    }
  },
  async executeSelected(): Promise<PaletteActionResult | null> {
    const state = get();
    const selected = state.items[state.selectedIndex];
    if (!selected) {
      return null;
    }
    try {
      const result = await invokeWithLog<PaletteActionResult>("palette_execute", {
        actionId: selected.id,
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

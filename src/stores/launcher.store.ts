import { create } from "zustand";

import type { LauncherAction, PaletteActionResult, PaletteItem } from "@/components/palette/types";
import {
  launcherExecute,
  launcherSearch,
  type LauncherSearchDiagnostics,
  type LauncherSearchIndexState,
  type LauncherSearchResponse,
} from "@/services/launcher.service";

interface LauncherState {
  query: string;
  result: LauncherSearchResponse | null;
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
const EMPTY_LAUNCHER_ITEMS: PaletteItem[] = [];

function getLauncherItems(result: LauncherSearchResponse | null): PaletteItem[] {
  return (result?.items ?? EMPTY_LAUNCHER_ITEMS) as PaletteItem[];
}

export function selectLauncherItems(state: LauncherStore): PaletteItem[] {
  return getLauncherItems(state.result);
}

export function selectLauncherSearchStatus(state: LauncherStore): LauncherSearchIndexState | null {
  return state.result?.index ?? null;
}

export function selectLauncherSearchDiagnostics(state: LauncherStore): LauncherSearchDiagnostics | null {
  return state.result?.diagnostics ?? null;
}

export const useLauncherStore = create<LauncherStore>((set, get) => ({
  query: "",
  result: null,
  selectedIndex: 0,
  loading: false,
  error: null,
  lastAction: null,
  reset() {
    latestSearchRequestVersion += 1;
    set({
      query: "",
      result: null,
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
    const count = selectLauncherItems(get()).length;
    if (count <= 0) {
      return;
    }

    const next = (get().selectedIndex + delta + count) % count;
    set({ selectedIndex: next });
  },
  setSelectedIndex(index) {
    const currentState = get();
    if (index < 0 || index >= selectLauncherItems(currentState).length) {
      return;
    }
    if (index === currentState.selectedIndex) {
      return;
    }
    set({ selectedIndex: index });
  },
  async search(limit?: number) {
    const requestVersion = ++latestSearchRequestVersion;
    const query = get().query;
    set({ loading: true, error: null });

    try {
      const result = await launcherSearch(query, limit);
      const items = getLauncherItems(result);

      if (requestVersion !== latestSearchRequestVersion || query !== get().query) {
        return;
      }

      set((prev) => ({
        result,
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
    const selected = selectLauncherItems(state)[state.selectedIndex];
    if (!selected?.action) {
      return null;
    }
    try {
      const result = (await launcherExecute(selected.action as LauncherAction)) as PaletteActionResult;

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

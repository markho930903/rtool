import { create } from "zustand";

import type { WindowMode } from "@/stores/types";

interface AppState {
  windowMode: WindowMode;
}

interface AppActions {
  setWindowMode: (windowMode: WindowMode) => void;
}

type AppStore = AppState & AppActions;

export const useAppStore = create<AppStore>((set) => ({
  windowMode: "app-manager",
  setWindowMode(windowMode) {
    set({ windowMode });
  },
}));

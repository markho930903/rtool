import { create } from "zustand";

import { getHomeModuleRouteConfig, type AppRouteId } from "@/routers/routes.config";
import {
  fetchAppHealthSnapshot,
  fetchDashboardSnapshot,
  type AppHealthSnapshot,
  type DashboardSnapshot,
} from "@/services/dashboard.service";

const POLLING_INTERVAL_MS = 3000;
const HISTORY_LIMIT = 20;

let pollingTimer: number | null = null;
let inFlightRefresh: Promise<void> | null = null;

export interface DashboardHistoryPoint {
  sampledAt: number;
  appMemoryBytes: number | null;
  systemUsedMemoryBytes: number | null;
}

export interface DashboardModuleStatusItem {
  id: AppRouteId;
  nameKey: string;
  detailKey: string;
  state: "online" | "booting" | "degraded";
}

interface DashboardState {
  snapshot: DashboardSnapshot | null;
  healthSnapshot: AppHealthSnapshot | null;
  history: DashboardHistoryPoint[];
  loading: boolean;
  error: string | null;
  lastUpdatedAt: number | null;
}

interface DashboardActions {
  refresh: () => Promise<void>;
  startPolling: () => void;
  stopPolling: () => void;
  getModuleStatusItems: () => DashboardModuleStatusItem[];
}

type DashboardStore = DashboardState & DashboardActions;

function toHistoryPoint(snapshot: DashboardSnapshot): DashboardHistoryPoint {
  return {
    sampledAt: snapshot.sampledAt,
    appMemoryBytes: snapshot.app.processMemoryBytes,
    systemUsedMemoryBytes: snapshot.system.usedMemoryBytes,
  };
}

function appendHistory(history: DashboardHistoryPoint[], nextPoint: DashboardHistoryPoint): DashboardHistoryPoint[] {
  const deduped = history.filter((point) => point.sampledAt !== nextPoint.sampledAt);
  return [...deduped, nextPoint].slice(-HISTORY_LIMIT);
}

export const useDashboardStore = create<DashboardStore>((set, get) => ({
  snapshot: null,
  healthSnapshot: null,
  history: [],
  loading: false,
  error: null,
  lastUpdatedAt: null,
  async refresh() {
    if (inFlightRefresh) {
      return inFlightRefresh;
    }

    const run = async () => {
      const shouldShowLoading = get().snapshot === null;
      if (shouldShowLoading) {
        set({ loading: true, error: null });
      } else {
        set({ error: null });
      }

      try {
        const [snapshot, healthSnapshot] = await Promise.all([
          fetchDashboardSnapshot(),
          fetchAppHealthSnapshot(),
        ]);
        const nextHistoryPoint = toHistoryPoint(snapshot);
        set((state) => ({
          snapshot,
          healthSnapshot,
          history: appendHistory(state.history, nextHistoryPoint),
          loading: false,
          error: null,
          lastUpdatedAt: Date.now(),
        }));
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        set({ loading: false, error: message });
      } finally {
        inFlightRefresh = null;
      }
    };

    inFlightRefresh = run();
    return inFlightRefresh;
  },
  startPolling() {
    if (pollingTimer !== null) {
      return;
    }

    void get().refresh();
    pollingTimer = window.setInterval(() => {
      void get().refresh();
    }, POLLING_INTERVAL_MS);
  },
  stopPolling() {
    if (pollingTimer === null) {
      return;
    }

    window.clearInterval(pollingTimer);
    pollingTimer = null;
  },
  getModuleStatusItems() {
    const health = get().healthSnapshot;
    return getHomeModuleRouteConfig().map((item) => {
      if (item.id === "launcher" && health) {
        const state: DashboardModuleStatusItem["state"] =
          health.launcher.started && !health.launcher.building ? "online" : "booting";
        return {
          id: item.id,
          nameKey: item.nameKey,
          detailKey: item.detailKey,
          state,
        };
      }
      if (item.id === "transfer" && health) {
        const state: DashboardModuleStatusItem["state"] = health.transfer.listenerStarted ? "online" : "degraded";
        return {
          id: item.id,
          nameKey: item.nameKey,
          detailKey: item.detailKey,
          state,
        };
      }
      return {
        id: item.id,
        nameKey: item.nameKey,
        detailKey: item.detailKey,
        state: item.state,
      };
    });
  },
}));

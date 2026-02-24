import { create } from "zustand";

import type { ResourcePointDto } from "@/contracts";
import {
  fetchResourceMonitorHistory,
  fetchResourceMonitorSnapshot,
  resetResourceMonitorSession,
  type ResourceSnapshot,
} from "@/services/resource-monitor.service";

const POLLING_INTERVAL_MS = 1000;
const DEFAULT_HISTORY_LIMIT = 1800;

let pollingTimer: number | null = null;
let inFlightSnapshot: Promise<void> | null = null;
let inFlightBootstrap: Promise<void> | null = null;

export type ResourceSortMetric = "cpu" | "memory" | "calls";

interface ResourceMonitorState {
  initialized: boolean;
  loading: boolean;
  error: string | null;
  snapshot: ResourceSnapshot | null;
  history: ResourcePointDto[];
  lastUpdatedAt: number | null;
  historyWindowMinutes: 5 | 15 | 30;
  sortMetric: ResourceSortMetric;
}

interface ResourceMonitorActions {
  initialize: () => Promise<void>;
  refreshSnapshot: () => Promise<void>;
  refreshAll: () => Promise<void>;
  startPolling: () => void;
  stopPolling: () => void;
  setHistoryWindowMinutes: (minutes: 5 | 15 | 30) => void;
  setSortMetric: (metric: ResourceSortMetric) => void;
  resetSession: () => Promise<void>;
}

type ResourceMonitorStore = ResourceMonitorState & ResourceMonitorActions;

function appendHistory(history: ResourcePointDto[], point: ResourcePointDto): ResourcePointDto[] {
  const deduped = history.filter((item) => item.sampledAt !== point.sampledAt);
  const next = [...deduped, point].sort((left, right) => left.sampledAt - right.sampledAt);
  return next.slice(-DEFAULT_HISTORY_LIMIT);
}

function snapshotToPoint(snapshot: ResourceSnapshot): ResourcePointDto {
  return {
    sampledAt: snapshot.overview.sampledAt,
    processCpuPercent: snapshot.overview.processCpuPercent,
    processMemoryBytes: snapshot.overview.processMemoryBytes,
    systemUsedMemoryBytes: snapshot.overview.systemUsedMemoryBytes,
    systemTotalMemoryBytes: snapshot.overview.systemTotalMemoryBytes,
  };
}

export const useResourceMonitorStore = create<ResourceMonitorStore>((set, get) => ({
  initialized: false,
  loading: false,
  error: null,
  snapshot: null,
  history: [],
  lastUpdatedAt: null,
  historyWindowMinutes: 30,
  sortMetric: "cpu",

  async initialize() {
    if (get().initialized) {
      return;
    }
    if (inFlightBootstrap) {
      return inFlightBootstrap;
    }

    const run = async () => {
      set({ loading: true, error: null });
      try {
        const [history, snapshot] = await Promise.all([
          fetchResourceMonitorHistory(DEFAULT_HISTORY_LIMIT),
          fetchResourceMonitorSnapshot(),
        ]);
        const seedHistory = appendHistory(history.points, snapshotToPoint(snapshot));
        set({
          initialized: true,
          loading: false,
          error: null,
          snapshot,
          history: seedHistory,
          lastUpdatedAt: Date.now(),
        });
      } catch (error) {
        set({
          loading: false,
          error: error instanceof Error ? error.message : String(error),
        });
      } finally {
        inFlightBootstrap = null;
      }
    };

    inFlightBootstrap = run();
    return inFlightBootstrap;
  },

  async refreshSnapshot() {
    if (inFlightSnapshot) {
      return inFlightSnapshot;
    }

    const run = async () => {
      try {
        const snapshot = await fetchResourceMonitorSnapshot();
        const point = snapshotToPoint(snapshot);
        set((state) => ({
          snapshot,
          history: appendHistory(state.history, point),
          lastUpdatedAt: Date.now(),
          error: null,
        }));
      } catch (error) {
        set({ error: error instanceof Error ? error.message : String(error) });
      } finally {
        inFlightSnapshot = null;
      }
    };

    inFlightSnapshot = run();
    return inFlightSnapshot;
  },

  async refreshAll() {
    set({ loading: true, error: null });
    try {
      const [history, snapshot] = await Promise.all([
        fetchResourceMonitorHistory(DEFAULT_HISTORY_LIMIT),
        fetchResourceMonitorSnapshot(),
      ]);
      const seedHistory = appendHistory(history.points, snapshotToPoint(snapshot));
      set({
        snapshot,
        history: seedHistory,
        loading: false,
        error: null,
        lastUpdatedAt: Date.now(),
      });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  },

  startPolling() {
    if (pollingTimer !== null) {
      return;
    }
    void get().initialize();
    pollingTimer = window.setInterval(() => {
      void get().refreshSnapshot();
    }, POLLING_INTERVAL_MS);
  },

  stopPolling() {
    if (pollingTimer === null) {
      return;
    }
    window.clearInterval(pollingTimer);
    pollingTimer = null;
  },

  setHistoryWindowMinutes(historyWindowMinutes) {
    set({ historyWindowMinutes });
  },

  setSortMetric(sortMetric) {
    set({ sortMetric });
  },

  async resetSession() {
    set({ loading: true, error: null });
    try {
      await resetResourceMonitorSession();
      const [history, snapshot] = await Promise.all([
        fetchResourceMonitorHistory(DEFAULT_HISTORY_LIMIT),
        fetchResourceMonitorSnapshot(),
      ]);
      const seedHistory = appendHistory(history.points, snapshotToPoint(snapshot));
      set({
        loading: false,
        error: null,
        snapshot,
        history: seedHistory,
        lastUpdatedAt: Date.now(),
      });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  },
}));

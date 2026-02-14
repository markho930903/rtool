import type { UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";

import {
  exportLogs,
  fetchLogPage,
  fetchLoggingConfig,
  saveLoggingConfig,
  subscribeLogStream,
  type LogEntry,
  type LogLevel,
  type LoggingConfig,
} from "@/services/logging.service";

const QUERY_LIMIT = 100;
const MAX_STREAM_ITEMS = 2000;

interface LogFilters {
  levels: LogLevel[];
  scope: string;
  requestId: string;
  windowLabel: string;
  keyword: string;
  startAt: number | null;
  endAt: number | null;
}

interface LoggingState {
  items: LogEntry[];
  nextCursor: string | null;
  loading: boolean;
  loadingMore: boolean;
  exporting: boolean;
  streamConnected: boolean;
  error: string | null;
  selectedLogId: number | null;
  config: LoggingConfig | null;
  filters: LogFilters;
  lastExportPath: string | null;
}

interface LoggingActions {
  fetchConfig: () => Promise<void>;
  saveConfig: (config: LoggingConfig) => Promise<void>;
  setFilters: (patch: Partial<LogFilters>) => void;
  resetFilters: () => void;
  refresh: () => Promise<void>;
  loadMore: () => Promise<void>;
  startStream: () => Promise<void>;
  stopStream: () => void;
  selectLog: (id: number | null) => void;
  exportCurrentQuery: (outputPath?: string) => Promise<string>;
}

type LoggingStore = LoggingState & LoggingActions;

let streamUnlisten: UnlistenFn | null = null;

const DEFAULT_FILTERS: LogFilters = {
  levels: ["info", "warn", "error"],
  scope: "",
  requestId: "",
  windowLabel: "",
  keyword: "",
  startAt: null,
  endAt: null,
};

function buildQuery(filters: LogFilters, cursor?: string) {
  return {
    cursor,
    limit: QUERY_LIMIT,
    levels: filters.levels.length > 0 ? filters.levels : undefined,
    scope: filters.scope.trim() || undefined,
    requestId: filters.requestId.trim() || undefined,
    windowLabel: filters.windowLabel.trim() || undefined,
    keyword: filters.keyword.trim() || undefined,
    startAt: filters.startAt ?? undefined,
    endAt: filters.endAt ?? undefined,
  };
}

function entryMatchesFilters(entry: LogEntry, filters: LogFilters): boolean {
  if (filters.levels.length > 0 && !filters.levels.includes(entry.level)) {
    return false;
  }

  if (filters.scope.trim() && entry.scope !== filters.scope.trim()) {
    return false;
  }

  if (filters.requestId.trim() && entry.requestId !== filters.requestId.trim()) {
    return false;
  }

  if (filters.windowLabel.trim() && entry.windowLabel !== filters.windowLabel.trim()) {
    return false;
  }

  if (filters.startAt !== null && entry.timestamp < filters.startAt) {
    return false;
  }

  if (filters.endAt !== null && entry.timestamp > filters.endAt) {
    return false;
  }

  const keyword = filters.keyword.trim().toLowerCase();
  if (!keyword) {
    return true;
  }

  const messageHit = entry.message.toLowerCase().includes(keyword);
  const eventHit = entry.event.toLowerCase().includes(keyword);
  const metadataHit = entry.metadata ? JSON.stringify(entry.metadata).toLowerCase().includes(keyword) : false;
  return messageHit || eventHit || metadataHit;
}

function mergeStreamEntry(items: LogEntry[], entry: LogEntry): LogEntry[] {
  const existedIndex = items.findIndex((item) => item.id === entry.id);
  if (existedIndex >= 0) {
    const next = [...items];
    next[existedIndex] = entry;
    return next;
  }

  const next = [entry, ...items];
  return next.slice(0, MAX_STREAM_ITEMS);
}

export const useLoggingStore = create<LoggingStore>((set, get) => ({
  items: [],
  nextCursor: null,
  loading: false,
  loadingMore: false,
  exporting: false,
  streamConnected: false,
  error: null,
  selectedLogId: null,
  config: null,
  filters: { ...DEFAULT_FILTERS },
  lastExportPath: null,

  async fetchConfig() {
    try {
      const config = await fetchLoggingConfig();
      set({ config });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ error: message });
    }
  },

  async saveConfig(config) {
    try {
      const next = await saveLoggingConfig(config);
      set({ config: next, error: null });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ error: message });
      throw error;
    }
  },

  setFilters(patch) {
    set((state) => ({
      filters: {
        ...state.filters,
        ...patch,
      },
    }));
  },

  resetFilters() {
    set({ filters: { ...DEFAULT_FILTERS } });
  },

  async refresh() {
    set((state) => ({ loading: state.items.length === 0, error: null }));

    try {
      const query = buildQuery(get().filters);
      const page = await fetchLogPage(query);
      set({
        items: page.items,
        nextCursor: page.nextCursor,
        loading: false,
        error: null,
        selectedLogId: page.items[0]?.id ?? null,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ loading: false, error: message });
    }
  },

  async loadMore() {
    const { nextCursor, loadingMore } = get();
    if (!nextCursor || loadingMore) {
      return;
    }

    set({ loadingMore: true, error: null });
    try {
      const query = buildQuery(get().filters, nextCursor);
      const page = await fetchLogPage(query);
      set((state) => ({
        items: [...state.items, ...page.items],
        nextCursor: page.nextCursor,
        loadingMore: false,
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ loadingMore: false, error: message });
    }
  },

  async startStream() {
    if (streamUnlisten) {
      return;
    }

    streamUnlisten = await subscribeLogStream((entry) => {
      const { filters, config } = get();
      if (config && !config.realtimeEnabled) {
        return;
      }

      if (!entryMatchesFilters(entry, filters)) {
        return;
      }

      set((state) => ({
        items: mergeStreamEntry(state.items, entry),
      }));
    });

    set({ streamConnected: true });
  },

  stopStream() {
    if (!streamUnlisten) {
      return;
    }

    const release = streamUnlisten;
    streamUnlisten = null;
    release();
    set({ streamConnected: false });
  },

  selectLog(id) {
    set({ selectedLogId: id });
  },

  async exportCurrentQuery(outputPath) {
    set({ exporting: true, error: null });
    try {
      const query = buildQuery(get().filters);
      const exportedPath = await exportLogs(query, outputPath);
      set({ exporting: false, lastExportPath: exportedPath });
      return exportedPath;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      set({ exporting: false, error: message });
      throw error;
    }
  },
}));

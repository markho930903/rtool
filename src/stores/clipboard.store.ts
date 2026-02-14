import { create } from "zustand";

import type { ClipboardFilter, ClipboardItem, ClipboardSyncPayload } from "@/components/clipboard/types";
import { invokeWithLog } from "@/services/invoke";
import { runRecoverable } from "@/services/recoverable";

interface ClipboardState {
  items: ClipboardItem[];
  loading: boolean;
  initializing: boolean;
  initialized: boolean;
  initError: string | null;
  query: string;
  itemType: string;
  onlyPinned: boolean;
  error: string | null;
}

interface ClipboardActions {
  setQuery: (query: string) => void;
  setItemType: (itemType: string) => void;
  setOnlyPinned: (onlyPinned: boolean) => void;
  ensureInitialized: () => Promise<void>;
  applySync: (payload: ClipboardSyncPayload) => void;
  pinItem: (id: string, pinned: boolean) => Promise<void>;
  deleteItem: (id: string) => Promise<void>;
  clearAllItems: () => Promise<void>;
  copyBack: (id: string) => Promise<void>;
  copyFilePathsBack: (id: string) => Promise<void>;
  copyImageBack: (id: string) => Promise<void>;
  upsertItem: (item: ClipboardItem) => void;
}

type ClipboardStore = ClipboardState & ClipboardActions;

interface ClipboardSettings {
  maxItems: number;
  sizeCleanupEnabled: boolean;
  maxTotalSizeMb: number;
}

function compareClipboardItems(left: ClipboardItem, right: ClipboardItem): number {
  if (left.pinned !== right.pinned) {
    return left.pinned ? -1 : 1;
  }
  return right.createdAt - left.createdAt;
}

function upsertSingleItemFast(items: ClipboardItem[], incoming: ClipboardItem): ClipboardItem[] {
  const existingIndex = items.findIndex((item) => item.id === incoming.id);
  const nextItems = [...items];
  if (existingIndex !== -1) {
    nextItems.splice(existingIndex, 1);
  }

  const insertAt = nextItems.findIndex((item) => compareClipboardItems(incoming, item) < 0);
  if (insertAt === -1) {
    nextItems.push(incoming);
    return nextItems;
  }

  nextItems.splice(insertAt, 0, incoming);
  return nextItems;
}

function sortItems(items: ClipboardItem[]): ClipboardItem[] {
  return [...items].sort(compareClipboardItems);
}

export const useClipboardStore = create<ClipboardStore>((set, get) => ({
  items: [],
  loading: false,
  initializing: false,
  initialized: false,
  initError: null,
  query: "",
  itemType: "",
  onlyPinned: false,
  error: null,
  setQuery(query) {
    set({ query });
  },
  setItemType(itemType) {
    set({ itemType });
  },
  setOnlyPinned(onlyPinned) {
    set({ onlyPinned });
  },
  async ensureInitialized() {
    const state = get();
    if (state.initialized || state.initializing) {
      return;
    }

    set({ loading: true, initializing: true, initError: null, error: null });
    const result = await runRecoverable(
      async () => {
        const settings = await invokeWithLog<ClipboardSettings>("clipboard_get_settings");
        const filter: ClipboardFilter = {
          limit: Math.max(1, settings.maxItems || 1),
          onlyPinned: false,
        };
        const items = await invokeWithLog<ClipboardItem[]>("clipboard_list", { filter });
        return sortItems(items);
      },
      {
        scope: "clipboard-store",
        action: "ensure_initialized",
        message: "initialize clipboard failed",
      },
    );

    if (!result.ok) {
      set({
        loading: false,
        initializing: false,
        initialized: false,
        initError: result.message,
        error: result.message,
      });
      return;
    }

    set({
      items: result.data,
      loading: false,
      initializing: false,
      initialized: true,
      initError: null,
      error: null,
    });
  },
  applySync(payload) {
    const upsert = payload.upsert ?? [];
    const removedIds = payload.removedIds ?? [];
    const clearAll = payload.clearAll === true;
    if (!clearAll && upsert.length === 0 && removedIds.length === 0) {
      return;
    }

    set((prev) => {
      let nextItems = clearAll ? [] : prev.items;
      if (removedIds.length > 0) {
        const removedIdSet = new Set(removedIds);
        nextItems = nextItems.filter((item) => !removedIdSet.has(item.id));
      }

      if (!clearAll && removedIds.length === 0 && upsert.length === 1) {
        return {
          items: upsertSingleItemFast(nextItems, upsert[0]),
        };
      }

      if (upsert.length > 0) {
        const merged = new Map(nextItems.map((item) => [item.id, item] as const));
        for (const item of upsert) {
          merged.set(item.id, item);
        }
        nextItems = Array.from(merged.values());
      }

      return {
        items: sortItems(nextItems),
      };
    });
  },
  async pinItem(id, pinned) {
    await invokeWithLog("clipboard_pin", { id, pinned });
  },
  async deleteItem(id) {
    await invokeWithLog("clipboard_delete", { id });
  },
  async clearAllItems() {
    await invokeWithLog("clipboard_clear_all");
  },
  async copyBack(id) {
    await invokeWithLog("clipboard_copy_back", { id });
  },
  async copyFilePathsBack(id) {
    await invokeWithLog("clipboard_copy_file_paths", { id });
  },
  async copyImageBack(id) {
    await invokeWithLog("clipboard_copy_image_back", { id });
  },
  upsertItem(item) {
    get().applySync({
      upsert: [item],
      removedIds: [],
      clearAll: false,
      reason: "manual_upsert",
    });
  },
}));

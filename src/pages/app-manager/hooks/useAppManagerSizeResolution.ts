import { type Dispatch, type SetStateAction, useCallback, useRef, useState } from "react";

import { useLatestRef } from "@/hooks/useLatestRef";
import { appManagerResolveSizes } from "@/services/app-manager.service";
import type { ManagedApp } from "@/components/app-manager/types";

import { delay, initialSizeState, retainById, type AppSizeState } from "./state";

const SIZE_BATCH = 10;

interface UseAppManagerSizeResolutionOptions {
  items: ManagedApp[];
  setItems: Dispatch<SetStateAction<ManagedApp[]>>;
  setListError: Dispatch<SetStateAction<string | null>>;
}

export interface MergeListItemsResult {
  mergedItems: ManagedApp[];
  keepIds: Set<string>;
}

export function useAppManagerSizeResolution(options: UseAppManagerSizeResolutionOptions) {
  const { items, setItems, setListError } = options;
  const [sizeStateByAppId, setSizeStateByAppId] = useState<Record<string, AppSizeState>>({});

  const itemsRef = useLatestRef(items);
  const sizeStateRef = useLatestRef(sizeStateByAppId);
  const sizeQueueRef = useRef<string[]>([]);
  const sizeQueuedSetRef = useRef(new Set<string>());
  const sizeFlushingRef = useRef(false);

  const updateSizeState = useCallback((appIds: string[], state: AppSizeState) => {
    setSizeStateByAppId((previous) => {
      const next = { ...previous };
      for (const appId of appIds) {
        if ((next[appId] ?? "pending") !== "exact") {
          next[appId] = state;
        }
      }
      return next;
    });
  }, []);

  const flushSizeQueue = useCallback(async () => {
    if (sizeFlushingRef.current) {
      return;
    }

    sizeFlushingRef.current = true;
    try {
      while (sizeQueueRef.current.length > 0) {
        const visibleIds = new Set(itemsRef.current.map((item) => item.id));
        const batch = sizeQueueRef.current.splice(0, SIZE_BATCH);
        for (const appId of batch) {
          sizeQueuedSetRef.current.delete(appId);
        }

        const candidates = batch.filter((appId) => {
          if (!visibleIds.has(appId)) {
            return false;
          }
          return (sizeStateRef.current[appId] ?? "pending") !== "exact";
        });

        if (candidates.length === 0) {
          continue;
        }

        updateSizeState(candidates, "resolving");
        try {
          const resolved = await appManagerResolveSizes({ appIds: candidates });
          const resolvedById = new Map(resolved.items.map((item) => [item.appId, item]));

          setItems((previous) =>
            previous.map((item) => {
              const next = resolvedById.get(item.id);
              if (!next) {
                return item;
              }
              return {
                ...item,
                sizeBytes: next.sizeBytes,
                sizeAccuracy: next.sizeAccuracy,
                sizeSource: next.sizeSource,
                sizeComputedAt: next.sizeComputedAt,
              };
            }),
          );

          setSizeStateByAppId((previous) => {
            const next = { ...previous };
            for (const appId of candidates) {
              const resolvedItem = resolvedById.get(appId);
              if (!resolvedItem) {
                next[appId] = "pending";
                continue;
              }
              next[appId] = resolvedItem.sizeAccuracy === "exact" ? "exact" : "estimated";
            }
            return next;
          });
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          setListError(message);
          updateSizeState(candidates, "pending");
        }

        await delay(12);
      }
    } finally {
      sizeFlushingRef.current = false;
    }
  }, [itemsRef, setItems, setListError, sizeFlushingRef, sizeQueueRef, sizeQueuedSetRef, sizeStateRef, updateSizeState]);

  const enqueueSizeResolution = useCallback(
    (appIds: string[], priority = false) => {
      const queue = sizeQueueRef.current;
      for (const appId of appIds) {
        const state = sizeStateRef.current[appId] ?? "pending";
        if (state === "exact" || sizeQueuedSetRef.current.has(appId)) {
          continue;
        }
        sizeQueuedSetRef.current.add(appId);
        if (priority) {
          queue.unshift(appId);
        } else {
          queue.push(appId);
        }
      }

      void flushSizeQueue();
    },
    [flushSizeQueue, sizeQueueRef, sizeQueuedSetRef, sizeStateRef],
  );

  const mergeListItems = useCallback(
    (nextItems: ManagedApp[], replace: boolean): MergeListItemsResult => {
      const base = replace ? [] : itemsRef.current;
      const mergedItems = replace ? nextItems : uniqueAppendById(base, nextItems);
      const keepIds = new Set(mergedItems.map((item) => item.id));

      setItems(mergedItems);
      setSizeStateByAppId((previous) => {
        const next: Record<string, AppSizeState> = replace ? {} : retainById(previous, keepIds);
        for (const item of mergedItems) {
          if (!next[item.id]) {
            next[item.id] = initialSizeState(item);
          }
        }
        return next;
      });

      if (replace) {
        sizeQueueRef.current = [];
        sizeQueuedSetRef.current = new Set<string>();
      }

      return { mergedItems, keepIds };
    },
    [itemsRef, setItems, sizeQueueRef, sizeQueuedSetRef],
  );

  return {
    enqueueSizeResolution,
    mergeListItems,
  };
}

function uniqueAppendById(base: ManagedApp[], nextItems: ManagedApp[]): ManagedApp[] {
  const map = new Map<string, ManagedApp>();
  for (const item of base) {
    map.set(item.id, item);
  }
  for (const item of nextItems) {
    map.set(item.id, item);
  }
  return [...map.values()];
}

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { ClipboardItem } from "@/components/clipboard/types";
import { runRecoverable } from "@/services/recoverable";

interface UseClipboardDeleteUndoOptions {
  delayMs: number;
  commitDelete: (id: string) => Promise<void>;
  onDeleteError?: (message: string) => void;
}

interface UseClipboardDeleteUndoResult {
  pendingDeleteIds: string[];
  pendingDeleteSet: Set<string>;
  undoItem: ClipboardItem | null;
  scheduleDelete: (item: ClipboardItem) => void;
  handleUndoDelete: () => void;
  cancelAllScheduledDeletes: () => void;
}

export function useClipboardDeleteUndo(options: UseClipboardDeleteUndoOptions): UseClipboardDeleteUndoResult {
  const { delayMs, commitDelete, onDeleteError } = options;
  const [pendingDeleteIds, setPendingDeleteIds] = useState<string[]>([]);
  const [undoItem, setUndoItem] = useState<ClipboardItem | null>(null);
  const deleteTimersRef = useRef<Map<string, number>>(new Map());
  const pendingDeleteSet = useMemo(() => new Set(pendingDeleteIds), [pendingDeleteIds]);

  const cancelAllScheduledDeletes = useCallback(() => {
    const timers = deleteTimersRef.current;
    for (const timerId of timers.values()) {
      window.clearTimeout(timerId);
    }
    timers.clear();
    setPendingDeleteIds([]);
    setUndoItem(null);
  }, []);

  const scheduleDelete = useCallback(
    (item: ClipboardItem) => {
      if (pendingDeleteSet.has(item.id) || deleteTimersRef.current.has(item.id)) {
        return;
      }

      setPendingDeleteIds((prev) => [...prev, item.id]);
      setUndoItem(item);

      const timerId = window.setTimeout(() => {
        deleteTimersRef.current.delete(item.id);
        setPendingDeleteIds((prev) => prev.filter((value) => value !== item.id));
        setUndoItem((current) => (current?.id === item.id ? null : current));

        void runRecoverable(() => commitDelete(item.id), {
          scope: "clipboard-panel",
          action: "commit_delete",
          message: "delete item failed",
          metadata: { id: item.id },
        }).then((result) => {
          if (!result.ok) {
            onDeleteError?.(result.message);
          }
        });
      }, delayMs);

      deleteTimersRef.current.set(item.id, timerId);
    },
    [commitDelete, delayMs, onDeleteError, pendingDeleteSet],
  );

  const handleUndoDelete = useCallback(() => {
    if (!undoItem) {
      return;
    }

    const timerId = deleteTimersRef.current.get(undoItem.id);
    if (timerId !== undefined) {
      window.clearTimeout(timerId);
      deleteTimersRef.current.delete(undoItem.id);
    }

    setPendingDeleteIds((prev) => prev.filter((value) => value !== undoItem.id));
    setUndoItem(null);
  }, [undoItem]);

  useEffect(() => {
    return () => {
      cancelAllScheduledDeletes();
    };
  }, [cancelAllScheduledDeletes]);

  return {
    pendingDeleteIds,
    pendingDeleteSet,
    undoItem,
    scheduleDelete,
    handleUndoDelete,
    cancelAllScheduledDeletes,
  };
}

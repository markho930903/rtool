import { useEffect } from "react";

import type { ClipboardItem } from "@/components/clipboard/types";

function shouldIgnoreHotkey(event: KeyboardEvent): boolean {
  const target = event.target as HTMLElement | null;
  if (!target) {
    return false;
  }

  const tagName = target.tagName;
  if (target.isContentEditable || tagName === "TEXTAREA" || tagName === "SELECT") {
    return true;
  }

  if (tagName === "INPUT") {
    if (event.key === "ArrowUp" || event.key === "ArrowDown" || event.key === "Enter" || event.key === "Escape") {
      return false;
    }
    return true;
  }

  return false;
}

interface UseClipboardHotkeysOptions {
  enabled?: boolean;
  visibleItems: ClipboardItem[];
  selectedItem: ClipboardItem | null;
  onSelectItemId: (id: string) => void;
  onCopyBack: (item: ClipboardItem) => void;
  onPinToggle: (item: ClipboardItem) => void;
  onDelete: (id: string) => void;
}

export function useClipboardHotkeys(options: UseClipboardHotkeysOptions) {
  const {
    enabled = true,
    visibleItems,
    selectedItem,
    onSelectItemId,
    onCopyBack,
    onPinToggle,
    onDelete,
  } = options;

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (shouldIgnoreHotkey(event)) {
        return;
      }

      if (visibleItems.length === 0) {
        return;
      }

      const currentIndex = visibleItems.findIndex((entry) => entry.id === selectedItem?.id);

      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const step = event.key === "ArrowDown" ? 1 : -1;
        const count = visibleItems.length;
        const baseIndex = currentIndex >= 0 ? currentIndex : 0;
        const nextIndex = (baseIndex + step + count) % count;
        onSelectItemId(visibleItems[nextIndex].id);
        return;
      }

      if (!selectedItem) {
        return;
      }

      if (event.key === "Enter") {
        event.preventDefault();
        onCopyBack(selectedItem);
        return;
      }

      if (event.key === "p" || event.key === "P") {
        event.preventDefault();
        onPinToggle(selectedItem);
        return;
      }

      const canDeleteByBackspace = event.key === "Backspace" && (event.metaKey || event.ctrlKey);
      if (event.key === "Delete" || canDeleteByBackspace) {
        event.preventDefault();
        onDelete(selectedItem.id);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [enabled, onCopyBack, onDelete, onPinToggle, onSelectItemId, selectedItem, visibleItems]);
}

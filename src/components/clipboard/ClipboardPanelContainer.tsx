import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";

import type { ClipboardItem } from "@/components/clipboard/types";
import ClipboardPanelView from "@/components/clipboard/ClipboardPanelView";
import { useClipboardActionFeedback } from "@/hooks/clipboard/useClipboardActionFeedback";
import { useClipboardDeleteUndo } from "@/hooks/clipboard/useClipboardDeleteUndo";
import { useClipboardHotkeys } from "@/hooks/clipboard/useClipboardHotkeys";
import { useBootState } from "@/components/loading";
import { useClipboardStore } from "@/stores/clipboard.store";

export interface ClipboardPanelProps {
  className?: string;
  compactMode?: boolean;
  onCompactModeToggle?: () => void;
  alwaysOnTop?: boolean;
  onAlwaysOnTopToggle?: () => void;
  searchInputRef?: RefObject<HTMLInputElement | null>;
}

const DELETE_UNDO_DELAY = 5_000;

export default function ClipboardPanelContainer(props: ClipboardPanelProps) {
  const { t } = useTranslation(["clipboard", "common"]);
  const compactMode = props.compactMode ?? false;
  const alwaysOnTop = props.alwaysOnTop ?? false;

  const items = useClipboardStore((state) => state.items);
  const loading = useClipboardStore((state) => state.loading);
  const initializing = useClipboardStore((state) => state.initializing);
  const initialized = useClipboardStore((state) => state.initialized);
  const query = useClipboardStore((state) => state.query);
  const itemType = useClipboardStore((state) => state.itemType);
  const onlyPinned = useClipboardStore((state) => state.onlyPinned);
  const error = useClipboardStore((state) => state.error);
  const setQuery = useClipboardStore((state) => state.setQuery);
  const setItemType = useClipboardStore((state) => state.setItemType);
  const setOnlyPinned = useClipboardStore((state) => state.setOnlyPinned);
  const ensureInitialized = useClipboardStore((state) => state.ensureInitialized);
  const pinItem = useClipboardStore((state) => state.pinItem);
  const deleteItem = useClipboardStore((state) => state.deleteItem);
  const clearAllItems = useClipboardStore((state) => state.clearAllItems);
  const copyBack = useClipboardStore((state) => state.copyBack);
  const copyFilePathsBack = useClipboardStore((state) => state.copyFilePathsBack);
  const copyImageBack = useClipboardStore((state) => state.copyImageBack);

  const [previewItem, setPreviewItem] = useState<ClipboardItem | null>(null);
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const clipboardItemRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  const {
    previewMessage,
    setPreviewMessage,
    clearAllError,
    setClearAllError,
    isClearingAll,
    handleCopyBack,
    handleCopyFilePaths,
    handleCopyPreviewImage,
    handleClearAll,
  } = useClipboardActionFeedback({
    t,
    copyBack,
    copyFilePathsBack,
    copyImageBack,
    clearAllItems,
  });

  const { pendingDeleteSet, undoItem, scheduleDelete, handleUndoDelete, cancelAllScheduledDeletes } =
    useClipboardDeleteUndo({
      delayMs: DELETE_UNDO_DELAY,
      commitDelete: deleteItem,
      onDeleteError: setPreviewMessage,
    });

  const bootReady = initialized && !initializing;
  const { mounted: bootMounted, visible: bootVisible } = useBootState({
    cycleKey: 1,
    ready: bootReady,
    delayMs: 220,
    minVisibleMs: 180,
    maxWaitMs: 1500,
    exitMs: 160,
  });

  const visibleItems = useMemo(() => {
    const queryKeyword = query.trim().toLowerCase();
    return items.filter((entry) => {
      if (pendingDeleteSet.has(entry.id)) {
        return false;
      }
      if (onlyPinned && !entry.pinned) {
        return false;
      }
      if (itemType && entry.itemType !== itemType) {
        return false;
      }
      if (!queryKeyword) {
        return true;
      }
      return entry.plainText.toLowerCase().includes(queryKeyword);
    });
  }, [items, itemType, onlyPinned, pendingDeleteSet, query]);

  const selectedItem = useMemo(
    () => visibleItems.find((entry) => entry.id === selectedItemId) ?? visibleItems[0] ?? null,
    [visibleItems, selectedItemId],
  );

  useEffect(() => {
    void ensureInitialized();
  }, [ensureInitialized]);

  useEffect(() => {
    if (!selectedItem) {
      setSelectedItemId(null);
      return;
    }

    if (selectedItem.id !== selectedItemId) {
      setSelectedItemId(selectedItem.id);
    }
  }, [selectedItem, selectedItemId]);

  useEffect(() => {
    if (!selectedItem?.id) {
      return;
    }

    const selectedNode = clipboardItemRefs.current.get(selectedItem.id);
    selectedNode?.scrollIntoView({ block: "nearest" });
  }, [selectedItem?.id, visibleItems.length]);

  useEffect(() => {
    if (!showClearConfirm) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !isClearingAll) {
        setShowClearConfirm(false);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isClearingAll, showClearConfirm]);

  const handleDelete = useCallback(
    (id: string) => {
      const item = items.find((entry) => entry.id === id);
      if (!item) {
        return;
      }

      scheduleDelete(item);
    },
    [items, scheduleDelete],
  );

  const handlePinToggle = useCallback(
    (item: ClipboardItem) => {
      void pinItem(item.id, !item.pinned);
    },
    [pinItem],
  );

  const handleCopyItem = useCallback(
    (item: ClipboardItem) => {
      void handleCopyBack(item);
    },
    [handleCopyBack],
  );

  const handleCopyPaths = useCallback(
    (item: ClipboardItem) => {
      void handleCopyFilePaths(item);
    },
    [handleCopyFilePaths],
  );

  const handlePreviewItem = useCallback((item: ClipboardItem) => {
    setPreviewMessage(null);
    setPreviewItem(item);
  }, [setPreviewMessage]);

  useClipboardHotkeys({
    visibleItems,
    selectedItem,
    onSelectItemId: setSelectedItemId,
    onCopyBack: handleCopyItem,
    onPinToggle: handlePinToggle,
    onDelete: handleDelete,
  });

  const handleConfirmClearAll = useCallback(() => {
    void handleClearAll({
      before: () => {
        cancelAllScheduledDeletes();
        setPreviewItem(null);
        setPreviewMessage(null);
      },
      success: () => {
        setSelectedItemId(null);
        setShowClearConfirm(false);
      },
    });
  }, [cancelAllScheduledDeletes, handleClearAll, setPreviewMessage]);

  const handleOpenClearConfirm = useCallback(() => {
    setClearAllError(null);
    setShowClearConfirm(true);
  }, [setClearAllError]);

  const handleCloseClearConfirm = useCallback(() => {
    if (!isClearingAll) {
      setShowClearConfirm(false);
    }
  }, [isClearingAll]);

  const handleClosePreview = useCallback(() => {
    setPreviewItem(null);
    setPreviewMessage(null);
  }, [setPreviewMessage]);

  return (
    <ClipboardPanelView
      className={props.className}
      compactMode={compactMode}
      alwaysOnTop={alwaysOnTop}
      onCompactModeToggle={props.onCompactModeToggle}
      onAlwaysOnTopToggle={props.onAlwaysOnTopToggle}
      searchInputRef={props.searchInputRef}
      query={query}
      itemType={itemType}
      onlyPinned={onlyPinned}
      error={error}
      clearAllError={clearAllError}
      loading={loading}
      visibleItems={visibleItems}
      selectedItem={selectedItem}
      clipboardItemRefs={clipboardItemRefs}
      onSelectItemId={setSelectedItemId}
      onPinToggleItem={handlePinToggle}
      onCopyBackItem={handleCopyItem}
      onCopyPathsItem={handleCopyPaths}
      onDeleteItem={handleDelete}
      onPreviewItem={handlePreviewItem}
      onQueryChange={setQuery}
      onTypeChange={setItemType}
      onOnlyPinnedChange={setOnlyPinned}
      bootMounted={bootMounted}
      bootVisible={bootVisible}
      undoItem={undoItem}
      onUndoDelete={handleUndoDelete}
      showClearConfirm={showClearConfirm}
      isClearingAll={isClearingAll}
      onOpenClearConfirm={handleOpenClearConfirm}
      onCloseClearConfirm={handleCloseClearConfirm}
      onConfirmClearAll={handleConfirmClearAll}
      previewItem={previewItem}
      previewMessage={previewMessage}
      onClosePreview={handleClosePreview}
      onCopyPreviewImage={handleCopyPreviewImage}
    />
  );
}

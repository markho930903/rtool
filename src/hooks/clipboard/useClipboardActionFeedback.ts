import { useCallback, useState } from "react";

import type { ClipboardItem } from "@/components/clipboard/types";
import { runRecoverable } from "@/services/recoverable";

type Translate = (key: string, options?: Record<string, unknown>) => string;

interface UseClipboardActionFeedbackOptions {
  t: Translate;
  copyBack: (id: string) => Promise<void>;
  copyFilePathsBack: (id: string) => Promise<void>;
  copyImageBack: (id: string) => Promise<void>;
  clearAllItems: () => Promise<void>;
}

interface ClearAllCallbacks {
  before?: () => void;
  success?: () => void;
}

interface UseClipboardActionFeedbackResult {
  previewMessage: string | null;
  setPreviewMessage: (message: string | null) => void;
  clearAllError: string | null;
  setClearAllError: (message: string | null) => void;
  isClearingAll: boolean;
  handleCopyBack: (item: ClipboardItem) => Promise<void>;
  handleCopyFilePaths: (item: ClipboardItem) => Promise<void>;
  handleCopyPreviewImage: (id: string) => Promise<void>;
  handleClearAll: (callbacks?: ClearAllCallbacks) => Promise<void>;
}

export function useClipboardActionFeedback(
  options: UseClipboardActionFeedbackOptions,
): UseClipboardActionFeedbackResult {
  const { t, copyBack, copyFilePathsBack, copyImageBack, clearAllItems } = options;
  const [previewMessage, setPreviewMessage] = useState<string | null>(null);
  const [clearAllError, setClearAllError] = useState<string | null>(null);
  const [isClearingAll, setIsClearingAll] = useState(false);

  const handleCopyPreviewImage = useCallback(
    async (id: string) => {
      const result = await runRecoverable(
        () => copyImageBack(id),
        {
          scope: "clipboard-panel",
          action: "copy_preview_image",
          message: "copy image failed",
          metadata: { id },
        },
      );

      if (!result.ok) {
        setPreviewMessage(result.message);
        return;
      }

      setPreviewMessage(t("panel.copyMessageImage"));
    },
    [copyImageBack, t],
  );

  const handleCopyBack = useCallback(
    async (item: ClipboardItem) => {
      const result = await runRecoverable(
        async () => {
          if (item.itemType === "image") {
            await copyImageBack(item.id);
            return t("panel.copyMessageImage");
          }

          await copyBack(item.id);
          return item.itemType === "file" ? t("panel.copyMessageFile") : t("panel.copyMessageText");
        },
        {
          scope: "clipboard-panel",
          action: "copy_back",
          message: "copy failed",
          metadata: { id: item.id, itemType: item.itemType },
        },
      );

      if (!result.ok) {
        setPreviewMessage(result.message);
        return;
      }

      setPreviewMessage(result.data);
    },
    [copyBack, copyImageBack, t],
  );

  const handleCopyFilePaths = useCallback(
    async (item: ClipboardItem) => {
      const result = await runRecoverable(
        async () => {
          await copyFilePathsBack(item.id);
          return t("panel.copyMessageFilePath");
        },
        {
          scope: "clipboard-panel",
          action: "copy_file_paths",
          message: "copy file paths failed",
          metadata: { id: item.id, itemType: item.itemType },
        },
      );

      if (!result.ok) {
        setPreviewMessage(result.message);
        return;
      }

      setPreviewMessage(result.data);
    },
    [copyFilePathsBack, t],
  );

  const handleClearAll = useCallback(
    async (callbacks?: ClearAllCallbacks) => {
      if (isClearingAll) {
        return;
      }

      setIsClearingAll(true);
      setClearAllError(null);
      callbacks?.before?.();

      const result = await runRecoverable(
        () => clearAllItems(),
        {
          scope: "clipboard-panel",
          action: "clear_all",
          message: "clear all failed",
        },
      );

      if (!result.ok) {
        setClearAllError(t("panel.clearAllFailed", { message: result.message }));
        setIsClearingAll(false);
        return;
      }

      callbacks?.success?.();
      setIsClearingAll(false);
    },
    [clearAllItems, isClearingAll, t],
  );

  return {
    previewMessage,
    setPreviewMessage,
    clearAllError,
    setClearAllError,
    isClearingAll,
    handleCopyBack,
    handleCopyFilePaths,
    handleCopyPreviewImage,
    handleClearAll,
  };
}

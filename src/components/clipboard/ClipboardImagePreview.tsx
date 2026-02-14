import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { ClipboardItem } from "@/components/clipboard/types";
import { Button, Dialog } from "@/components/ui";

interface ClipboardImagePreviewProps {
  item: ClipboardItem | null;
  message: string | null;
  onClose: () => void;
  onCopyImage: (id: string) => Promise<void>;
}

function resolveImageUrlCandidates(item: ClipboardItem | null): string[] {
  if (!item) {
    return [];
  }

  const candidates: string[] = [];

  if (item.previewPath) {
    candidates.push(convertFileSrc(item.previewPath));
  }

  if (item.previewDataUrl) {
    candidates.push(item.previewDataUrl);
  }

  return [...new Set(candidates)];
}

export default function ClipboardImagePreview(props: ClipboardImagePreviewProps) {
  const { t } = useTranslation("clipboard");
  const { item, message, onClose, onCopyImage } = props;

  const imageUrlCandidates = useMemo(() => resolveImageUrlCandidates(item), [item]);
  const [imageUrlIndex, setImageUrlIndex] = useState(0);

  useEffect(() => {
    setImageUrlIndex(0);
  }, [item?.id, item?.previewDataUrl, item?.previewPath]);

  const imageUrl = imageUrlCandidates[imageUrlIndex] ?? null;

  if (!item || !imageUrl) {
    return null;
  }

  const itemId = item.id;
  const downloadName = `${itemId}.png`;

  return (
    <Dialog
      open
      onClose={onClose}
      zIndexClassName="z-[70] flex items-center justify-center"
      className="flex max-h-[88vh] w-[min(900px,92vw)] flex-col rounded-xl border border-border-muted bg-surface-overlay shadow-[var(--shadow-overlay)] backdrop-blur-[16px]"
      ariaLabel={t("preview.title")}
      closeOnBackdrop
      closeOnEscape
      canClose
    >
      <>
        <header className="flex items-center justify-between gap-2.5 border-b border-border-muted px-[14px] py-3">
          <h3 className="m-0 text-sm text-text-primary">{t("preview.title")}</h3>
          <Button size="xs" variant="secondary" onClick={onClose}>
            <span className="btn-icon i-noto:cross-mark" aria-hidden="true" />
            <span>{t("preview.close")}</span>
          </Button>
        </header>

        <div className="flex justify-center overflow-auto p-3">
          <img
            src={imageUrl}
            alt={item.plainText}
            className="max-h-[66vh] max-w-full rounded-[calc(var(--radius-md)+2px)] border border-border-muted bg-surface object-contain"
            onError={() => {
              setImageUrlIndex((index) => Math.min(index + 1, imageUrlCandidates.length));
            }}
          />
        </div>

        <footer className="flex items-center justify-end gap-2.5 border-t border-border-muted px-[14px] py-3">
          <Button size="xs" variant="secondary" onClick={() => void onCopyImage(itemId)}>
            <span className="btn-icon i-noto:clipboard" aria-hidden="true" />
            <span>{t("preview.copyImage")}</span>
          </Button>
          <Button as="a" href={imageUrl} download={downloadName} size="xs" variant="secondary">
            <span className="btn-icon i-noto:inbox-tray" aria-hidden="true" />
            <span>{t("preview.downloadImage")}</span>
          </Button>
        </footer>

        {message ? <div className="px-[14px] pb-3 text-xs text-info">{message}</div> : null}
      </>
    </Dialog>
  );
}

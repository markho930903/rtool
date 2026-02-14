import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import { itemTypeLabel } from "@/components/clipboard/clipboard-labels";
import type { ClipboardItem } from "@/components/clipboard/types";
import { Button } from "@/components/ui";

interface ClipboardItemCardProps {
  item: ClipboardItem;
  compact?: boolean;
  selected?: boolean;
  hideActions?: boolean;
  onSelect?: () => void;
  onPinToggle: () => void;
  onCopyBack: () => void;
  onCopyPaths: () => void;
  onDelete: () => void;
  onPreview: () => void;
}

function formatTime(timestamp: number, locale: string): string {
  const normalized = timestamp > 10_000_000_000 ? timestamp : timestamp * 1000;
  return new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(normalized));
}

function resolveImageUrlCandidates(item: ClipboardItem): string[] {
  const candidates: string[] = [];

  if (item.previewPath) {
    candidates.push(convertFileSrc(item.previewPath));
  }

  if (item.previewDataUrl) {
    candidates.push(item.previewDataUrl);
  }

  return [...new Set(candidates)];
}

function parseFilePaths(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((value) => value.trim())
    .filter(Boolean)
    .map((value) => (value.startsWith("file://") ? value.slice("file://".length) : value));
}

function fileBaseName(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const segments = normalized.split("/").filter(Boolean);
  return segments.length > 0 ? segments[segments.length - 1] : path;
}

function toCompactSummary(
  item: ClipboardItem,
  filePaths: string[],
  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  if (item.itemType === "file") {
    if (filePaths.length === 0) {
      return t("item.fileContent");
    }

    const names = filePaths.slice(0, 2).map(fileBaseName);
    if (filePaths.length > 2) {
      return t("item.fileSummaryMore", { names: names.join("、"), count: filePaths.length });
    }

    return names.join("、");
  }

  const plain = item.plainText.replace(/\s+/g, " ").trim();
  if (plain.length > 0) {
    return plain;
  }

  if (item.itemType === "image") {
    return t("item.imageContent");
  }

  return t("item.emptyContent");
}

export default function ClipboardItemCard(props: ClipboardItemCardProps) {
  const { t, i18n } = useTranslation(["clipboard", "common"]);
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const isCompact = props.compact ?? false;
  const isSelected = props.selected ?? false;
  const hideActions = props.hideActions ?? false;

  const preview = props.item.plainText.length > 800 ? `${props.item.plainText.slice(0, 800)}...` : props.item.plainText;
  const imageUrlCandidates = useMemo(() => resolveImageUrlCandidates(props.item), [props.item]);
  const filePaths = useMemo(() => parseFilePaths(props.item.plainText), [props.item.plainText]);
  const compactSummary = useMemo(() => toCompactSummary(props.item, filePaths, t), [filePaths, props.item, t]);
  const [imageUrlIndex, setImageUrlIndex] = useState(0);
  const isImage = props.item.itemType === "image";
  const isFile = props.item.itemType === "file";
  const typeLabel = itemTypeLabel(props.item.itemType, t);

  useEffect(() => {
    setImageUrlIndex(0);
  }, [props.item.id, props.item.previewDataUrl, props.item.previewPath]);

  const imageUrl = imageUrlCandidates[imageUrlIndex] ?? null;

  const handleSelect = () => {
    props.onSelect?.();
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (!props.onSelect) {
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      props.onSelect();
    }
  };

  if (isCompact) {
    const rowClassName = [
      "rounded-md border border-transparent px-2.5 py-2.25 text-left transition-colors duration-[140ms] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
      isSelected ? "border-accent bg-accent-soft" : "hover:bg-surface-soft",
      props.onSelect
        ? "cursor-pointer"
        : "",
    ]
      .filter(Boolean)
      .join(" ");

    return (
      <article
        className={rowClassName}
        onClick={handleSelect}
        onKeyDown={handleKeyDown}
        role={props.onSelect ? "button" : undefined}
        tabIndex={props.onSelect ? 0 : undefined}
      >
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-1.5">
            <span className="text-[10px] text-info uppercase tracking-wide">{typeLabel}</span>
            <span
              className="max-w-[130px] truncate text-[10px] text-text-muted"
              title={props.item.sourceApp ?? t("common:status.empty")}
            >
              {props.item.sourceApp ?? t("common:status.empty")}
            </span>
            {props.item.pinned ? (
              <span className="rounded-full border border-border-muted bg-surface px-1.5 py-0.5 text-[10px] text-text-secondary">
                {t("item.pinnedShort")}
              </span>
            ) : null}
          </div>
          <span className="text-[11px] text-text-muted">{formatTime(props.item.createdAt, locale)}</span>
        </div>
        <p className="mt-1 text-[12px] leading-[1.35] text-text-secondary [display:-webkit-box] [-webkit-line-clamp:2] [-webkit-box-orient:vertical] overflow-hidden">
          {compactSummary}
        </p>
      </article>
    );
  }

  return (
    <article className="rounded-md border border-border-muted bg-surface p-3">
      <header className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5">
          <span className="rounded-full border border-border-muted bg-surface px-2 py-0.5 text-[11px] text-text-secondary">
            {typeLabel}
          </span>
          <span
            className="max-w-[180px] truncate rounded-full border border-border-muted bg-surface px-2 py-0.5 text-[11px] text-text-secondary"
            title={props.item.sourceApp ?? t("common:status.empty")}
          >
            {t("item.sourcePrefix")}
            {props.item.sourceApp ?? t("common:status.empty")}
          </span>
          {props.item.pinned ? (
            <span className="rounded-full border border-border-muted bg-surface px-2 py-0.5 text-[11px] text-text-secondary">
              {t("item.pinned")}
            </span>
          ) : null}
        </div>
        <span className="text-[11px] text-text-muted">{formatTime(props.item.createdAt, locale)}</span>
      </header>

      <div className="mt-2 space-y-2">
        {isImage ? (
          imageUrl ? (
            <img
              src={imageUrl}
              alt={props.item.plainText}
              className="max-h-[220px] w-full cursor-zoom-in rounded-md border border-border-muted bg-surface object-contain"
              loading="lazy"
              onClick={props.onPreview}
              onError={() => {
                setImageUrlIndex((index) => {
                  const nextIndex = index + 1;
                  const exhausted = nextIndex >= imageUrlCandidates.length;
                  if (exhausted) {
                    console.warn("[clipboard] image preview unavailable", {
                      itemId: props.item.id,
                      candidateCount: imageUrlCandidates.length,
                    });
                    return imageUrlCandidates.length;
                  }
                  return nextIndex;
                });
              }}
            />
          ) : (
            <div className="rounded-md border border-dashed border-border-muted bg-surface px-3 py-6 text-center text-xs text-text-muted">
              {t("item.imageUnavailable")}
            </div>
          )
        ) : null}

        {isFile ? (
          <div className="rounded-md border border-border-muted bg-surface p-2">
            <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
              {filePaths.slice(0, 6).map((path) => (
                <li key={path} className="flex flex-col gap-0.5" title={path}>
                  <span className="text-xs font-semibold text-text-primary">{fileBaseName(path)}</span>
                  <span className="truncate whitespace-nowrap text-[11px] text-text-muted">{path}</span>
                </li>
              ))}
            </ul>
            {filePaths.length > 6 ? (
              <div className="mt-2 text-[11px] text-text-muted">
                {t("item.moreFiles", { count: filePaths.length - 6 })}
              </div>
            ) : null}
          </div>
        ) : (
          <pre className="m-0 max-h-[300px] overflow-auto whitespace-pre-wrap break-words rounded-md border border-border-muted bg-surface p-2 text-xs text-text-secondary">
            {preview || t("item.emptyContent")}
          </pre>
        )}
      </div>

      {!hideActions ? (
        <footer className="mt-3 flex flex-wrap gap-2">
          <Button
            variant="secondary"
            size="xs"
            onClick={(event) => {
              event.stopPropagation();
              props.onPinToggle();
            }}
          >
            <span className="btn-icon i-noto:pushpin" aria-hidden="true" />
            <span>{props.item.pinned ? t("action.unpin") : t("action.pin")}</span>
          </Button>

          <Button
            variant="secondary"
            size="xs"
            onClick={(event) => {
              event.stopPropagation();
              props.onCopyBack();
            }}
          >
            <span className="btn-icon i-noto:clipboard" aria-hidden="true" />
            <span>{isImage ? t("action.copyImage") : isFile ? t("action.copyFile") : t("action.copyBack")}</span>
          </Button>

          {isFile ? (
            <Button
              variant="secondary"
              size="xs"
              onClick={(event) => {
                event.stopPropagation();
                props.onCopyPaths();
              }}
            >
              <span className="btn-icon i-noto:link" aria-hidden="true" />
              <span>{t("action.copyPath")}</span>
            </Button>
          ) : null}

          {isImage ? (
            <Button
              variant="secondary"
              size="xs"
              onClick={(event) => {
                event.stopPropagation();
                props.onPreview();
              }}
            >
              <span className="btn-icon i-noto:eye" aria-hidden="true" />
              <span>{t("action.preview")}</span>
            </Button>
          ) : null}

          <Button
            variant="danger"
            size="xs"
            onClick={(event) => {
              event.stopPropagation();
              props.onDelete();
            }}
          >
            <span className="btn-icon i-noto:wastebasket" aria-hidden="true" />
            <span>{t("action.delete")}</span>
          </Button>
        </footer>
      ) : null}
    </article>
  );
}

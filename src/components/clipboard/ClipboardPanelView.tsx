import type { MutableRefObject, RefObject } from "react";
import { useTranslation } from "react-i18next";

import { itemTypeLabel, undoLabel } from "@/components/clipboard/clipboard-labels";
import ClipboardFilterBar from "@/components/clipboard/ClipboardFilterBar";
import ClipboardImagePreview from "@/components/clipboard/ClipboardImagePreview";
import ClipboardItemCard from "@/components/clipboard/ClipboardItemCard";
import type { ClipboardItem } from "@/components/clipboard/types";
import { BootOverlay } from "@/components/loading";
import { Button, Dialog } from "@/components/ui";
import type { ClipboardActionFeedback } from "@/hooks/clipboard/useClipboardActionFeedback";

interface ClipboardPanelViewProps {
  className?: string;
  compactMode: boolean;
  alwaysOnTop: boolean;
  onCompactModeToggle?: () => void;
  onAlwaysOnTopToggle?: () => void;
  searchInputRef?: RefObject<HTMLInputElement | null>;
  query: string;
  itemType: string;
  onlyPinned: boolean;
  error: string | null;
  actionFeedback: ClipboardActionFeedback | null;
  clearAllError: string | null;
  loading: boolean;
  visibleItems: ClipboardItem[];
  selectedItem: ClipboardItem | null;
  clipboardItemRefs: MutableRefObject<Map<string, HTMLDivElement>>;
  onSelectItemId: (id: string) => void;
  onPinToggleItem: (item: ClipboardItem) => void;
  onCopyBackItem: (item: ClipboardItem) => void;
  onCopyPathsItem: (item: ClipboardItem) => void;
  onDeleteItem: (id: string) => void;
  onPreviewItem: (item: ClipboardItem) => void;
  onQueryChange: (query: string) => void;
  onTypeChange: (itemType: string) => void;
  onOnlyPinnedChange: (onlyPinned: boolean) => void;
  bootMounted: boolean;
  bootVisible: boolean;
  undoItem: ClipboardItem | null;
  onUndoDelete: () => void;
  showClearConfirm: boolean;
  isClearingAll: boolean;
  onOpenClearConfirm: () => void;
  onCloseClearConfirm: () => void;
  onConfirmClearAll: () => void;
  previewItem: ClipboardItem | null;
  onClosePreview: () => void;
  onCopyPreviewImage: (id: string) => Promise<void>;
}

export default function ClipboardPanelView(props: ClipboardPanelViewProps) {
  const { t } = useTranslation(["clipboard", "common"]);

  const renderDetail = () => {
    const selectedItem = props.selectedItem;
    if (!selectedItem) {
      return <div className="mt-2 text-[12px] text-text-muted">{t("panel.empty")}</div>;
    }

    return (
      <ClipboardItemCard
        item={selectedItem}
        onPinToggle={() => props.onPinToggleItem(selectedItem)}
        onCopyBack={() => props.onCopyBackItem(selectedItem)}
        onCopyPaths={() => props.onCopyPathsItem(selectedItem)}
        onDelete={() => props.onDeleteItem(selectedItem.id)}
        onPreview={() => props.onPreviewItem(selectedItem)}
      />
    );
  };

  const compactToggleLabel = props.compactMode ? t("action.exitCompactMode") : t("action.enterCompactMode");
  const alwaysOnTopLabel = props.alwaysOnTop ? t("action.unpinWindow") : t("action.pinWindow");

  return (
    <section
      className={["relative flex h-full flex-col rounded-2xl bg-surface-soft p-3", props.className ?? ""]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="flex flex-wrap items-center gap-2">
        <p className="m-0 min-w-0 flex-1 text-[11px] text-text-muted">
          {props.compactMode
            ? t("panel.summaryCompact", { count: props.visibleItems.length })
            : t("panel.summary", { count: props.visibleItems.length })}
        </p>
        <div className="ml-auto flex flex-wrap items-center justify-end gap-2">
          <Button
            size="xs"
            variant={props.alwaysOnTop ? "secondary" : "ghost"}
            iconOnly
            title={alwaysOnTopLabel}
            aria-label={alwaysOnTopLabel}
            disabled={!props.onAlwaysOnTopToggle}
            onClick={props.onAlwaysOnTopToggle}
          >
            <span
              className={[
                "inline-block leading-none text-[2.2rem] transform-gpu scale-[1.25] origin-center",
                props.alwaysOnTop ? "i-noto:pushpin" : "i-noto:round-pushpin",
              ].join(" ")}
              aria-hidden="true"
            />
          </Button>
          <Button
            size="xs"
            variant="ghost"
            iconOnly
            title={compactToggleLabel}
            aria-label={compactToggleLabel}
            onClick={props.onCompactModeToggle}
          >
            <span
              className={[
                "inline-block leading-none text-[2.2rem] transform-gpu scale-[1.25] origin-center",
                props.compactMode ? "i-noto:left-right-arrow" : "i-noto:up-down-arrow",
              ].join(" ")}
              aria-hidden="true"
            />
          </Button>
          <Button
            size="xs"
            variant="danger"
            disabled={props.visibleItems.length === 0 || props.isClearingAll}
            onClick={props.onOpenClearConfirm}
          >
            <span className="btn-icon i-noto:wastebasket" aria-hidden="true" />
            <span>{t("action.clearAll")}</span>
          </Button>
        </div>
      </div>

      <ClipboardFilterBar
        searchInputRef={props.searchInputRef}
        compact={props.compactMode}
        query={props.query}
        itemType={props.itemType}
        onlyPinned={props.onlyPinned}
        onQueryChange={props.onQueryChange}
        onTypeChange={props.onTypeChange}
        onOnlyPinnedChange={props.onOnlyPinnedChange}
      />

      {props.error ? <div className="mt-2 text-[12px] text-text-muted text-danger">{props.error}</div> : null}
      {props.actionFeedback ? (
        <div
          className={[
            "mt-2 text-[12px]",
            props.actionFeedback.kind === "error" ? "text-danger" : "text-info",
          ].join(" ")}
        >
          {props.actionFeedback.message}
        </div>
      ) : null}
      {props.clearAllError ? <div className="mt-2 text-[12px] text-text-muted text-danger">{props.clearAllError}</div> : null}

      <div
        className={
          props.compactMode
            ? "mt-3 min-h-0 flex flex-1 flex-col"
            : "mt-3 grid min-h-0 flex-1 grid-cols-[330px_1fr] gap-3"
        }
      >
        <section
          className="min-h-0 h-full flex flex-col overflow-hidden rounded-md border border-border-muted bg-surface-soft"
          aria-label={t("panel.listAria")}
        >
          <div className="min-h-0 flex-1 space-y-1.5 overflow-auto p-2">
            {props.loading && props.visibleItems.length === 0 ? (
              <div className="space-y-1.5" aria-hidden="true">
                {[0, 1, 2, 3, 4, 5].map((index) => (
                  <div
                    key={`clipboard-skeleton-${index}`}
                    className="relative overflow-hidden rounded-md border border-border-muted/65 bg-surface px-2.5 py-2.5"
                  >
                    <div className="h-3 w-[64%] rounded bg-border-muted/70" />
                    <div className="mt-2 h-2.5 w-[82%] rounded bg-border-muted/55" />
                    <span
                      className="rtool-boot-shimmer-layer absolute inset-y-0 bg-gradient-to-r from-transparent via-shimmer-highlight/26 to-transparent"
                      style={{
                        left: "-45%",
                        width: "45%",
                        animation: "rtool-boot-shimmer 1.2s linear infinite",
                        animationDelay: `${index * 80}ms`,
                      }}
                    />
                  </div>
                ))}
              </div>
            ) : null}
            {!props.loading && props.visibleItems.length === 0 ? (
              <div className="mt-2 text-[12px] text-text-muted">{t("panel.empty")}</div>
            ) : null}
            {props.visibleItems.map((item) => (
              <div
                key={item.id}
                ref={(node) => {
                  if (node) {
                    props.clipboardItemRefs.current.set(item.id, node);
                    return;
                  }
                  props.clipboardItemRefs.current.delete(item.id);
                }}
              >
                <ClipboardItemCard
                  item={item}
                  compact
                  hideActions
                  selected={props.selectedItem?.id === item.id}
                  onSelect={() => props.onSelectItemId(item.id)}
                  onPinToggle={() => props.onPinToggleItem(item)}
                  onCopyBack={() => props.onCopyBackItem(item)}
                  onCopyPaths={() => props.onCopyPathsItem(item)}
                  onDelete={() => props.onDeleteItem(item.id)}
                  onPreview={() => props.onPreviewItem(item)}
                />
              </div>
            ))}
          </div>
        </section>

        {!props.compactMode ? (
          <section
            className="min-h-0 flex flex-col overflow-hidden rounded-md border border-border-muted bg-surface-soft"
            aria-label={t("panel.detailAria")}
          >
            <header className="flex items-start justify-between gap-3 border-b border-border-muted px-3 py-2.5">
              <div>
                <h3 className="m-0 text-sm font-semibold text-text-primary">
                  {props.selectedItem ? t("panel.detailTitle") : t("panel.noItem")}
                </h3>
                <p className="mt-1 text-xs text-text-muted">{t("panel.detailHint")}</p>
              </div>
              {props.selectedItem ? (
                <div className="flex flex-wrap items-center gap-1.5">
                  <span className="rounded-full border border-border-muted bg-surface px-2 py-0.5 text-[11px] text-text-secondary">
                    {itemTypeLabel(props.selectedItem.itemType, t)}
                  </span>
                  <span
                    className="max-w-[220px] truncate rounded-full border border-border-muted bg-surface px-2 py-0.5 text-[11px] text-text-secondary"
                    title={props.selectedItem.sourceApp ?? t("common:status.empty")}
                  >
                    {t("item.sourcePrefix")}
                    {props.selectedItem.sourceApp ?? t("common:status.empty")}
                  </span>
                  {props.selectedItem.pinned ? (
                    <span className="rounded-full border border-border-muted bg-surface px-2 py-0.5 text-[11px] text-text-secondary">
                      {t("item.pinned")}
                    </span>
                  ) : null}
                </div>
              ) : null}
            </header>

            <div className="min-h-0 flex-1 overflow-auto p-3">{renderDetail()}</div>
          </section>
        ) : null}
      </div>
      {props.bootMounted ? <BootOverlay variant="clipboard" visible={props.bootVisible} /> : null}

      {props.undoItem ? (
        <div
          className="mt-2 flex items-center justify-between gap-2 rounded-lg border border-border-strong bg-surface px-3 py-2 text-xs text-text-secondary"
          role="status"
          aria-live="polite"
        >
          <span>{t("panel.undoDeleted", { label: undoLabel(props.undoItem, t) })}</span>
          <Button size="xs" variant="secondary" onClick={props.onUndoDelete}>
            {t("panel.undo")}
          </Button>
        </div>
      ) : null}

      <Dialog
        open={props.showClearConfirm}
        onClose={props.onCloseClearConfirm}
        zIndexClassName="z-[72] flex items-center justify-center"
        className="w-[min(460px,92vw)] rounded-xl border border-border-muted bg-surface-overlay p-4 shadow-overlay backdrop-blur-[16px]"
        ariaLabel={t("confirm.clearAllTitle")}
        closeOnBackdrop
        closeOnEscape
        canClose={!props.isClearingAll}
      >
        <h3 className="m-0 text-sm font-semibold text-text-primary">{t("confirm.clearAllTitle")}</h3>
        <p className="mt-2 text-xs text-text-secondary">{t("confirm.clearAllDesc")}</p>
        <div className="mt-4 flex items-center justify-end gap-2">
          <Button size="xs" variant="secondary" disabled={props.isClearingAll} onClick={props.onCloseClearConfirm}>
            {t("confirm.cancel")}
          </Button>
          <Button size="xs" variant="danger" disabled={props.isClearingAll} onClick={props.onConfirmClearAll}>
            {props.isClearingAll ? t("common:status.loading") : t("confirm.confirm")}
          </Button>
        </div>
      </Dialog>

      <ClipboardImagePreview
        item={props.previewItem}
        onClose={props.onClosePreview}
        onCopyImage={props.onCopyPreviewImage}
      />
    </section>
  );
}

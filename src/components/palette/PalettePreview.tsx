import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { getFileExtension, resolveFileIconByExtension } from "@/components/icons/pathIcon";
import type { PaletteItem } from "@/components/palette/types";

interface PalettePreviewProps {
  selectedItem: PaletteItem | null;
  context?: "launcher" | "palette";
}

type ItemKind = "file" | "application" | "builtin" | "action" | "other";

const APPLICATION_FALLBACK_ICON = "i-noto:desktop-computer";
const BUILTIN_FALLBACK_ICON = "i-noto:hammer-and-wrench";
const ACTION_OR_OTHER_FALLBACK_ICON = "i-noto:card-index-dividers";

function categoryKey(category: string): string {
  if (category === "builtin") {
    return "category.builtin";
  }

  if (category === "application") {
    return "category.application";
  }

  if (category === "file") {
    return "category.file";
  }

  if (category === "action") {
    return "category.action";
  }

  return "category.other";
}

function actionKey(kind: string | undefined): string {
  if (kind === "open_builtin_route") {
    return "action.openBuiltinRoute";
  }

  if (kind === "open_builtin_tool") {
    return "action.openBuiltinTool";
  }

  if (kind === "open_builtin_window") {
    return "action.openBuiltinWindow";
  }

  if (kind === "open_file") {
    return "action.openFile";
  }

  if (kind === "open_application") {
    return "action.openApplication";
  }

  return "action.execute";
}

function inferItemKind(item: PaletteItem): ItemKind {
  const actionKind = item.action?.kind;
  if (actionKind === "open_file") {
    return "file";
  }

  if (actionKind === "open_application") {
    return "application";
  }

  if (
    actionKind === "open_builtin_route" ||
    actionKind === "open_builtin_tool" ||
    actionKind === "open_builtin_window"
  ) {
    return "builtin";
  }

  if (item.category === "file") {
    return "file";
  }

  if (item.category === "application") {
    return "application";
  }

  if (item.category === "builtin") {
    return "builtin";
  }

  if (item.category === "action") {
    return "action";
  }

  return "other";
}

function resolveLauncherFallbackIcon(item: PaletteItem): string {
  const kind = inferItemKind(item);
  if (kind === "file") {
    const path = item.action?.kind === "open_file" ? item.action.path : undefined;
    const ext = getFileExtension(path);
    return resolveFileIconByExtension(ext);
  }

  if (kind === "application") {
    return APPLICATION_FALLBACK_ICON;
  }

  if (kind === "builtin") {
    return BUILTIN_FALLBACK_ICON;
  }

  return ACTION_OR_OTHER_FALLBACK_ICON;
}

function isValidIconifyIcon(iconValue: string | undefined): boolean {
  if (!iconValue) {
    return false;
  }

  const value = iconValue.trim();
  if (!value) {
    return false;
  }

  return value.startsWith("i-") || value.includes(":");
}

export default function PalettePreview(props: PalettePreviewProps) {
  const { t } = useTranslation("palette");
  const [imageLoadFailed, setImageLoadFailed] = useState(false);
  const previewContext = props.context ?? "palette";

  useEffect(() => {
    setImageLoadFailed(false);
  }, [props.selectedItem?.id, props.selectedItem?.iconKind, props.selectedItem?.iconValue]);

  if (!props.selectedItem) {
    return <div className="p-4 text-text-secondary">{t("preview.empty")}</div>;
  }

  const item = props.selectedItem;
  const categoryLabel = t(categoryKey(item.category));
  const sourceLabel = item.source ?? categoryLabel;
  const actionLabel = t(actionKey(item.action?.kind));

  const icon = (() => {
    if (previewContext !== "launcher") {
      if (item.iconKind === "raster" && item.iconValue) {
        return (
          <img
            src={item.iconValue}
            alt=""
            className="h-8 w-8 rounded-md object-cover"
            loading="lazy"
            decoding="async"
          />
        );
      }

      return (
        <span
          className={`btn-icon h-8 w-8 text-[1.4rem] text-text-muted ${item.iconValue || ACTION_OR_OTHER_FALLBACK_ICON}`}
          aria-hidden="true"
        />
      );
    }

    if (item.iconKind === "raster" && item.iconValue && !imageLoadFailed) {
      return (
        <img
          src={item.iconValue}
          alt=""
          className="h-8 w-8 rounded-md object-cover"
          loading="lazy"
          decoding="async"
          onError={() => setImageLoadFailed(true)}
        />
      );
    }

    if (item.iconKind === "iconify" && isValidIconifyIcon(item.iconValue)) {
      return <span className={`btn-icon h-8 w-8 text-[1.4rem] text-text-muted ${item.iconValue}`} aria-hidden="true" />;
    }

    const fallbackIcon = resolveLauncherFallbackIcon(item);
    return <span className={`btn-icon h-8 w-8 text-[1.4rem] text-text-muted ${fallbackIcon}`} aria-hidden="true" />;
  })();

  return (
    <div className="flex h-full flex-col bg-gradient-to-b from-surface-soft/40 to-transparent p-4 text-text-secondary">
      <div className="text-[11px] uppercase text-text-muted">{t("preview.title")}</div>
      <div className="mt-2 flex items-center gap-2.5">
        <span className="inline-flex h-9 w-9 items-center justify-center">{icon}</span>
        <h3 className="text-base font-semibold text-text-primary text-pretty">{item.title}</h3>
      </div>
      <p className="mt-2 break-words text-[13px] text-text-secondary">{item.subtitle}</p>

      <div className="mt-3 flex flex-wrap gap-2 text-xs">
        <span className="rounded-full bg-surface-soft px-2 py-1 text-text-secondary">
          {t("preview.category", { value: categoryLabel })}
        </span>
        <span className="rounded-full bg-surface-soft px-2 py-1 text-text-secondary">
          {t("preview.source", { value: sourceLabel })}
        </span>
        {item.shortcut ? (
          <span className="rounded-full bg-accent-soft px-2 py-1 text-accent">
            {t("preview.shortcut", { value: item.shortcut })}
          </span>
        ) : null}
      </div>

      <div className="mt-3 text-xs text-text-muted">{t("preview.action", { value: actionLabel })}</div>

      <div className="mt-auto rounded-md border border-border-muted bg-surface-soft/60 px-3 py-2 text-[11px] text-text-muted">
        <div className="font-medium text-text-secondary">{t("preview.quickTitle")}</div>
        <div className="mt-1">{t("preview.quickHint1")}</div>
        <div className="mt-0.5">{t("preview.quickHint2")}</div>
      </div>
    </div>
  );
}

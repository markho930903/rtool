import type { ClipboardItem } from "@/components/clipboard/types";

type Translate = (key: string, options?: Record<string, unknown>) => string;

export function itemTypeLabel(itemType: string, t: Translate): string {
  if (itemType === "text") {
    return t("filter.type.text");
  }
  if (itemType === "link") {
    return t("filter.type.link");
  }
  if (itemType === "image") {
    return t("filter.type.image");
  }
  if (itemType === "file") {
    return t("filter.type.file");
  }
  if (itemType === "code") {
    return t("filter.type.code");
  }
  if (itemType === "color") {
    return t("filter.type.color");
  }
  return itemType;
}

export function undoLabel(item: ClipboardItem, t: Translate): string {
  if (item.itemType === "image") {
    return t("panel.undo.image");
  }

  if (item.itemType === "file") {
    return t("panel.undo.file");
  }

  return t("panel.undo.text");
}

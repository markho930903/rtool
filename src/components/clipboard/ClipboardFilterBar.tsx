import type { RefObject } from "react";
import { useTranslation } from "react-i18next";

import { Checkbox, Input, Select, type SelectOptionInput } from "@/components/ui";

interface ClipboardFilterBarProps {
  query: string;
  itemType: string;
  onlyPinned: boolean;
  compact?: boolean;
  searchInputRef?: RefObject<HTMLInputElement | null>;
  onQueryChange: (query: string) => void;
  onTypeChange: (itemType: string) => void;
  onOnlyPinnedChange: (onlyPinned: boolean) => void;
}

export default function ClipboardFilterBar(props: ClipboardFilterBarProps) {
  const { t } = useTranslation("clipboard");
  const compact = props.compact ?? false;
  const clipboardTypeOptions: SelectOptionInput[] = [
    { value: "", label: t("filter.type.all"), icon: "i-noto:card-index-dividers" },
    { value: "text", label: t("filter.type.text"), icon: "i-noto:memo" },
    { value: "link", label: t("filter.type.link"), icon: "i-noto:link" },
    { value: "image", label: t("filter.type.image"), icon: "i-noto:framed-picture" },
    { value: "file", label: t("filter.type.file"), icon: "i-noto:file-folder" },
    { value: "code", label: t("filter.type.code"), icon: "i-noto:desktop-computer" },
    { value: "color", label: t("filter.type.color"), icon: "i-noto:artist-palette" },
  ];

  return (
    <div className={compact ? "mt-2 flex flex-col gap-2" : "mt-2 flex flex-wrap items-center gap-2"}>
      <Input
        ref={props.searchInputRef}
        variant="clipboard"
        className={compact ? "w-full min-w-0" : "min-w-[220px] flex-1"}
        value={props.query}
        onChange={(event) => props.onQueryChange(event.currentTarget.value)}
        placeholder={t("filter.searchPlaceholder")}
      />

      <div className={compact ? "flex flex-wrap items-center gap-2" : "flex items-center gap-2"}>
        <Select
          variant="clipboard"
          className={compact ? "min-w-[150px]" : undefined}
          value={props.itemType}
          options={clipboardTypeOptions}
          onChange={(event) => props.onTypeChange(event.currentTarget.value)}
        />

        <Checkbox
          checked={props.onlyPinned}
          onChange={(event) => props.onOnlyPinnedChange(event.currentTarget.checked)}
          size="default"
          wrapperClassName="inline-flex gap-1.5 rounded-md border border-border-muted bg-surface px-2 py-1.5 text-[12px] text-text-secondary"
        >
          {t("filter.onlyPinned")}
        </Checkbox>
      </div>
    </div>
  );
}

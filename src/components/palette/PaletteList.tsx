import { useTranslation } from "react-i18next";

import type { PaletteItem } from "@/components/palette/types";

interface PaletteListProps {
  items: PaletteItem[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onActivate: (index: number) => void;
}

function categoryLabel(category: string, t: (key: string, options?: Record<string, unknown>) => string): string {
  if (category === "builtin") {
    return t("category.builtin");
  }

  if (category === "application") {
    return t("category.application");
  }

  if (category === "file") {
    return t("category.file");
  }

  if (category === "action") {
    return t("category.action");
  }

  return t("category.other");
}

export default function PaletteList(props: PaletteListProps) {
  const { t } = useTranslation("palette");

  if (props.items.length === 0) {
    return <div className="p-4 text-[13px] text-text-muted">{t("list.empty")}</div>;
  }

  return (
    <ul
      className="m-0 flex max-h-[440px] list-none flex-col gap-[6px] overflow-y-auto border-r border-border-muted p-2"
      role="listbox"
      aria-label={t("list.aria")}
    >
      {props.items.map((item, index) => {
        const isSelected = props.selectedIndex === index;
        return (
          <li
            key={item.id}
            className={
              isSelected
                ? "cursor-pointer rounded-md border border-transparent p-2.5 transition-colors duration-[140ms] border-accent bg-accent-soft"
                : "cursor-pointer rounded-md border border-transparent p-2.5 transition-colors duration-[140ms]"
            }
            onMouseEnter={() => props.onSelect(index)}
            onClick={() => props.onActivate(index)}
            role="option"
            aria-selected={isSelected}
          >
            <div className="text-sm font-semibold text-text-primary">{item.title}</div>
            <div className="mt-[3px] text-xs text-text-muted">{item.subtitle}</div>
            <div className="mt-[6px] text-[11px] text-text-muted uppercase">{categoryLabel(item.category, t)}</div>
          </li>
        );
      })}
    </ul>
  );
}

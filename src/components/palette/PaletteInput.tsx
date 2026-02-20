import { LoadingIndicator } from "@/components/loading";
import { Input } from "@/components/ui";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface PaletteInputProps {
  query: string;
  loading: boolean;
  onQueryChange: (query: string) => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
  trailingActions?: ReactNode;
}

export default function PaletteInput(props: PaletteInputProps) {
  const { t } = useTranslation("palette");

  return (
    <div className="border-b border-border-muted px-3.5 py-[10px]">
      <div className="flex items-center gap-2">
        <label
          className="flex min-w-0 flex-1 items-center gap-2.5 rounded-md px-2 py-[2px] transition-colors focus-within:bg-surface-soft"
          htmlFor="launcher-query-input"
        >
          <span className="text-[13px] text-text-secondary" aria-hidden="true">
            ⌘
          </span>
          <Input
            id="launcher-query-input"
            variant="palette"
            ref={props.inputRef}
            name="launcherQuery"
            autoComplete="off"
            spellCheck={false}
            aria-label={t("input.aria")}
            value={props.query}
            onChange={(event) => props.onQueryChange(event.currentTarget.value)}
            placeholder={t("input.placeholder")}
          />
          {props.loading ? <LoadingIndicator text={t("input.searching")} /> : null}
        </label>
        {props.trailingActions ? <div className="shrink-0">{props.trailingActions}</div> : null}
      </div>
      <p className="mt-1 pl-7 text-[11px] text-text-muted">{t("input.hint")}</p>
    </div>
  );
}

import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui";
import type { TransferSession } from "@/components/transfer/types";

interface TransferHistoryPanelProps {
  history: TransferSession[];
  onRefresh: () => Promise<void>;
  onClear: () => Promise<void>;
}

function formatTime(value: number): string {
  if (!value) {
    return "-";
  }
  return new Date(value).toLocaleString();
}

export default function TransferHistoryPanel(props: TransferHistoryPanelProps) {
  const { t } = useTranslation("transfer");

  return (
    <section className="rounded-4 border border-border-muted bg-surface p-4">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-text-primary">{t("history.title")}</h2>
        <div className="flex items-center gap-1">
          <Button
            type="button"
            size="xs"
            variant="secondary"
            className="ui-text-micro"
            onClick={() => {
              void props.onRefresh();
            }}
          >
            {t("history.refresh")}
          </Button>
          <Button
            type="button"
            size="xs"
            variant="secondary"
            className="ui-text-micro"
            onClick={() => {
              void props.onClear();
            }}
          >
            {t("history.clear")}
          </Button>
        </div>
      </div>

      <div className="mt-3 max-h-[28rem] space-y-2 overflow-auto">
        {props.history.length === 0 ? (
          <p className="text-xs text-text-secondary">{t("history.empty")}</p>
        ) : null}

        {props.history.map((item) => (
          <article key={item.id} className="rounded-3 border border-border-muted p-2">
            <div className="text-xs font-medium text-text-primary">{item.peerName}</div>
            <div className="mt-1 text-[11px] text-text-secondary">{item.status}</div>
            <div className="mt-1 text-[11px] text-text-secondary">{formatTime(item.createdAt)}</div>
          </article>
        ))}
      </div>
    </section>
  );
}

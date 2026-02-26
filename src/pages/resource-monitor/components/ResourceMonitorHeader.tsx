import { useTranslation } from "react-i18next";

import { Button, Select } from "@/components/ui";

interface ResourceMonitorHeaderProps {
  initialized: boolean;
  loading: boolean;
  error: string | null;
  historyWindowMinutes: 5 | 15 | 30;
  sampledAtText: string;
  lastUpdatedText: string;
  historyPointsText: string;
  onRefresh: () => void;
  onResetSession: () => void;
  onHistoryWindowChange: (value: 5 | 15 | 30) => void;
}

export default function ResourceMonitorHeader(props: ResourceMonitorHeaderProps) {
  const { t } = useTranslation(["resource_monitor"]);

  return (
    <section className="ui-glass-panel-strong rounded-2xl p-4">
      <div className="font-mono ui-text-micro uppercase tracking-ui-wider text-text-muted">
        rtool / resource monitor
      </div>
      <div className="mt-2 flex flex-wrap items-center gap-2">
        <h1 className="m-0 text-xl font-semibold tracking-tight text-text-primary">{t("header.title")}</h1>
        <span className="ui-glass-chip px-2 py-0.5 font-mono ui-text-micro uppercase tracking-ui-wide text-accent">
          {props.loading && !props.initialized
            ? t("status.booting")
            : props.error
              ? t("status.degraded")
              : t("status.online")}
        </span>
      </div>
      <p className="mt-2 text-sm text-text-secondary">{t("header.subtitle")}</p>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Button size="default" variant="secondary" onClick={props.onRefresh}>
          <span className="btn-icon i-lucide:refresh-cw" aria-hidden="true" />
          <span>{t("action.refreshNow")}</span>
        </Button>

        <Button size="default" variant="danger" onClick={props.onResetSession}>
          <span className="btn-icon i-noto:broom" aria-hidden="true" />
          <span>{t("action.resetSession")}</span>
        </Button>

        <div className="w-[150px]">
          <Select
            value={`${props.historyWindowMinutes}`}
            options={[
              { value: "5", label: t("filter.window.5m") },
              { value: "15", label: t("filter.window.15m") },
              { value: "30", label: t("filter.window.30m") },
            ]}
            onChange={(event) => {
              const value = Number(event.target.value);
              if (value === 5 || value === 15 || value === 30) {
                props.onHistoryWindowChange(value);
              }
            }}
          />
        </div>
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-text-muted">
        <span>{props.sampledAtText}</span>
        <span>{props.lastUpdatedText}</span>
        <span>{props.historyPointsText}</span>
      </div>

      {props.error ? (
        <div className="mt-2 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
          {t("error.sampleFailed", { message: props.error })}
        </div>
      ) : null}
    </section>
  );
}

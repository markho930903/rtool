import { useTranslation } from "react-i18next";

import { Button, Select, SwitchField } from "@/components/ui";
import type { LoggingSettingsSectionState } from "@/pages/settings/hooks/useSettingsPageState";

interface LoggingSettingsSectionProps {
  state: LoggingSettingsSectionState;
}

export default function LoggingSettingsSection(props: LoggingSettingsSectionProps) {
  const { t } = useTranslation("settings");

  return (
    <section className="h-full min-h-0">
      <div className="space-y-3">
        <h2 className="m-0 text-sm font-semibold text-text-primary">{t("logging.title")}</h2>
        <p className="m-0 text-xs text-text-muted">{t("logging.desc")}</p>

        <div className="max-w-[560px] space-y-3">
          <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
            <div className="space-y-1">
              <label className="text-xs text-text-secondary" htmlFor="logging-min-level">
                {t("logging.minLevel")}
              </label>
              <Select
                id="logging-min-level"
                value={props.state.minLevel}
                options={[
                  { value: "trace", label: "trace" },
                  { value: "debug", label: "debug" },
                  { value: "info", label: "info" },
                  { value: "warn", label: "warn" },
                  { value: "error", label: "error" },
                ]}
                onChange={(event) => props.state.onMinLevelChange(event.currentTarget.value)}
              />
            </div>
          </div>

          <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
            <div className="space-y-1">
              <label className="text-xs text-text-secondary" htmlFor="logging-keep-days">
                {t("logging.keepDays", { min: props.state.limits.keepDaysMin, max: props.state.limits.keepDaysMax })}
              </label>
              <Select
                id="logging-keep-days"
                value={props.state.keepDaysInput}
                invalid={props.state.keepDaysInvalid}
                options={props.state.keepDaysOptions}
                onChange={(event) => props.state.onKeepDaysChange(event.currentTarget.value)}
              />
            </div>
          </div>

          <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
            <div className="space-y-1">
              <label className="text-xs text-text-secondary" htmlFor="logging-high-freq-window">
                {t("logging.windowMs", {
                  min: props.state.limits.windowMsMin,
                  max: props.state.limits.windowMsMax,
                })}
              </label>
              <Select
                id="logging-high-freq-window"
                value={props.state.highFreqWindowMsInput}
                invalid={props.state.highFreqWindowInvalid}
                options={props.state.windowMsOptions}
                onChange={(event) => props.state.onHighFreqWindowMsChange(event.currentTarget.value)}
              />
            </div>
          </div>

          <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
            <div className="space-y-1">
              <label className="text-xs text-text-secondary" htmlFor="logging-high-freq-max">
                {t("logging.maxPerKey", {
                  min: props.state.limits.maxPerKeyMin,
                  max: props.state.limits.maxPerKeyMax,
                })}
              </label>
              <Select
                id="logging-high-freq-max"
                value={props.state.highFreqMaxPerKeyInput}
                invalid={props.state.highFreqMaxPerKeyInvalid}
                options={props.state.maxPerKeyOptions}
                onChange={(event) => props.state.onHighFreqMaxPerKeyChange(event.currentTarget.value)}
              />
            </div>
          </div>

          <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
            <SwitchField
              checked={props.state.realtimeEnabled}
              controlPosition="end"
              onChange={(event) => props.state.onRealtimeEnabledChange(event.currentTarget.checked)}
              wrapperClassName="text-sm text-text-primary"
              labelClassName="gap-1"
              label={<span className="text-sm font-medium leading-5">{t("logging.realtime.label")}</span>}
              description={<span className="leading-5">{t("logging.realtime.desc")}</span>}
            />
          </div>

          <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
            <SwitchField
              checked={props.state.allowRawView}
              controlPosition="end"
              onChange={(event) => props.state.onAllowRawViewChange(event.currentTarget.checked)}
              wrapperClassName="text-sm text-text-primary"
              labelClassName="gap-1"
              label={<span className="text-sm font-medium leading-5">{t("logging.raw.label")}</span>}
              description={<span className="leading-5">{t("logging.raw.desc")}</span>}
            />
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="default"
            variant="primary"
            disabled={props.state.invalid || props.state.unchanged}
            onClick={() => {
              void props.state.onSave();
            }}
          >
            {t("logging.save")}
          </Button>
          {props.state.error ? <span className="text-xs text-danger">{props.state.error}</span> : null}
          {props.state.saveMessage ? (
            <span className={`text-xs ${props.state.saveMessage.isError ? "text-danger" : "text-text-secondary"}`}>
              {props.state.saveMessage.text}
            </span>
          ) : null}
        </div>
      </div>
    </section>
  );
}

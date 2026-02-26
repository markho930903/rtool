import { useTranslation } from "react-i18next";

import { LoadingIndicator } from "@/components/loading";
import { Button, Input, SwitchField } from "@/components/ui";
import type { ClipboardSettingsSectionState } from "@/pages/settings/hooks/useSettingsPageState";

interface ClipboardSettingsSectionProps {
  state: ClipboardSettingsSectionState;
}

export default function ClipboardSettingsSection(props: ClipboardSettingsSectionProps) {
  const { t } = useTranslation("settings");

  return (
    <section className="h-full min-h-0">
      <div className="space-y-3">
        <h2 className="m-0 text-sm font-semibold text-text-primary">{t("clipboard.title")}</h2>
        <p className="m-0 text-xs text-text-muted">{t("clipboard.desc")}</p>

        <div className="grid max-w-[420px] gap-2">
          <label htmlFor="clipboard-max-items" className="text-xs text-text-secondary">
            {t("clipboard.maxItems")}
          </label>
          <Input
            id="clipboard-max-items"
            type="number"
            min={props.state.limits.maxItemsMin}
            max={props.state.limits.maxItemsMax}
            value={props.state.maxItemsInput}
            invalid={props.state.maxItemsInvalid}
            onChange={(event) => props.state.onMaxItemsChange(event.currentTarget.value)}
          />
          <p className={`m-0 text-xs ${props.state.maxItemsInvalid ? "text-danger" : "text-text-muted"}`}>
            {props.state.clipboardMaxItemsHelperText}
          </p>
        </div>

        <div className="max-w-[560px] space-y-3 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
          <SwitchField
            checked={props.state.sizeCleanupEnabled}
            label={t("clipboard.sizeCleanupEnabled")}
            description={t("clipboard.sizeCleanupEnabledDesc")}
            controlPosition="end"
            onChange={(event) => props.state.onSizeCleanupEnabledChange(event.currentTarget.checked)}
          />

          <div className="space-y-2">
            <label className="text-xs text-text-secondary">{t("clipboard.sizePreset")}</label>
            <div role="radiogroup" aria-label={t("clipboard.sizePreset")} className="grid gap-2 sm:grid-cols-2">
              {props.state.presets.map((presetValue) => {
                const active =
                  props.state.sizeThresholdMode === "preset" && props.state.selectedPresetMb === presetValue;
                return (
                  <Button
                    key={presetValue}
                    size="default"
                    variant={active ? "primary" : "secondary"}
                    disabled={!props.state.sizeCleanupEnabled}
                    aria-pressed={active}
                    className="justify-start"
                    onClick={() => props.state.onPresetSelect(presetValue)}
                  >
                    {t("clipboard.sizePresetLabel", { value: presetValue })}
                  </Button>
                );
              })}
              <Button
                size="default"
                variant={props.state.sizeThresholdMode === "custom" ? "primary" : "secondary"}
                disabled={!props.state.sizeCleanupEnabled}
                aria-pressed={props.state.sizeThresholdMode === "custom"}
                className="justify-start"
                onClick={props.state.onCustomModeSelect}
              >
                {t("clipboard.sizePresetCustom")}
              </Button>
            </div>
          </div>

          {props.state.sizeThresholdMode === "custom" ? (
            <div className="space-y-1">
              <label htmlFor="clipboard-size-custom" className="text-xs text-text-secondary">
                {t("clipboard.maxTotalSizeMb")}
              </label>
              <Input
                ref={props.state.customSizeInputRef}
                id="clipboard-size-custom"
                type="number"
                min={props.state.limits.maxTotalSizeMin}
                max={props.state.limits.maxTotalSizeMax}
                disabled={!props.state.sizeCleanupEnabled}
                value={props.state.customSizeMbInput}
                invalid={props.state.maxTotalSizeInvalid}
                onChange={(event) => props.state.onCustomSizeChange(event.currentTarget.value)}
              />
            </div>
          ) : null}

          <p className={`m-0 text-xs ${props.state.maxTotalSizeInvalid ? "text-danger" : "text-text-muted"}`}>
            {props.state.clipboardSizeHelperText}
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="default"
            variant="primary"
            disabled={props.state.loading || props.state.saving || props.state.invalid || props.state.unchanged}
            onClick={() => {
              void props.state.onSave();
            }}
          >
            {props.state.saving ? t("common:action.saving") : t("common:action.save")}
          </Button>
          {props.state.loading ? <LoadingIndicator text={t("common:status.loading")} /> : null}
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

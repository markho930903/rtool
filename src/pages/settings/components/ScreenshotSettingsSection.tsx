import { useTranslation } from "react-i18next";

import { Button } from "@ui/button";
import { Input } from "@ui/input";
import { Message } from "@ui/message/Message";
import { SwitchField } from "@ui/switch";
import type { ScreenshotSettingsSectionState } from "@/pages/settings/hooks/useSettingsPageState";

interface ScreenshotSettingsSectionProps {
  state: ScreenshotSettingsSectionState;
}

export default function ScreenshotSettingsSection(props: ScreenshotSettingsSectionProps) {
  const { t } = useTranslation("settings");

  return (
    <section className="h-full min-h-0">
      <div className="space-y-3">
        <h2 className="m-0 text-sm font-semibold text-text-primary">{t("screenshot.title")}</h2>
        <p className="m-0 text-xs text-text-muted">{t("screenshot.desc")}</p>

        <div className="max-w-[640px] space-y-3 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
          <div className="space-y-1">
            <label htmlFor="screenshot-shortcut" className="text-xs text-text-secondary">
              {t("screenshot.shortcut")}
            </label>
            <Input
              id="screenshot-shortcut"
              value={props.state.shortcutInput}
              invalid={props.state.shortcutInvalid}
              placeholder="Alt+Shift+S"
              onChange={(event) => props.state.onShortcutChange(event.currentTarget.value)}
            />
            <p className={`m-0 text-xs ${props.state.shortcutInvalid ? "text-danger" : "text-text-muted"}`}>
              {props.state.shortcutInvalid ? t("screenshot.shortcutInvalid") : t("screenshot.shortcutHint")}
            </p>
          </div>

          <SwitchField
            checked={props.state.autoSaveEnabled}
            label={t("screenshot.autoSaveEnabled")}
            description={t("screenshot.autoSaveEnabledDesc")}
            controlPosition="end"
            onChange={(event) => props.state.onAutoSaveEnabledChange(event.currentTarget.checked)}
          />

          <div className="grid gap-2 md:grid-cols-2">
            <div className="space-y-1">
              <label htmlFor="screenshot-max-items" className="text-xs text-text-secondary">
                {t("screenshot.maxItems")}
              </label>
              <Input
                id="screenshot-max-items"
                type="number"
                min={props.state.limits.maxItemsMin}
                max={props.state.limits.maxItemsMax}
                value={props.state.maxItemsInput}
                invalid={props.state.maxItemsInvalid}
                onChange={(event) => props.state.onMaxItemsChange(event.currentTarget.value)}
              />
              <p className={`m-0 text-xs ${props.state.maxItemsInvalid ? "text-danger" : "text-text-muted"}`}>
                {t("screenshot.maxItemsHint", {
                  min: props.state.limits.maxItemsMin,
                  max: props.state.limits.maxItemsMax,
                })}
              </p>
            </div>

            <div className="space-y-1">
              <label htmlFor="screenshot-max-total-size" className="text-xs text-text-secondary">
                {t("screenshot.maxTotalSizeMb")}
              </label>
              <Input
                id="screenshot-max-total-size"
                type="number"
                min={props.state.limits.maxTotalSizeMin}
                max={props.state.limits.maxTotalSizeMax}
                value={props.state.maxTotalSizeInput}
                invalid={props.state.maxTotalSizeInvalid}
                onChange={(event) => props.state.onMaxTotalSizeChange(event.currentTarget.value)}
              />
              <p className={`m-0 text-xs ${props.state.maxTotalSizeInvalid ? "text-danger" : "text-text-muted"}`}>
                {t("screenshot.maxTotalSizeHint", {
                  min: props.state.limits.maxTotalSizeMin,
                  max: props.state.limits.maxTotalSizeMax,
                })}
              </p>
            </div>
          </div>

          <div className="space-y-1">
            <label htmlFor="screenshot-pin-max-instances" className="text-xs text-text-secondary">
              {t("screenshot.pinMaxInstances")}
            </label>
            <Input
              id="screenshot-pin-max-instances"
              type="number"
              min={props.state.limits.pinMaxInstancesMin}
              max={props.state.limits.pinMaxInstancesMax}
              value={props.state.pinMaxInstancesInput}
              invalid={props.state.pinMaxInstancesInvalid}
              onChange={(event) => props.state.onPinMaxInstancesChange(event.currentTarget.value)}
            />
            <p className={`m-0 text-xs ${props.state.pinMaxInstancesInvalid ? "text-danger" : "text-text-muted"}`}>
              {t("screenshot.pinMaxInstancesHint", {
                min: props.state.limits.pinMaxInstancesMin,
                max: props.state.limits.pinMaxInstancesMax,
              })}
            </p>
          </div>
        </div>

        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="default"
              variant="primary"
              disabled={
                props.state.loading ||
                props.state.saving ||
                props.state.shortcutInvalid ||
                props.state.maxItemsInvalid ||
                props.state.maxTotalSizeInvalid ||
                props.state.pinMaxInstancesInvalid ||
                props.state.unchanged
              }
              onClick={() => {
                void props.state.onSave();
              }}
            >
              {props.state.saving ? t("common:action.saving") : t("common:action.save")}
            </Button>
          </div>
          {props.state.saveMessage ? (
            <Message
              type={props.state.saveMessage.isError ? "error" : "success"}
              description={props.state.saveMessage.text}
              className="max-w-[640px]"
            />
          ) : null}
        </div>
      </div>
    </section>
  );
}

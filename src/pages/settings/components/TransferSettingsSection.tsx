import { useTranslation } from "react-i18next";

import { Button, Input, SwitchField } from "@/components/ui";
import type { TransferSettingsSectionState } from "@/pages/settings/hooks/useSettingsPageState";

interface TransferSettingsSectionProps {
  state: TransferSettingsSectionState;
}

export default function TransferSettingsSection(props: TransferSettingsSectionProps) {
  const { t } = useTranslation("settings");

  return (
    <section className="h-full min-h-0">
      <div className="space-y-3">
        <h2 className="m-0 text-sm font-semibold text-text-primary">{t("transfer.title")}</h2>
        <p className="m-0 text-xs text-text-muted">{t("transfer.desc")}</p>

        <div className="max-w-[640px] space-y-3 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
          <div className="space-y-1">
            <label htmlFor="transfer-default-dir" className="text-xs text-text-secondary">
              {t("transfer.defaultDir")}
            </label>
            <Input
              id="transfer-default-dir"
              value={props.state.defaultDirInput}
              invalid={props.state.transferDirInvalid}
              onChange={(event) => props.state.onDefaultDirChange(event.currentTarget.value)}
            />
          </div>

          <div className="space-y-1">
            <label htmlFor="transfer-auto-cleanup-days" className="text-xs text-text-secondary">
              {t("transfer.autoCleanupDays")}
            </label>
            <Input
              id="transfer-auto-cleanup-days"
              type="number"
              min={1}
              max={365}
              value={props.state.autoCleanupDaysInput}
              invalid={props.state.transferCleanupInvalid}
              onChange={(event) => props.state.onAutoCleanupDaysChange(event.currentTarget.value)}
            />
          </div>

          <SwitchField
            checked={props.state.resumeEnabled}
            label={t("transfer.resumeEnabled")}
            controlPosition="end"
            onChange={(event) => props.state.onResumeEnabledChange(event.currentTarget.checked)}
          />
          <SwitchField
            checked={props.state.discoveryEnabled}
            label={t("transfer.discoveryEnabled")}
            controlPosition="end"
            onChange={(event) => props.state.onDiscoveryEnabledChange(event.currentTarget.checked)}
          />
          <SwitchField
            checked={props.state.pairingRequired}
            label={t("transfer.pairingRequired")}
            controlPosition="end"
            onChange={(event) => props.state.onPairingRequiredChange(event.currentTarget.checked)}
          />
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="default"
            variant="primary"
            disabled={props.state.loading || props.state.saving || props.state.transferDirInvalid || props.state.transferCleanupInvalid}
            onClick={() => {
              void props.state.onSave();
            }}
          >
            {props.state.saving ? t("common:action.saving") : t("transfer.save")}
          </Button>
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

import { useTranslation } from "react-i18next";

import { RadioGroup, Select, SwitchField } from "@/components/ui";
import type { GeneralSettingsSectionState } from "@/pages/settings/hooks/useSettingsPageState";

interface GeneralSettingsSectionProps {
  state: GeneralSettingsSectionState;
}

export default function GeneralSettingsSection(props: GeneralSettingsSectionProps) {
  const { t } = useTranslation("settings");

  return (
    <section className="h-full min-h-0">
      <div className="space-y-5">
        <div className="space-y-1.5">
          <h2 className="m-0 text-sm font-semibold text-text-primary">{t("general.title")}</h2>
          <p className="m-0 text-xs text-text-muted">{t("general.desc")}</p>
        </div>

        <div className="max-w-[860px] overflow-hidden rounded-lg border border-border-strong bg-surface shadow-surface">
          <div className="grid gap-0 md:grid-cols-[220px_1fr] md:items-center">
            <div className="border-b border-border-strong px-4 py-3 md:border-b-0 md:border-r">
              <div className="text-xs font-semibold text-text-primary">{t("general.preference")}</div>
              <div className="mt-1 text-xs text-text-muted">{t("general.effective", { locale: props.state.resolvedLocaleLabel })}</div>
            </div>
            <div className="px-4 py-3">
              <Select
                id="locale-preference"
                value={props.state.localePreference}
                options={props.state.localePreferenceOptions}
                onChange={(event) => props.state.onLocalePreferenceChange(event.currentTarget.value)}
              />
            </div>
          </div>

          <div className="grid gap-0 border-t border-border-strong md:grid-cols-[220px_1fr] md:items-center">
            <div className="border-b border-border-strong px-4 py-3 md:border-b-0 md:border-r">
              <div className="text-xs font-semibold text-text-primary">{t("general.layoutPreference")}</div>
              <div className="mt-1 text-xs text-text-muted">{t("general.layout.desc")}</div>
            </div>
            <div className="px-4 py-3">
              <Select
                id="layout-preference"
                value={props.state.layoutPreference}
                options={props.state.layoutPreferenceOptions}
                onChange={(event) => props.state.onLayoutPreferenceChange(event.currentTarget.value)}
              />
            </div>
          </div>
        </div>

        <div className="space-y-1.5">
          <h3 className="m-0 text-sm font-semibold text-text-primary">{t("general.appearance.title")}</h3>
          <p className="m-0 text-xs text-text-muted">{t("general.appearance.desc")}</p>
        </div>

        <div className="max-w-[860px] overflow-hidden rounded-lg border border-border-strong bg-surface shadow-surface">
          <div className="grid gap-0 md:grid-cols-[220px_1fr] md:items-center">
            <div className="border-b border-border-strong px-4 py-3 md:border-b-0 md:border-r">
              <div className="text-xs font-semibold text-text-primary">{t("general.theme.label")}</div>
              <div className="mt-1 text-xs text-text-muted">{t("general.theme.desc")}</div>
            </div>
            <div className="px-4 py-3">
              <RadioGroup
                name="theme-preference"
                value={props.state.themePreference}
                options={props.state.themePreferenceOptions}
                orientation="horizontal"
                size="md"
                onValueChange={props.state.onThemePreferenceChange}
                className="w-full flex-nowrap items-stretch gap-2"
                optionClassName="w-fit min-h-10 shrink-0 items-center overflow-visible rounded-md border border-border-strong bg-surface-soft px-3 py-2 text-xs text-text-secondary shadow-inset-soft transition-[background-color,border-color,color] duration-150 hover:border-accent/45 hover:bg-surface"
              />
            </div>
          </div>

          <div className="grid gap-0 border-t border-border-strong md:grid-cols-[220px_1fr] md:items-center">
            <div className="border-b border-border-strong px-4 py-3 md:border-b-0 md:border-r">
              <div className="text-xs font-semibold text-text-primary">{t("general.transparentWindowBackground.label")}</div>
              <div className="mt-1 text-xs text-text-muted">{t("general.transparentWindowBackground.desc")}</div>
            </div>
            <div className="px-4 py-3">
              <SwitchField
                checked={props.state.transparentWindowBackground}
                controlPosition="end"
                label={t("general.transparentWindowBackground.toggle")}
                description={t("general.transparentWindowBackground.toggleDesc")}
                onChange={(event) => props.state.onTransparentWindowBackgroundChange(event.currentTarget.checked)}
              />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

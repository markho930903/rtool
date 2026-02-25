import { useTranslation } from "react-i18next";

import { LoadingIndicator } from "@/components/loading";
import { Button, Input, RadioGroup, Select, Slider } from "@/components/ui";
import type { GeneralSettingsSectionState } from "@/pages/settings/hooks/useSettingsPageState";
import { GLASS_RANGES } from "@/theme/constants";

interface GeneralSettingsSectionProps {
  state: GeneralSettingsSectionState;
}

export default function GeneralSettingsSection(props: GeneralSettingsSectionProps) {
  const { t } = useTranslation("settings");

  return (
    <section className="h-full min-h-0">
      <div className="space-y-3">
        <h2 className="m-0 text-sm font-semibold text-text-primary">{t("general.title")}</h2>
        <p className="m-0 text-xs text-text-muted">{t("general.desc")}</p>

        <div className="grid max-w-[420px] gap-2">
          <label htmlFor="locale-preference" className="text-xs text-text-secondary">
            {t("general.preference")}
          </label>
          <Select
            id="locale-preference"
            value={props.state.localePreference}
            options={props.state.localePreferenceOptions}
            onChange={(event) => props.state.onLocalePreferenceChange(event.currentTarget.value)}
          />
          <p className="m-0 text-xs text-text-muted">
            {t("general.effective", {
              locale: props.state.resolvedLocaleLabel,
            })}
          </p>
        </div>

        <div className="grid max-w-[420px] gap-2">
          <label htmlFor="layout-preference" className="text-xs text-text-secondary">
            {t("general.layoutPreference")}
          </label>
          <Select
            id="layout-preference"
            value={props.state.layoutPreference}
            options={props.state.layoutPreferenceOptions}
            onChange={(event) => props.state.onLayoutPreferenceChange(event.currentTarget.value)}
          />
        </div>

        <div className="mt-5 max-w-[640px] space-y-3 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
          <h3 className="m-0 text-sm font-semibold text-text-primary">{t("general.glass.title")}</h3>
          <p className="m-0 text-xs text-text-muted">{t("general.glass.desc")}</p>
          <p className="m-0 text-xs text-text-secondary">
            {t("general.glass.currentTheme", {
              theme: props.state.effectiveThemeLabel,
            })}
          </p>

          <div className="grid max-w-[420px] gap-1.5">
            <div className="text-xs text-text-secondary">{t("general.glass.targetTheme")}</div>
            <RadioGroup
              name="glass-target-theme"
              value={props.state.glassTargetTheme}
              options={props.state.glassThemeOptions}
              orientation="horizontal"
              size="md"
              onValueChange={props.state.onGlassThemeChange}
              className="w-full flex-nowrap items-stretch gap-2"
              optionClassName="w-fit min-h-10 shrink-0 items-center overflow-visible rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-2 text-xs text-text-secondary shadow-inset-soft transition-colors duration-150 hover:border-border-glass-strong hover:bg-surface-glass"
            />
          </div>

          <div className="space-y-3">
            <div className="space-y-1">
              <div className="text-xs text-text-secondary">{t("general.glass.opacity")}</div>
              <Slider
                min={GLASS_RANGES.opacity.min}
                max={GLASS_RANGES.opacity.max}
                step={1}
                value={props.state.activeGlassProfile.opacity}
                variant="theme"
                showValue
                formatValue={(value) => `${value}%`}
                onValueChange={(value) => props.state.onPreviewGlassField("opacity", value)}
                onValueCommit={(value) => props.state.onCommitGlassField("opacity", value)}
              />
            </div>
            <div className="space-y-1">
              <div className="text-xs text-text-secondary">{t("general.glass.blur")}</div>
              <Slider
                min={GLASS_RANGES.blur.min}
                max={GLASS_RANGES.blur.max}
                step={1}
                value={props.state.activeGlassProfile.blur}
                variant="theme"
                showValue
                formatValue={(value) => `${value}px`}
                onValueChange={(value) => props.state.onPreviewGlassField("blur", value)}
                onValueCommit={(value) => props.state.onCommitGlassField("blur", value)}
              />
            </div>
            <div className="space-y-1">
              <div className="text-xs text-text-secondary">{t("general.glass.saturate")}</div>
              <Slider
                min={GLASS_RANGES.saturate.min}
                max={GLASS_RANGES.saturate.max}
                step={5}
                value={props.state.activeGlassProfile.saturate}
                variant="theme"
                showValue
                formatValue={(value) => `${value}%`}
                onValueChange={(value) => props.state.onPreviewGlassField("saturate", value)}
                onValueCommit={(value) => props.state.onCommitGlassField("saturate", value)}
              />
            </div>
            <div className="space-y-1">
              <div className="text-xs text-text-secondary">{t("general.glass.brightness")}</div>
              <Slider
                min={GLASS_RANGES.brightness.min}
                max={GLASS_RANGES.brightness.max}
                step={1}
                value={props.state.activeGlassProfile.brightness}
                variant="theme"
                showValue
                formatValue={(value) => `${value}%`}
                onValueChange={(value) => props.state.onPreviewGlassField("brightness", value)}
                onValueCommit={(value) => props.state.onCommitGlassField("brightness", value)}
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button size="default" variant="secondary" onClick={props.state.onResetGlassTheme}>
              {t("general.glass.resetCurrent")}
            </Button>
          </div>
        </div>

        <div className="mt-5 space-y-3 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
          <h3 className="m-0 text-sm font-semibold text-text-primary">{t("general.import.title")}</h3>
          <p className="m-0 text-xs text-text-muted">{t("general.import.desc")}</p>

          <p className="m-0 text-xs text-text-secondary">
            {t("general.import.catalog", {
              builtin: props.state.catalogSummary.builtin,
              overlay: props.state.catalogSummary.overlay,
              effective: props.state.catalogSummary.effective,
            })}
          </p>

          <div className="grid max-w-[520px] gap-2 md:grid-cols-2">
            <div className="space-y-1">
              <label htmlFor="locale-import-locale" className="text-xs text-text-secondary">
                {t("general.import.locale")}
              </label>
              <Select
                id="locale-import-locale"
                value={props.state.importLocale}
                options={props.state.importLocaleOptions}
                onChange={(event) => props.state.onImportLocaleChange(event.currentTarget.value)}
              />
            </div>

            <div className="space-y-1">
              <label htmlFor="locale-import-namespace" className="text-xs text-text-secondary">
                {t("general.import.namespace")}
              </label>
              <Select
                id="locale-import-namespace"
                value={props.state.importNamespace}
                options={props.state.importNamespaceOptions}
                onChange={(event) => props.state.onImportNamespaceChange(event.currentTarget.value)}
              />
            </div>
          </div>

          <div className="grid max-w-[520px] gap-2">
            <label htmlFor="locale-import-file" className="text-xs text-text-secondary">
              {t("general.import.file")}
            </label>
            <Input
              id="locale-import-file"
              type="file"
              accept=".json,application/json"
              onChange={(event) => {
                void props.state.onImportFileSelect(event.currentTarget.files?.[0] ?? null);
              }}
            />
            <p className="m-0 text-xs text-text-muted">
              {props.state.importFileName
                ? t("general.import.selected", { name: props.state.importFileName })
                : t("general.import.noFile")}
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="default"
              variant="primary"
              disabled={props.state.importingLocaleFile || !props.state.importFileContent.trim()}
              onClick={() => {
                void props.state.onImportLocaleFile();
              }}
            >
              {props.state.importingLocaleFile ? t("general.import.importing") : t("general.import.import")}
            </Button>
            <Button
              size="default"
              variant="secondary"
              disabled={props.state.localeCatalogLoading}
              onClick={() => {
                void props.state.onReloadLocales();
              }}
            >
              {t("general.import.reload")}
            </Button>
            {props.state.localeCatalogLoading ? <LoadingIndicator text={t("common:status.loading")} /> : null}
          </div>

          {props.state.localeCatalogError ? <p className="m-0 text-xs text-danger">{props.state.localeCatalogError}</p> : null}
          {props.state.importMessage ? (
            <p className={`m-0 text-xs ${props.state.importMessage.isError ? "text-danger" : "text-text-secondary"}`}>
              {props.state.importMessage.text}
            </p>
          ) : null}
        </div>
      </div>
    </section>
  );
}

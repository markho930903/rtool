import { useTranslation } from "react-i18next";

import { LoadingIndicator } from "@/components/loading";
import { Button, Input } from "@/components/ui";
import type { LauncherSettingsSectionState } from "@/pages/settings/hooks/useSettingsPageState";

interface LauncherSettingsSectionProps {
  state: LauncherSettingsSectionState;
}

export default function LauncherSettingsSection(props: LauncherSettingsSectionProps) {
  const { t } = useTranslation("settings");

  return (
    <section className="h-full min-h-0">
      <div className="space-y-3">
        <h2 className="m-0 text-sm font-semibold text-text-primary">{t("launcher.title")}</h2>
        <p className="m-0 text-xs text-text-muted">{t("launcher.desc")}</p>

        <div className="max-w-[760px] space-y-3 rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 shadow-inset-soft">
          <div className="space-y-1">
            <label htmlFor="launcher-roots" className="text-xs text-text-secondary">
              {t("launcher.roots")}
            </label>
            <textarea
              id="launcher-roots"
              className={[
                "min-h-[88px] w-full resize-y rounded-md border border-border-glass bg-surface-glass-soft px-2.5 py-2 text-xs shadow-inset-soft outline-none",
                props.state.rootsInvalid ? "border-danger" : "border-border-glass focus:border-border-glass-strong",
              ].join(" ")}
              value={props.state.rootsInput}
              onChange={(event) => props.state.onRootsChange(event.currentTarget.value)}
            />
            <p className={`m-0 text-xs ${props.state.rootsInvalid ? "text-danger" : "text-text-muted"}`}>
              {props.state.rootsInvalid ? t("launcher.rootsInvalid") : t("launcher.rootsHint")}
            </p>
          </div>

          <div className="space-y-1">
            <label htmlFor="launcher-excludes" className="text-xs text-text-secondary">
              {t("launcher.excludes")}
            </label>
            <textarea
              id="launcher-excludes"
              className="min-h-[110px] w-full resize-y rounded-md border border-border-glass bg-surface-glass-soft px-2.5 py-2 text-xs shadow-inset-soft outline-none focus:border-border-glass-strong"
              value={props.state.excludeInput}
              onChange={(event) => props.state.onExcludeChange(event.currentTarget.value)}
            />
            <p className="m-0 text-xs text-text-muted">{t("launcher.excludesHint")}</p>
          </div>

          <div className="grid gap-2 md:grid-cols-2">
            <div className="space-y-1">
              <label htmlFor="launcher-depth" className="text-xs text-text-secondary">
                {t("launcher.maxDepth")}
              </label>
              <Input
                id="launcher-depth"
                type="number"
                min={props.state.limits.depthMin}
                max={props.state.limits.depthMax}
                value={props.state.depthInput}
                invalid={props.state.depthInvalid}
                onChange={(event) => props.state.onDepthChange(event.currentTarget.value)}
              />
            </div>

            <div className="space-y-1">
              <label htmlFor="launcher-items-per-root" className="text-xs text-text-secondary">
                {t("launcher.maxItemsPerRoot")}
              </label>
              <Input
                id="launcher-items-per-root"
                type="number"
                min={props.state.limits.itemsPerRootMin}
                max={props.state.limits.itemsPerRootMax}
                value={props.state.itemsPerRootInput}
                invalid={props.state.itemsPerRootInvalid}
                onChange={(event) => props.state.onItemsPerRootChange(event.currentTarget.value)}
              />
            </div>

            <div className="space-y-1">
              <label htmlFor="launcher-total-items" className="text-xs text-text-secondary">
                {t("launcher.maxTotalItems")}
              </label>
              <Input
                id="launcher-total-items"
                type="number"
                min={props.state.limits.totalItemsMin}
                max={props.state.limits.totalItemsMax}
                value={props.state.totalItemsInput}
                invalid={props.state.totalItemsInvalid}
                onChange={(event) => props.state.onTotalItemsChange(event.currentTarget.value)}
              />
            </div>

            <div className="space-y-1">
              <label htmlFor="launcher-refresh" className="text-xs text-text-secondary">
                {t("launcher.refreshInterval")}
              </label>
              <Input
                id="launcher-refresh"
                type="number"
                min={props.state.limits.refreshMin}
                max={props.state.limits.refreshMax}
                value={props.state.refreshInput}
                invalid={props.state.refreshInvalid}
                onChange={(event) => props.state.onRefreshInputChange(event.currentTarget.value)}
              />
            </div>
          </div>
        </div>

        <div className="max-w-[760px] rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-3 text-xs text-text-secondary shadow-inset-soft">
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <span>
              {t("launcher.status.ready", {
                value: props.state.status?.ready ? t("launcher.value.yes") : t("launcher.value.no"),
              })}
            </span>
            <span>
              {t("launcher.status.building", {
                value: props.state.status?.building ? t("launcher.value.yes") : t("launcher.value.no"),
              })}
            </span>
            <span>{t("launcher.status.indexedItems", { value: props.state.status?.indexedItems ?? 0 })}</span>
            <span>{t("launcher.status.indexedRoots", { value: props.state.status?.indexedRoots ?? 0 })}</span>
            <span>{t("launcher.status.lastBuild", { value: props.state.launcherLastBuildText })}</span>
            <span>{t("launcher.status.lastDuration", { value: props.state.launcherLastDurationText })}</span>
            <span>{t("launcher.status.version", { value: props.state.status?.indexVersion ?? "--" })}</span>
            <span>
              {t("launcher.status.truncated", {
                value: props.state.status?.truncated ? t("launcher.value.yes") : t("launcher.value.no"),
              })}
            </span>
          </div>
          {props.state.status?.lastError ? (
            <p className="mt-2 mb-0 text-xs text-danger">{t("launcher.status.lastError", { value: props.state.status.lastError })}</p>
          ) : null}
          {props.state.launcherTruncatedHintText ? (
            <p className="mt-2 mb-0 text-xs text-text-muted">{props.state.launcherTruncatedHintText}</p>
          ) : null}
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
            {props.state.saving ? t("common:action.saving") : t("launcher.save")}
          </Button>
          <Button
            size="default"
            variant="secondary"
            disabled={props.state.loading || props.state.rebuilding}
            onClick={() => {
              void props.state.onRefreshStatus();
            }}
          >
            {t("launcher.refreshStatus")}
          </Button>
          <Button
            size="default"
            variant="secondary"
            disabled={props.state.loading || props.state.rebuilding}
            onClick={() => {
              void props.state.onRebuildIndex();
            }}
          >
            {props.state.rebuilding ? t("launcher.rebuilding") : t("launcher.rebuild")}
          </Button>
          {props.state.loading ? <LoadingIndicator text={t("common:status.loading")} /> : null}
          {props.state.message ? (
            <span className={`text-xs ${props.state.message.isError ? "text-danger" : "text-text-secondary"}`}>
              {props.state.message.text}
            </span>
          ) : null}
        </div>
      </div>
    </section>
  );
}

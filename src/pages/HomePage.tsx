import { useEffect, useMemo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui";
import { useAppStore } from "@/stores/app.store";
import { useDashboardStore, type DashboardHistoryPoint } from "@/stores/dashboard.store";

interface MetricCardProps {
  title: string;
  value: string;
  hint: string;
  tone?: "normal" | "accent";
}

interface InfoRowProps {
  label: string;
  value: string;
}

interface TerminalPanelProps {
  title: string;
  subtitle: string;
  children: ReactNode;
}

const MODULE_STATUS = [
  { nameKey: "module.mainWindow.name", detailKey: "module.mainWindow.detail", state: "online" },
  { nameKey: "module.launcher.name", detailKey: "module.launcher.detail", state: "online" },
  { nameKey: "module.clipboard.name", detailKey: "module.clipboard.detail", state: "online" },
  { nameKey: "module.tools.name", detailKey: "module.tools.detail", state: "online" },
] as const;

function formatBytes(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes) || bytes < 0) {
    return "--";
  }

  if (bytes === 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB", "TB"] as const;
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** exponent;
  const digits = exponent <= 1 ? 0 : value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${units[exponent]}`;
}

function formatUptime(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) {
    return "--";
  }

  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);

  if (d > 0) {
    return `${d}d ${h}h ${m}m`;
  }
  if (h > 0) {
    return `${h}h ${m}m ${s}s`;
  }
  if (m > 0) {
    return `${m}m ${s}s`;
  }
  return `${s}s`;
}

function formatTimestamp(value: number | null, locale: string): string {
  if (value === null || !Number.isFinite(value) || value <= 0) {
    return "--";
  }

  return new Intl.DateTimeFormat(locale, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(value));
}

function formatPercent(value: number | null): string {
  if (value === null || !Number.isFinite(value)) {
    return "--";
  }

  return `${(value * 100).toFixed(1)}%`;
}

function nonEmpty(value: string | null | undefined): string {
  const normalized = value?.trim();
  return normalized ? normalized : "--";
}

function joinNonEmpty(values: Array<string | null | undefined>): string {
  const tokens = values.map((value) => value?.trim()).filter((value): value is string => Boolean(value));

  if (tokens.length === 0) {
    return "--";
  }

  return tokens.join(" ");
}

function MetricCard(props: MetricCardProps) {
  const valueClassName = props.tone === "accent" ? "text-accent" : "text-text-primary";

  return (
    <article className="relative overflow-hidden rounded-xl border border-border-muted bg-surface px-4 py-3">
      <div className="font-mono text-[11px] uppercase tracking-wide text-text-muted">{props.title}</div>
      <div className={`mt-2 font-mono text-[1.65rem] leading-none font-semibold ${valueClassName}`}>{props.value}</div>
      <div className="mt-1.5 text-xs text-text-secondary">{props.hint}</div>
    </article>
  );
}

function InfoRow(props: InfoRowProps) {
  return (
    <div className="flex items-start justify-between gap-3 border-b border-border-muted/65 py-2 last:border-b-0">
      <span className="text-xs text-text-muted">{props.label}</span>
      <span className="text-right font-mono text-xs text-text-primary">{props.value}</span>
    </div>
  );
}

function TerminalPanel(props: TerminalPanelProps) {
  return (
    <section className="rounded-xl border border-border-muted bg-surface p-4">
      <header className="mb-3">
        <div className="font-mono text-[11px] uppercase tracking-wide text-text-muted">{props.title}</div>
        <h2 className="mt-1 text-sm font-semibold text-text-primary">{props.subtitle}</h2>
      </header>
      {props.children}
    </section>
  );
}

function MemorySparkline(props: {
  points: DashboardHistoryPoint[];
  locale: string;
  t: (key: string, options?: Record<string, unknown>) => string;
}) {
  if (props.points.length === 0) {
    return <div className="py-5 text-center text-xs text-text-muted">{props.t("memory.noData")}</div>;
  }

  const values = props.points
    .map((point) => point.appMemoryBytes)
    .filter((value): value is number => value !== null && Number.isFinite(value) && value >= 0);
  const max = Math.max(...values, 1);
  const min = values.length === 0 ? 0 : Math.min(...values);

  return (
    <div>
      <div className="grid h-24 grid-cols-[repeat(20,minmax(0,1fr))] items-end gap-1 rounded-lg border border-border-muted bg-app/65 px-2 py-2">
        {props.points.map((point) => {
          const value = point.appMemoryBytes ?? 0;
          const height = values.length === 0 ? 0.1 : Math.max(value / max, 0.08);

          return (
            <div
              key={point.sampledAt}
              className="rounded-sm bg-accent/75 transition-[height,opacity] duration-300"
              style={{ height: `${(height * 100).toFixed(2)}%`, opacity: point.appMemoryBytes === null ? 0.25 : 1 }}
              title={`${formatTimestamp(point.sampledAt, props.locale)}  ${formatBytes(point.appMemoryBytes)}`}
            />
          );
        })}
      </div>
      <div className="mt-2 flex justify-between text-[11px] text-text-muted">
        <span>{props.t("timeline.min", { value: formatBytes(min) })}</span>
        <span>{props.t("timeline.max", { value: formatBytes(max) })}</span>
      </div>
    </div>
  );
}

export default function HomePage() {
  const { t, i18n } = useTranslation("home");
  const emptyToken = t("common:status.empty");
  const locale = i18n.resolvedLanguage ?? i18n.language;

  const snapshot = useDashboardStore((state) => state.snapshot);
  const history = useDashboardStore((state) => state.history);
  const loading = useDashboardStore((state) => state.loading);
  const error = useDashboardStore((state) => state.error);
  const lastUpdatedAt = useDashboardStore((state) => state.lastUpdatedAt);
  const refresh = useDashboardStore((state) => state.refresh);
  const startPolling = useDashboardStore((state) => state.startPolling);
  const stopPolling = useDashboardStore((state) => state.stopPolling);
  const windowMode = useAppStore((state) => state.windowMode);

  useEffect(() => {
    startPolling();
    return () => {
      stopPolling();
    };
  }, [startPolling, stopPolling]);

  const systemUsageRate =
    snapshot?.system.totalMemoryBytes &&
    snapshot.system.totalMemoryBytes > 0 &&
    snapshot.system.usedMemoryBytes !== null
      ? snapshot.system.usedMemoryBytes / snapshot.system.totalMemoryBytes
      : null;

  const statusLabel = error ? t("status.degraded") : loading && !snapshot ? t("status.booting") : t("status.online");
  const sampledAt = snapshot?.sampledAt ?? null;

  const moduleStatus = useMemo(
    () => MODULE_STATUS.map((item) => ({ ...item, name: t(item.nameKey), detail: t(item.detailKey) })),
    [t],
  );

  return (
    <div className="space-y-3 pb-2">
      <section className="rounded-2xl border border-border-strong bg-surface p-4">
        <div>
          <div className="font-mono text-[11px] uppercase tracking-widest text-text-muted">
            rtool / dashboard / live telemetry
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <h1 className="m-0 text-xl font-semibold tracking-tight text-text-primary">{t("header.title")}</h1>
            <span className="rounded-full border border-border-muted bg-app px-2 py-0.5 font-mono text-[11px] uppercase tracking-wide text-accent">
              {statusLabel}
            </span>
          </div>
          <p className="mt-2 max-w-3xl text-sm text-text-secondary">{t("header.subtitle")}</p>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Button size="sm" variant="secondary" onClick={() => void refresh()}>
              <span
                className="btn-icon i-noto:anticlockwise-downwards-and-upwards-open-circle-arrows"
                aria-hidden="true"
              />
              <span>{t("action.refreshNow")}</span>
            </Button>
            <Button as="link" to="/tools" variant="primary">
              <span className="btn-icon i-noto:hammer-and-wrench" aria-hidden="true" />
              <span>{t("action.openTools")}</span>
            </Button>
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-text-muted">
            <span>{t("meta.sampledAt", { value: formatTimestamp(sampledAt, locale) })}</span>
            <span>{t("meta.lastUpdated", { value: formatTimestamp(lastUpdatedAt, locale) })}</span>
            <span>{t("meta.windowMode", { value: windowMode })}</span>
          </div>
          {error ? (
            <div className="mt-2 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
              {t("error.sampleFailed", { message: error })}
            </div>
          ) : null}
        </div>
      </section>

      <section className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          title={t("metric.appMemory.title")}
          value={formatBytes(snapshot?.app.processMemoryBytes ?? null)}
          hint={t("metric.appMemory.hint")}
          tone="accent"
        />
        <MetricCard
          title={t("metric.uptime.title")}
          value={formatUptime(snapshot?.app.uptimeSeconds ?? null)}
          hint={t("metric.uptime.hint")}
        />
        <MetricCard
          title={t("metric.database.title")}
          value={formatBytes(snapshot?.app.databaseSizeBytes ?? null)}
          hint={t("metric.database.hint")}
        />
        <MetricCard
          title={t("metric.systemMemory.title")}
          value={formatPercent(systemUsageRate)}
          hint={`${formatBytes(snapshot?.system.usedMemoryBytes ?? null)} / ${formatBytes(snapshot?.system.totalMemoryBytes ?? null)}`}
        />
      </section>

      <section className="grid grid-cols-1 gap-3 xl:grid-cols-2">
        <TerminalPanel title={t("panel.application.title")} subtitle={t("panel.application.subtitle")}>
          <InfoRow label={t("info.appName")} value={snapshot?.app.appName ?? emptyToken} />
          <InfoRow label={t("info.version")} value={snapshot?.app.appVersion ?? emptyToken} />
          <InfoRow label={t("info.buildMode")} value={snapshot?.app.buildMode ?? emptyToken} />
          <InfoRow label={t("info.runtime")} value={formatUptime(snapshot?.app.uptimeSeconds ?? null)} />
          <InfoRow label={t("info.mainShortcut")} value={t("shortcut.main")} />
          <InfoRow label={t("info.clipboardShortcut")} value={t("shortcut.clipboard")} />
          <InfoRow label={t("info.currentWindowMode")} value={windowMode} />
        </TerminalPanel>

        <TerminalPanel title={t("panel.host.title")} subtitle={t("panel.host.subtitle")}>
          <InfoRow label={t("info.os")} value={joinNonEmpty([snapshot?.system.osName, snapshot?.system.osVersion])} />
          <InfoRow label={t("info.kernel")} value={nonEmpty(snapshot?.system.kernelVersion)} />
          <InfoRow label={t("info.arch")} value={nonEmpty(snapshot?.system.arch)} />
          <InfoRow label={t("info.host")} value={nonEmpty(snapshot?.system.hostName)} />
          <InfoRow label={t("info.cpu")} value={nonEmpty(snapshot?.system.cpuBrand)} />
          <InfoRow
            label={t("info.cpuCores")}
            value={snapshot?.system.cpuCores ? `${snapshot.system.cpuCores}` : emptyToken}
          />
          <InfoRow
            label={t("info.totalMemory")}
            value={`${formatBytes(snapshot?.system.totalMemoryBytes ?? null)} / ${formatBytes(snapshot?.system.usedMemoryBytes ?? null)}`}
          />
        </TerminalPanel>
      </section>

      <section className="grid grid-cols-1 gap-3 xl:grid-cols-[1.6fr_1fr]">
        <TerminalPanel title={t("panel.timeline.title")} subtitle={t("panel.timeline.subtitle")}>
          <MemorySparkline points={history} locale={locale} t={t} />
        </TerminalPanel>

        <TerminalPanel title={t("panel.modules.title")} subtitle={t("panel.modules.subtitle")}>
          <div className="space-y-2">
            {moduleStatus.map((item) => (
              <article
                key={item.name}
                className="flex items-start justify-between gap-3 rounded-lg border border-border-muted/70 bg-app/55 px-3 py-2"
              >
                <div>
                  <div className="text-sm font-medium text-text-primary">{item.name}</div>
                  <div className="mt-0.5 text-xs text-text-muted">{item.detail}</div>
                </div>
                <span className="rounded-full border border-accent/50 bg-accent/15 px-2 py-0.5 font-mono text-[11px] uppercase text-accent">
                  {item.state}
                </span>
              </article>
            ))}
          </div>
        </TerminalPanel>
      </section>
    </div>
  );
}

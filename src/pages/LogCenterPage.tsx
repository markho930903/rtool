import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { LoadingIndicator } from "@/components/loading";
import { Button, Input, Textarea } from "@/components/ui";
import { useLoggingStore } from "@/stores/logging.store";
import type { LogLevel } from "@/services/logging.service";

const LEVEL_OPTIONS: Array<{ value: LogLevel; label: string }> = [
  { value: "error", label: "Error" },
  { value: "warn", label: "Warn" },
  { value: "info", label: "Info" },
  { value: "debug", label: "Debug" },
  { value: "trace", label: "Trace" },
];

function toDateTimeLocalValue(timestamp: number | null): string {
  if (!timestamp) {
    return "";
  }

  const date = new Date(timestamp);
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  const hour = `${date.getHours()}`.padStart(2, "0");
  const minute = `${date.getMinutes()}`.padStart(2, "0");
  return `${year}-${month}-${day}T${hour}:${minute}`;
}

function parseDateTimeLocalValue(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const parsed = new Date(trimmed).getTime();
  if (!Number.isFinite(parsed)) {
    return null;
  }
  return parsed;
}

function formatTimestamp(value: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(value));
}

function levelClassName(level: LogLevel): string {
  if (level === "error") {
    return "bg-danger/15 text-danger border-danger/30";
  }
  if (level === "warn") {
    return "bg-accent/15 text-accent border-accent/35";
  }
  if (level === "info") {
    return "bg-accent/15 text-accent border-accent/35";
  }
  if (level === "debug") {
    return "bg-surface-soft text-text-secondary border-border-muted";
  }
  return "bg-surface-soft text-text-muted border-border-muted";
}

export default function LogCenterPage() {
  const { t, i18n } = useTranslation("logs");
  const locale = i18n.resolvedLanguage ?? i18n.language;

  const items = useLoggingStore((state) => state.items);
  const nextCursor = useLoggingStore((state) => state.nextCursor);
  const loading = useLoggingStore((state) => state.loading);
  const loadingMore = useLoggingStore((state) => state.loadingMore);
  const exporting = useLoggingStore((state) => state.exporting);
  const streamConnected = useLoggingStore((state) => state.streamConnected);
  const error = useLoggingStore((state) => state.error);
  const selectedLogId = useLoggingStore((state) => state.selectedLogId);
  const config = useLoggingStore((state) => state.config);
  const filters = useLoggingStore((state) => state.filters);
  const lastExportPath = useLoggingStore((state) => state.lastExportPath);
  const fetchConfig = useLoggingStore((state) => state.fetchConfig);
  const setFilters = useLoggingStore((state) => state.setFilters);
  const resetFilters = useLoggingStore((state) => state.resetFilters);
  const refresh = useLoggingStore((state) => state.refresh);
  const loadMore = useLoggingStore((state) => state.loadMore);
  const startStream = useLoggingStore((state) => state.startStream);
  const stopStream = useLoggingStore((state) => state.stopStream);
  const selectLog = useLoggingStore((state) => state.selectLog);
  const exportCurrentQuery = useLoggingStore((state) => state.exportCurrentQuery);
  const [filtersCollapsed, setFiltersCollapsed] = useState(true);

  useEffect(() => {
    void fetchConfig();
    void refresh();
    void startStream();

    return () => {
      stopStream();
    };
  }, [fetchConfig, refresh, startStream, stopStream]);

  const selectedLog = useMemo(() => {
    if (selectedLogId === null) {
      return items[0] ?? null;
    }
    return items.find((item) => item.id === selectedLogId) ?? items[0] ?? null;
  }, [items, selectedLogId]);

  const toggleLevel = (level: LogLevel, checked: boolean) => {
    const nextLevels = checked
      ? [...new Set([...filters.levels, level])]
      : filters.levels.filter((item) => item !== level);
    setFilters({ levels: nextLevels });
  };

  const handleExport = async () => {
    await exportCurrentQuery();
  };

  return (
    <div className="space-y-3 pb-2">
      <section className="rounded-2xl border border-border-strong bg-surface p-4">
        <div className="font-mono ui-text-micro uppercase tracking-ui-wider text-text-muted">
          rtool / log center / realtime
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <h1 className="m-0 text-xl font-semibold tracking-tight text-text-primary">{t("header.title")}</h1>
          <span
            className={`rounded-full border px-2 py-0.5 font-mono ui-text-micro uppercase tracking-ui-wide ${
              streamConnected ? "border-accent/35 bg-accent/10 text-accent" : "border-border-muted text-text-muted"
            }`}
          >
            {streamConnected ? t("stream.on") : t("stream.off")}
          </span>
          <span className="rounded-full border border-border-muted bg-app px-2 py-0.5 font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">
            {t("minLevel", { level: config?.minLevel ?? t("common:status.empty") })}
          </span>
        </div>
        <p className="mt-2 max-w-4xl text-sm text-text-secondary">{t("header.subtitle")}</p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Button size="default" variant="secondary" onClick={() => void refresh()} disabled={loading}>
            {t("action.refresh")}
          </Button>
          <Button size="default" variant="secondary" onClick={() => void handleExport()} disabled={exporting}>
            {exporting ? t("action.exporting") : t("action.export")}
          </Button>
          <Button
            size="default"
            variant="ghost"
            onClick={() => {
              resetFilters();
              void refresh();
            }}
          >
            {t("action.resetFilters")}
          </Button>
          <Button as="link" to="/settings" variant="primary">
            {t("action.openSettings")}
          </Button>
        </div>
        {lastExportPath ? (
          <div className="mt-2 text-xs text-text-muted">
            {t("lastExport")}
            <span className="font-mono text-text-secondary">{lastExportPath}</span>
          </div>
        ) : null}
        {error ? (
          <div className="mt-2 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
            {error}
          </div>
        ) : null}
      </section>

      <section className="space-y-3">
        <aside className="space-y-3 rounded-xl border border-border-muted bg-surface p-4">
          <div className="flex items-start justify-between gap-3">
            <div>
              <h2 className="m-0 text-sm font-semibold text-text-primary">{t("filters.title")}</h2>
              <p className="mt-1 text-xs text-text-muted">
                {filtersCollapsed ? t("filters.collapsedHint") : t("filters.expandedHint")}
              </p>
            </div>
            <Button
              size="default"
              variant="ghost"
              onClick={() => setFiltersCollapsed((value) => !value)}
              className="shrink-0"
            >
              <span
                className={`btn-icon ${filtersCollapsed ? "i-noto:down-arrow" : "i-noto:up-arrow"}`}
                aria-hidden="true"
              />
              <span>{filtersCollapsed ? t("filters.expand") : t("filters.collapse")}</span>
            </Button>
          </div>

          <div className="grid grid-cols-1 gap-2 rounded-lg border border-border-muted/70 bg-app/40 px-3 py-2 sm:grid-cols-[88px_minmax(0,1fr)] sm:items-center sm:gap-3">
            <div className="text-xs text-text-muted">{t("filters.level")}</div>
            <div className="overflow-x-auto">
              <div className="flex min-w-max items-center gap-2 sm:justify-end">
                {LEVEL_OPTIONS.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    aria-pressed={filters.levels.includes(option.value)}
                    className={`inline-flex items-center rounded-full border px-2.5 py-1 text-xs transition-colors ${
                      filters.levels.includes(option.value)
                        ? "border-accent/50 bg-accent/15 text-accent"
                        : "border-border-muted bg-surface text-text-secondary hover:border-border-strong hover:bg-surface-soft"
                    }`}
                    onClick={() => toggleLevel(option.value, !filters.levels.includes(option.value))}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>
          </div>

          {!filtersCollapsed ? (
            <>
              <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-6">
                <div className="space-y-1">
                  <div className="text-xs text-text-muted">{t("filters.scope")}</div>
                  <Input
                    value={filters.scope}
                    onChange={(event) => setFilters({ scope: event.currentTarget.value })}
                    placeholder={t("filters.scopePlaceholder")}
                  />
                </div>

                <div className="space-y-1">
                  <div className="text-xs text-text-muted">{t("filters.requestId")}</div>
                  <Input
                    value={filters.requestId}
                    onChange={(event) => setFilters({ requestId: event.currentTarget.value })}
                    placeholder={t("filters.requestIdPlaceholder")}
                  />
                </div>

                <div className="space-y-1">
                  <div className="text-xs text-text-muted">{t("filters.windowLabel")}</div>
                  <Input
                    value={filters.windowLabel}
                    onChange={(event) => setFilters({ windowLabel: event.currentTarget.value })}
                    placeholder={t("filters.windowPlaceholder")}
                  />
                </div>

                <div className="space-y-1">
                  <div className="text-xs text-text-muted">{t("filters.keyword")}</div>
                  <Input
                    value={filters.keyword}
                    onChange={(event) => setFilters({ keyword: event.currentTarget.value })}
                    placeholder={t("filters.keywordPlaceholder")}
                  />
                </div>

                <div className="space-y-1">
                  <div className="text-xs text-text-muted">{t("filters.startAt")}</div>
                  <Input
                    type="datetime-local"
                    value={toDateTimeLocalValue(filters.startAt)}
                    onChange={(event) => setFilters({ startAt: parseDateTimeLocalValue(event.currentTarget.value) })}
                  />
                </div>

                <div className="space-y-1">
                  <div className="text-xs text-text-muted">{t("filters.endAt")}</div>
                  <Input
                    type="datetime-local"
                    value={toDateTimeLocalValue(filters.endAt)}
                    onChange={(event) => setFilters({ endAt: parseDateTimeLocalValue(event.currentTarget.value) })}
                  />
                </div>
              </div>
            </>
          ) : null}

          <div className="flex justify-end">
            <Button size="default" variant="primary" onClick={() => void refresh()}>
              {t("action.applyFilters")}
            </Button>
          </div>
        </aside>

        <div className="grid grid-cols-1 gap-3 xl:grid-cols-[minmax(0,1fr)_360px]">
          <section className="min-h-[540px] rounded-xl border border-border-muted bg-surface p-3">
            <div className="mb-2 flex items-center justify-between">
              <h2 className="m-0 text-sm font-semibold text-text-primary">{t("list.title")}</h2>
              <div className="text-xs text-text-muted">{t("list.count", { count: items.length })}</div>
            </div>

            <LoadingIndicator
              mode="overlay"
              loading={loading && items.length === 0}
              text={t("list.loading")}
              containerClassName="h-[500px] overflow-auto rounded-lg border border-border-muted/80 bg-app/55"
            >
              <>
                {!loading && items.length === 0 ? (
                  <div className="p-3 text-xs text-text-muted">{t("list.empty")}</div>
                ) : null}

                {items.map((item) => {
                  const selected = selectedLog?.id === item.id;
                  return (
                    <Button
                      unstyled
                      key={item.id}
                      type="button"
                      className={`w-full border-b border-border-muted/60 px-3 py-2 text-left last:border-b-0 ${
                        selected ? "bg-accent-soft" : "hover:bg-surface-soft"
                      }`}
                      onClick={() => selectLog(item.id)}
                    >
                      <div className="flex items-center gap-2">
                        <span
                          className={`rounded border px-1.5 py-0.5 font-mono ui-text-micro uppercase ${levelClassName(item.level)}`}
                        >
                          {item.level}
                        </span>
                        <span className="truncate font-mono ui-text-micro text-text-secondary">
                          {formatTimestamp(item.timestamp, locale)}
                        </span>
                        <span className="truncate text-xs text-text-primary">{item.scope}</span>
                      </div>
                      <div className="mt-1 truncate text-xs text-text-secondary">
                        {item.event} · {item.message}
                      </div>
                      <div className="mt-1 truncate font-mono ui-text-micro text-text-muted">{item.requestId}</div>
                    </Button>
                  );
                })}
              </>
            </LoadingIndicator>

            <div className="mt-2 flex items-center justify-between gap-2">
              <div className="text-xs text-text-muted">{nextCursor ? t("list.hasMore") : t("list.end")}</div>
              <Button
                size="default"
                variant="secondary"
                disabled={!nextCursor || loadingMore}
                onClick={() => void loadMore()}
              >
                {loadingMore ? t("list.loadingMore") : t("list.loadMore")}
              </Button>
            </div>
          </section>

          <aside className="min-h-[540px] rounded-xl border border-border-muted bg-surface p-4">
            <div className="mb-2 flex items-center justify-between">
              <h2 className="m-0 text-sm font-semibold text-text-primary">{t("detail.title")}</h2>
              {selectedLog?.aggregatedCount ? (
                <span className="rounded border border-border-muted px-2 py-0.5 font-mono ui-text-micro text-text-muted">
                  {t("detail.aggregated", { count: selectedLog.aggregatedCount })}
                </span>
              ) : null}
            </div>

            {!selectedLog ? <div className="text-xs text-text-muted">{t("detail.selectPrompt")}</div> : null}

            {selectedLog ? (
              <div className="space-y-2">
                <div className="rounded-lg border border-border-muted bg-app/60 p-2 text-xs">
                  <div className="grid grid-cols-[96px_1fr] gap-1">
                    <span className="text-text-muted">{t("field.time")}</span>
                    <span className="font-mono text-text-secondary">
                      {formatTimestamp(selectedLog.timestamp, locale)}
                    </span>
                    <span className="text-text-muted">{t("field.level")}</span>
                    <span className="font-mono text-text-secondary">{selectedLog.level}</span>
                    <span className="text-text-muted">{t("field.scope")}</span>
                    <span className="font-mono text-text-secondary">{selectedLog.scope}</span>
                    <span className="text-text-muted">{t("field.event")}</span>
                    <span className="font-mono text-text-secondary">{selectedLog.event}</span>
                    <span className="text-text-muted">{t("field.requestId")}</span>
                    <span className="font-mono text-text-secondary break-all">{selectedLog.requestId}</span>
                    <span className="text-text-muted">{t("field.window")}</span>
                    <span className="font-mono text-text-secondary">
                      {selectedLog.windowLabel ?? t("common:status.empty")}
                    </span>
                    <span className="text-text-muted">{t("field.rawRef")}</span>
                    <span className="font-mono text-text-secondary break-all">
                      {selectedLog.rawRef ?? t("common:status.empty")}
                    </span>
                  </div>
                </div>

                <div>
                  <div className="mb-1 text-xs text-text-muted">{t("field.message")}</div>
                  <Textarea
                    value={selectedLog.message}
                    readOnly
                    resize="none"
                    className="min-h-[84px] font-mono text-xs"
                  />
                </div>

                <div>
                  <div className="mb-1 text-xs text-text-muted">{t("field.metadata")}</div>
                  <Textarea
                    value={selectedLog.metadata ? JSON.stringify(selectedLog.metadata, null, 2) : "{}"}
                    readOnly
                    resize="none"
                    className="min-h-[220px] font-mono text-xs"
                  />
                </div>
              </div>
            ) : null}
          </aside>
        </div>
      </section>
    </div>
  );
}

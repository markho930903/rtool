import { memo } from "react";

import type { AppManagerIndexState, ManagedApp } from "@/components/app-manager/types";
import { AppEntityIcon } from "@/components/icons/AppEntityIcon";
import { LoadingIndicator } from "@/components/loading";
import { Button, Input } from "@/components/ui";
import { formatBytes, toBreadcrumb } from "@/features/app-manager/format";

interface AppListPaneProps {
  items: ManagedApp[];
  selectedAppId: string | null;
  loading: boolean;
  loadingMore: boolean;
  hasMore: boolean;
  keyword: string;
  indexedAtText: string;
  indexState: AppManagerIndexState;
  totalCount: number;
  onKeywordChange: (value: string) => void;
  onSelect: (appId: string) => void;
  onRefresh: () => void;
  onLoadMore: () => void;
}

const AppListRow = memo(function AppListRow(props: {
  app: ManagedApp;
  selected: boolean;
  onSelect: (appId: string) => void;
}) {
  const { app, selected, onSelect } = props;
  const sizeText = app.sizeBytes === null ? "计算中..." : formatBytes(app.sizeBytes);

  return (
    <button
      type="button"
      className={`w-full rounded-xl border px-3 py-2.5 text-left transition-colors ${
        selected
          ? "border-accent/70 bg-accent/10"
          : "border-border-glass bg-surface-glass-soft shadow-inset-soft hover:border-accent/45"
      }`}
      onClick={() => onSelect(app.id)}
    >
      <div className="flex items-start gap-2.5">
        <AppEntityIcon
          iconKind={app.iconKind}
          iconValue={app.iconValue}
          fallbackIcon="i-noto:desktop-computer"
          imgClassName="h-9 w-9 shrink-0 rounded-md object-cover"
          iconClassName="h-9 w-9 shrink-0 text-[1.1rem] text-text-secondary"
        />
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-semibold text-text-primary">{app.name}</div>
          <div className="mt-0.5 truncate text-[11px] text-text-muted">{toBreadcrumb(app.path)}</div>
          <div className="mt-1.5 text-xs text-text-secondary">
            <span>{sizeText}</span>
          </div>
        </div>
      </div>
    </button>
  );
});

export function AppListPane(props: AppListPaneProps) {
  const {
    items,
    selectedAppId,
    loading,
    loadingMore,
    hasMore,
    keyword,
    indexedAtText,
    indexState,
    totalCount,
    onKeywordChange,
    onSelect,
    onRefresh,
    onLoadMore,
  } = props;

  return (
    <aside className="ui-glass-panel flex h-full min-h-0 flex-col">
      <div className="shrink-0 space-y-2.5 border-b border-border-glass px-3 py-3">
        <div className="flex items-start justify-between gap-2">
          <div className="space-y-0.5">
            <h1 className="m-0 text-base font-semibold text-text-primary">应用管理</h1>
            <p className="m-0 text-xs text-text-secondary">先加载列表，大小逐步精确补齐</p>
          </div>
          <Button
            size="xs"
            variant="secondary"
            disabled={loading}
            onClick={onRefresh}
            aria-label="刷新应用列表"
            title="刷新应用列表"
            className="px-2"
          >
            <span className={`i-lucide:refresh-cw h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} aria-hidden="true" />
          </Button>
        </div>
        <Input
          value={keyword}
          placeholder="搜索应用名、路径、发布者"
          onChange={(event) => onKeywordChange(event.currentTarget.value)}
        />
        <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-text-muted">
          <span>{`已索引: ${indexedAtText}`}</span>
          <span>{`显示 ${items.length}/${totalCount}`}</span>
          <span>{`状态: ${indexState}`}</span>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
        <LoadingIndicator
          mode="overlay"
          loading={loading && items.length === 0}
          text="加载应用列表中..."
          containerClassName="min-h-24"
        >
          <>
            {!loading && items.length === 0 ? (
              <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-6 text-center text-sm text-text-muted shadow-inset-soft">
                当前没有匹配的应用
              </div>
            ) : null}
            <div className="space-y-2">
              {items.map((app) => (
                <AppListRow key={app.id} app={app} selected={app.id === selectedAppId} onSelect={onSelect} />
              ))}
            </div>

            {hasMore ? (
              <div className="mt-3 flex justify-center">
                <Button size="default" variant="secondary" disabled={loadingMore} onClick={onLoadMore}>
                  {loadingMore ? "加载中..." : "加载更多"}
                </Button>
              </div>
            ) : null}
          </>
        </LoadingIndicator>
      </div>
    </aside>
  );
}

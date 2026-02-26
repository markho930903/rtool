import { memo } from "react";

import type {
  AppManagerCleanupResult,
  AppManagerResidueKind,
  AppManagerResidueScanResult,
  ManagedApp,
  ManagedAppDetail,
} from "@/components/app-manager/types";
import { AppEntityIcon } from "@/components/icons/AppEntityIcon";
import { resolvePathIcon, resolveResiduePathIcon } from "@/components/icons/pathIcon";
import { LoadingIndicator, SkeletonComposer, type SkeletonItemSpec } from "@/components/loading";
import { Button, RadioGroup, type RadioOption } from "@/components/ui";
import { DiskPlaceholder } from "@/features/app-manager/DiskPlaceholder";
import { formatBytes, getPathName, toBreadcrumb } from "@/features/app-manager/format";

interface AppDetailPaneProps {
  selectedApp: ManagedApp | null;
  coreDetail: ManagedAppDetail | null;
  heavyDetail: AppManagerResidueScanResult | null;
  coreLoading: boolean;
  heavyLoading: boolean;
  deepCompleting: boolean;
  detailError: string | null;
  selectedResidueIds: string[];
  selectedIncludeMain: boolean;
  selectedDeleteMode: "trash" | "permanent";
  cleanupLoading: boolean;
  cleanupResult: AppManagerCleanupResult | null;
  cleanupError: string | null;
  onToggleResidue: (itemId: string, checked: boolean) => void;
  onToggleIncludeMain: (checked: boolean) => void;
  onSetDeleteMode: (mode: "trash" | "permanent") => void;
  onCleanupNow: () => void;
  onRetryFailed: () => void;
  onRevealPath: (path: string) => void;
  onScanAgain: () => void;
}

function SelectionButton(props: { checked: boolean; disabled?: boolean; onClick: () => void }) {
  const { checked, disabled, onClick } = props;
  return (
    <button
      type="button"
      aria-pressed={checked}
      disabled={disabled}
      className={`mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full border transition-colors ${
        checked
          ? "border-accent bg-accent text-accent-foreground"
          : "border-border-glass bg-surface-glass-soft text-transparent hover:border-accent/55"
      } ${disabled ? "cursor-not-allowed opacity-55" : "cursor-pointer"}`}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
    >
      <svg viewBox="0 0 16 16" className="h-3 w-3" aria-hidden="true">
        <path
          d="m3.5 8.25 2.5 2.5L12.5 4.5"
          fill="none"
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="1.6"
        />
      </svg>
    </button>
  );
}

function PathTypeIcon(props: { path: string; pathType?: "file" | "directory"; residueKind?: AppManagerResidueKind }) {
  const iconClass = props.residueKind
    ? resolveResiduePathIcon(props.path, props.pathType, props.residueKind)
    : resolvePathIcon(props.path, props.pathType);
  return <span className={`btn-icon h-5 w-5 shrink-0 text-[1rem] text-text-muted ${iconClass}`} aria-hidden="true" />;
}

const APP_RESIDUE_SKELETON_ITEMS: SkeletonItemSpec[] = Array.from({ length: 5 }, (_, index) => ({
  key: `app-residue-skeleton-${index}`,
  leading: [
    {
      nodes: [
        {
          kind: "circle",
          className: "mt-0.5 shrink-0 border border-border-glass bg-surface-glass",
        },
      ],
    },
    {
      nodes: [
        {
          kind: "block",
          widthClassName: "w-5",
          heightClassName: "h-5",
          className: "mt-0.5 shrink-0 rounded bg-border-muted/60",
        },
      ],
    },
  ],
  body: [
    {
      nodes: [
        { widthClassName: "w-[44%]", heightClassName: "h-3.5", className: "bg-border-muted/70" },
        { widthClassName: "w-[74%]", offsetTopClassName: "mt-2", className: "bg-border-muted/55" },
        { widthClassName: "w-[36%]", className: "bg-border-muted/50" },
      ],
    },
  ],
  trailing: [
    {
      nodes: [
        {
          kind: "block",
          widthClassName: "w-12",
          heightClassName: "h-3.5",
          className: "mt-0.5 rounded bg-border-muted/65",
        },
      ],
    },
  ],
  shimmerDelayMs: index * 80,
}));

function AppDetailPaneImpl(props: AppDetailPaneProps) {
  const {
    selectedApp,
    coreDetail,
    heavyDetail,
    coreLoading,
    heavyLoading,
    deepCompleting,
    detailError,
    selectedResidueIds,
    selectedIncludeMain,
    selectedDeleteMode,
    cleanupLoading,
    cleanupResult,
    cleanupError,
    onToggleResidue,
    onToggleIncludeMain,
    onSetDeleteMode,
    onCleanupNow,
    onRetryFailed,
    onRevealPath,
    onScanAgain,
  } = props;

  if (!selectedApp) {
    return <DiskPlaceholder title="等待选择应用" desc="左侧选择一个应用后，右侧加载精确详情与清理项。" />;
  }

  const flatResidues = (heavyDetail?.groups ?? []).flatMap((group) =>
    group.items.map((item) => ({ ...item, groupLabel: group.label })),
  );
  const hasHeavyData = Boolean(heavyDetail);
  const isHeavyPending = !hasHeavyData && !detailError;
  const showResidueEmpty = hasHeavyData && flatResidues.length === 0;
  const showOverlayLoading = (coreLoading && Boolean(coreDetail)) || (heavyLoading && hasHeavyData);
  const selectedResidueCountText = isHeavyPending ? "--" : String(selectedResidueIds.length);
  const residueCountText = isHeavyPending ? "--" : String(flatResidues.length);
  const mainAppPath = coreDetail?.installPath ?? selectedApp.path;
  const revealPathButtonClass =
    "w-full cursor-pointer truncate text-left text-[11px] text-text-muted underline-offset-2 focus-visible:underline hover:underline disabled:cursor-not-allowed disabled:no-underline";
  const deleteModeOptions: RadioOption[] = [
    { value: "trash", label: "移入废纸篓" },
    { value: "permanent", label: "永久删除" },
  ];

  return (
    <section className="ui-glass-panel flex h-full min-h-0 flex-col px-4 py-4">
      <div className="shrink-0 space-y-3 border-b border-border-glass pb-3">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0 space-y-1">
            <div className="flex items-center gap-2">
              <AppEntityIcon
                iconKind={selectedApp.iconKind}
                iconValue={selectedApp.iconValue}
                fallbackIcon="i-noto:desktop-computer"
                imgClassName="h-9 w-9 shrink-0 rounded-md object-cover"
                iconClassName="h-9 w-9 shrink-0 text-[1.1rem] text-text-secondary"
              />
              <h2 className="m-0 truncate text-lg font-semibold text-text-primary">{selectedApp.name}</h2>
            </div>
            <p className="m-0 truncate text-xs text-text-muted">{toBreadcrumb(mainAppPath)}</p>
            <div className="flex min-w-0 items-center gap-2 overflow-hidden whitespace-nowrap text-xs text-text-secondary">
              <span className="min-w-0 max-w-[9rem] truncate">{`版本: ${selectedApp.version ?? "-"}`}</span>
              <span className="min-w-0 max-w-[12rem] truncate">{`发布者: ${selectedApp.publisher ?? "-"}`}</span>
              <span className="shrink-0">{`主程序: ${formatBytes(coreDetail?.sizeSummary.appBytes ?? selectedApp.sizeBytes)}`}</span>
            </div>
          </div>
          <Button size="sm" variant="secondary" disabled={heavyLoading && !deepCompleting} onClick={onScanAgain}>
            {heavyLoading ? "扫描中..." : "重新扫描"}
          </Button>
        </div>

        <div className="min-h-4 text-xs text-text-secondary" aria-live="polite">
          {deepCompleting ? "深度补全中，结果会自动更新" : "\u00a0"}
        </div>

        <div className="rounded-md border border-border-glass bg-surface-glass-soft px-2.5 py-2 shadow-inset-soft">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex shrink-0 items-center gap-1 text-[11px] text-text-secondary tabular-nums">
              <span className="inline-flex min-w-[84px] justify-center rounded-full border border-border-glass bg-surface-glass px-1.5 py-0.5">{`已选残留 ${selectedResidueCountText} 项`}</span>
              <span className="inline-flex min-w-[82px] justify-center rounded-full border border-border-glass bg-surface-glass px-1.5 py-0.5">{`可清理 ${residueCountText} 项`}</span>
            </div>

            <div className="flex items-center justify-end gap-1.5">
              <Button
                size="xs"
                variant="secondary"
                disabled={cleanupLoading || !cleanupResult || cleanupResult.failed.length === 0}
                onClick={onRetryFailed}
              >
                重试失败项
              </Button>
              <Button size="xs" variant="danger" disabled={cleanupLoading} onClick={onCleanupNow}>
                {cleanupLoading ? "清理中..." : "立即清理"}
              </Button>
            </div>
          </div>

          <div className="mt-2 border-t border-border-glass/70 pt-2">
            <div className="flex min-w-[176px] items-center gap-1.5 overflow-x-auto">
              <span className="shrink-0 text-[11px] text-text-secondary">清理方式</span>
              <RadioGroup
                name="app-manager-delete-mode"
                value={selectedDeleteMode}
                options={deleteModeOptions}
                orientation="horizontal"
                size="sm"
                variant="card"
                onValueChange={(value) => onSetDeleteMode(value as "trash" | "permanent")}
                className="w-full flex-nowrap items-stretch gap-1"
                optionClassName="w-fit min-h-8 shrink-0 items-center overflow-visible rounded-md border border-border-glass bg-surface-glass px-2 py-1 text-[11px] text-text-secondary shadow-inset-soft transition-colors duration-150 hover:border-border-glass-strong hover:bg-surface-glass"
              />
            </div>
          </div>
        </div>
      </div>

      {detailError ? (
        <div className="mt-3 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
          {detailError}
        </div>
      ) : null}
      {cleanupError ? (
        <div className="mt-3 rounded-md border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
          {cleanupError}
        </div>
      ) : null}

      <div className="mt-3 min-h-0 flex-1 overflow-y-auto">
        <LoadingIndicator
          mode="overlay"
          loading={showOverlayLoading}
          text={deepCompleting ? "正在深度补全结果..." : "正在加载精确详情..."}
          containerClassName="min-h-24"
          showMask={false}
        >
          <div className="space-y-2">
            <div
              className={`rounded-lg border px-3 py-2.5 shadow-inset-soft transition-colors ${
                selectedIncludeMain
                  ? "border-accent/55 bg-accent/10"
                  : "border-border-glass bg-surface-glass-soft hover:border-accent/35 hover:bg-surface-glass"
              } cursor-pointer`}
              onClick={() => onToggleIncludeMain(!selectedIncludeMain)}
            >
              <div className="flex items-start gap-2">
                <SelectionButton
                  checked={selectedIncludeMain}
                  onClick={() => onToggleIncludeMain(!selectedIncludeMain)}
                />
                <AppEntityIcon
                  iconKind={selectedApp.iconKind}
                  iconValue={selectedApp.iconValue}
                  fallbackIcon="i-noto:desktop-computer"
                  imgClassName="h-5 w-5 shrink-0 rounded-sm object-cover"
                  iconClassName="h-5 w-5 shrink-0 text-[1rem] text-text-muted"
                />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold text-text-primary">{selectedApp.name}</div>
                  <button
                    type="button"
                    className={revealPathButtonClass}
                    onClick={(event) => {
                      event.stopPropagation();
                      onRevealPath(mainAppPath);
                    }}
                  >
                    {toBreadcrumb(mainAppPath)}
                  </button>
                </div>
                <span className="shrink-0 text-sm text-text-primary">
                  {formatBytes(coreDetail?.sizeSummary.appBytes ?? selectedApp.sizeBytes)}
                </span>
              </div>
            </div>

            {isHeavyPending ? (
              <SkeletonComposer items={APP_RESIDUE_SKELETON_ITEMS} tone="glass" />
            ) : showResidueEmpty ? (
              <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-6 text-center text-sm text-text-muted shadow-inset-soft">
                当前没有可清理的相关项
              </div>
            ) : (
              <div className="space-y-2">
                {flatResidues.map((item) => {
                  const checked = selectedResidueIds.includes(item.itemId);
                  const disabled = item.readonly && item.readonlyReasonCode === "managed_by_policy";
                  return (
                    <div
                      key={item.itemId}
                      className={`rounded-lg border px-3 py-2.5 transition-colors ${
                        checked
                          ? "border-accent/55 bg-accent/10"
                          : "border-border-glass bg-surface-glass-soft hover:border-accent/35 hover:bg-surface-glass"
                      } ${disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer"}`}
                      onClick={() => {
                        if (disabled) {
                          return;
                        }
                        onToggleResidue(item.itemId, !checked);
                      }}
                    >
                      <div className="flex items-start gap-2">
                        <SelectionButton
                          checked={checked}
                          disabled={disabled}
                          onClick={() => onToggleResidue(item.itemId, !checked)}
                        />
                        <PathTypeIcon path={item.path} pathType={item.pathType} residueKind={item.kind} />
                        <div className="min-w-0 flex-1">
                          <div className="truncate text-sm font-medium text-text-primary">{getPathName(item.path)}</div>
                          <button
                            type="button"
                            disabled={disabled}
                            className={revealPathButtonClass}
                            onClick={(event) => {
                              event.stopPropagation();
                              onRevealPath(item.path);
                            }}
                          >
                            {toBreadcrumb(item.path)}
                          </button>
                          <div className="mt-1 text-[11px] text-text-secondary">{`${item.groupLabel} · ${item.kind}`}</div>
                        </div>
                        <span className="shrink-0 pt-0.5 text-sm text-text-primary">{formatBytes(item.sizeBytes)}</span>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </LoadingIndicator>
      </div>
    </section>
  );
}

export const AppDetailPane = memo(AppDetailPaneImpl);

import type {
  AppManagerCleanupResult,
  AppManagerResidueScanResult,
  ManagedApp,
  ManagedAppDetail,
} from "@/components/app-manager/types";
import { AppEntityIcon } from "@/components/icons/AppEntityIcon";
import { LoadingIndicator } from "@/components/loading";
import { Button, RadioGroup, type RadioOption, SwitchField } from "@/components/ui";
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
      onClick={onClick}
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

export function AppDetailPane(props: AppDetailPaneProps) {
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
  const selectedCount = selectedResidueIds.length + (selectedIncludeMain ? 1 : 0);
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
            <p className="m-0 break-all text-xs text-text-muted">
              {toBreadcrumb(coreDetail?.installPath ?? selectedApp.path)}
            </p>
            <div className="flex flex-wrap gap-2 text-xs text-text-secondary">
              <span>{`版本: ${selectedApp.version ?? "-"}`}</span>
              <span>{`发布者: ${selectedApp.publisher ?? "-"}`}</span>
              <span>{`主程序: ${formatBytes(coreDetail?.sizeSummary.appBytes ?? selectedApp.sizeBytes)}`}</span>
            </div>
          </div>
          <Button size="sm" variant="secondary" disabled={heavyLoading && !deepCompleting} onClick={onScanAgain}>
            {heavyLoading ? "扫描中..." : "重新扫描"}
          </Button>
        </div>
        {deepCompleting ? <div className="text-xs text-text-secondary">深度补全中，结果会自动更新</div> : null}

        <div className="grid gap-2 md:grid-cols-[280px_1fr]">
          <div className="grid gap-1.5">
            <div className="text-xs text-text-secondary">删除方式</div>
            <RadioGroup
              name="app-manager-delete-mode"
              value={selectedDeleteMode}
              options={deleteModeOptions}
              orientation="horizontal"
              size="md"
              variant="card"
              onValueChange={(value) => onSetDeleteMode(value as "trash" | "permanent")}
              className="w-full flex-nowrap items-stretch gap-2"
              optionClassName="w-fit min-h-10 shrink-0 items-center overflow-visible rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-2 text-xs text-text-secondary shadow-inset-soft transition-colors duration-150 hover:border-border-glass-strong hover:bg-surface-glass"
            />
          </div>
          <SwitchField
            checked={selectedIncludeMain}
            controlPosition="end"
            onChange={(event) => onToggleIncludeMain(event.currentTarget.checked)}
            label={<span className="text-sm text-text-primary">包含主应用卸载</span>}
            description={<span className="leading-5">开启后会触发主程序卸载，同时清理所选残留项。</span>}
          />
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
          loading={coreLoading || heavyLoading}
          text={deepCompleting ? "正在深度补全结果..." : "正在加载精确详情..."}
          containerClassName="min-h-24"
          showMask={false}
        >
          <div className="space-y-2">
            <div className="rounded-lg border border-border-glass bg-surface-glass-soft px-3 py-2.5 shadow-inset-soft">
              <div className="flex items-start gap-2">
                <SelectionButton
                  checked={selectedIncludeMain}
                  onClick={() => onToggleIncludeMain(!selectedIncludeMain)}
                />
                <button
                  type="button"
                  className="min-w-0 flex-1 text-left"
                  onClick={() => onRevealPath(coreDetail?.installPath ?? selectedApp.path)}
                >
                  <div className="truncate text-sm font-semibold text-text-primary">{selectedApp.name}</div>
                  <div className="truncate text-[11px] text-text-muted">
                    {toBreadcrumb(coreDetail?.installPath ?? selectedApp.path)}
                  </div>
                </button>
                <span className="shrink-0 text-sm text-text-primary">
                  {formatBytes(coreDetail?.sizeSummary.appBytes ?? selectedApp.sizeBytes)}
                </span>
              </div>
            </div>

            {flatResidues.length === 0 ? (
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
                      } ${disabled ? "opacity-60" : ""}`}
                    >
                      <div className="flex items-start gap-2">
                        <SelectionButton
                          checked={checked}
                          disabled={disabled}
                          onClick={() => onToggleResidue(item.itemId, !checked)}
                        />
                        <button
                          type="button"
                          disabled={disabled}
                          className="min-w-0 flex-1 text-left"
                          onClick={() => onRevealPath(item.path)}
                        >
                          <div className="truncate text-sm font-medium text-text-primary">{getPathName(item.path)}</div>
                          <div className="truncate text-[11px] text-text-muted">{toBreadcrumb(item.path)}</div>
                          <div className="mt-1 text-[11px] text-text-secondary">{`${item.groupLabel} · ${item.kind}`}</div>
                        </button>
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

      <div className="mt-3 shrink-0 space-y-2 border-t border-border-glass pt-3">
        <div className="flex items-center justify-between text-xs text-text-secondary">
          <span>{`已选 ${selectedCount} 项`}</span>
          <span>{`可清理 ${flatResidues.length} 项`}</span>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            size="sm"
            variant="secondary"
            disabled={cleanupLoading || !cleanupResult || cleanupResult.failed.length === 0}
            onClick={onRetryFailed}
          >
            重试失败项
          </Button>
          <Button size="sm" variant="danger" disabled={cleanupLoading} onClick={onCleanupNow}>
            {cleanupLoading ? "清理中..." : "立即清理"}
          </Button>
        </div>
        {cleanupResult ? (
          <div className="rounded-md border border-border-glass bg-surface-glass-soft px-3 py-2 text-xs text-text-secondary">
            {`已释放 ${formatBytes(cleanupResult.releasedSizeBytes)} · 删除 ${cleanupResult.deleted.length} · 跳过 ${cleanupResult.skipped.length} · 失败 ${cleanupResult.failed.length}`}
          </div>
        ) : null}
      </div>
    </section>
  );
}

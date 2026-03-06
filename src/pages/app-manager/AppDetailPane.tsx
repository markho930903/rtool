import { type ReactElement, memo } from "react";
import { useTranslation } from "react-i18next";

import type {
  AppManagerActionResult,
  AppManagerCleanupDeleteMode,
  AppManagerCleanupItemResult,
  AppManagerCleanupResult,
  AppManagerExportScanResult,
  AppManagerResidueKind,
  AppManagerScanWarning,
  AppManagerResidueScanResult,
  ManagedApp,
  ManagedAppDetail,
} from "@/components/app-manager/types";
import { AppEntityIcon } from "@/components/icons/AppEntityIcon";
import { resolvePathIcon, resolveResiduePathIcon } from "@/components/icons/pathIcon";
import { LoadingIndicator, SkeletonComposer, type SkeletonItemSpec } from "@/components/loading";
import { Button } from "@ui/button";
import { Message } from "@ui/message/Message";
import { RadioGroup, type RadioOption } from "@ui/radio";
import { DiskPlaceholder } from "@/pages/app-manager/DiskPlaceholder";
import { formatBytes, getPathName, toBreadcrumb } from "@/pages/app-manager/format";

type ResidueGroup = AppManagerResidueScanResult["groups"][number];
type ResidueItem = ResidueGroup["items"][number] & { groupLabel: string };
type CleanupSectionKey = "deleted" | "skipped" | "failed";

interface CleanupSectionMeta {
  key: CleanupSectionKey;
  titleKey: "result.deletedRows" | "result.skippedRows" | "result.failedRows";
  panelClassName: string;
  labelClassName: string;
}

interface CleanupSectionView {
  key: CleanupSectionKey;
  title: string;
  items: AppManagerCleanupItemResult[];
  panelClassName: string;
  labelClassName: string;
}

const CLEANUP_SECTION_META: CleanupSectionMeta[] = [
  {
    key: "deleted",
    titleKey: "result.deletedRows",
    panelClassName: "border-success/35 bg-success/10",
    labelClassName: "text-success",
  },
  {
    key: "skipped",
    titleKey: "result.skippedRows",
    panelClassName: "border-border-glass bg-surface-glass-soft",
    labelClassName: "text-text-secondary",
  },
  {
    key: "failed",
    titleKey: "result.failedRows",
    panelClassName: "border-danger/35 bg-danger/10",
    labelClassName: "text-danger",
  },
];

export interface AppDetailViewState {
  selectedApp: ManagedApp | null;
  coreDetail: ManagedAppDetail | null;
  heavyDetail: AppManagerResidueScanResult | null;
  coreLoading: boolean;
  heavyLoading: boolean;
  deepCompleting: boolean;
  detailError: string | null;
}

export interface AppCleanupViewState {
  selectedResidueIds: string[];
  selectedIncludeMain: boolean;
  selectedDeleteMode: AppManagerCleanupDeleteMode;
  cleanupLoading: boolean;
  cleanupResult: AppManagerCleanupResult | null;
  cleanupError: string | null;
}

export interface AppDetailOperationsState {
  startupLoading: boolean;
  uninstallLoading: boolean;
  openHelpLoading: boolean;
  openPermissionHelpLoading: boolean;
  exportLoading: boolean;
  openExportDirLoading: boolean;
  exportResult: AppManagerExportScanResult | null;
  exportError: string | null;
  actionResult: AppManagerActionResult | null;
  actionError: string | null;
}

export interface AppDetailActions {
  onToggleResidue: (itemId: string, checked: boolean) => void;
  onSelectAllResidues: (itemIds: string[]) => void;
  onToggleIncludeMain: (checked: boolean) => void;
  onSetDeleteMode: (mode: AppManagerCleanupDeleteMode) => void;
  onCleanupNow: () => void | Promise<void>;
  onRetryFailed: () => void | Promise<void>;
  onRevealPath: (path: string) => void;
  onScanAgain: () => void | Promise<void>;
  onToggleStartup: () => void | Promise<void>;
  onOpenUninstallHelp: () => void | Promise<void>;
  onOpenPermissionHelp: () => void | Promise<void>;
  onUninstall: () => void | Promise<void>;
  onExportScanResult: () => void | Promise<void>;
  onOpenExportDirectory: () => void | Promise<void>;
}

export interface AppDetailPaneModel {
  detail: AppDetailViewState;
  cleanup: AppCleanupViewState;
  operations: AppDetailOperationsState;
  actions: AppDetailActions;
}

interface AppDetailPaneProps {
  model: AppDetailPaneModel;
}

function isFileProviderPermissionWarning(warning: AppManagerScanWarning): boolean {
  if (warning.detailCode !== "permission_denied") {
    return false;
  }
  return warning.path?.toLowerCase().includes("/library/application support/fileprovider") ?? false;
}

interface SelectionButtonProps {
  checked: boolean;
  disabled?: boolean;
  onClick: () => void;
}

function SelectionButton(props: SelectionButtonProps): ReactElement {
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

interface PathTypeIconProps {
  path: string;
  pathType?: "file" | "directory";
  residueKind?: AppManagerResidueKind;
}

function PathTypeIcon(props: PathTypeIconProps): ReactElement {
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

const SELECTED_CARD_CLASS = "border-accent/55 bg-accent/10";
const UNSELECTED_CARD_CLASS = "border-border-glass bg-surface-glass-soft hover:border-accent/35 hover:bg-surface-glass";

function getSelectableCardClass(checked: boolean, disabled = false): string {
  if (checked) {
    return disabled ? `${SELECTED_CARD_CLASS} cursor-not-allowed opacity-60` : `${SELECTED_CARD_CLASS} cursor-pointer`;
  }
  return disabled
    ? `${UNSELECTED_CARD_CLASS} cursor-not-allowed opacity-60`
    : `${UNSELECTED_CARD_CLASS} cursor-pointer`;
}

interface ResidueCardProps {
  item: ResidueItem;
  checked: boolean;
  disabled: boolean;
  revealPathButtonClass: string;
  onToggleResidue: (itemId: string, checked: boolean) => void;
  onRevealPath: (path: string) => void;
}

function ResidueCard(props: ResidueCardProps): ReactElement {
  const { item, checked, disabled, revealPathButtonClass, onToggleResidue, onRevealPath } = props;
  const cardClassName = `rounded-lg border px-3 py-2.5 transition-colors ${getSelectableCardClass(checked, disabled)}`;

  const toggle = (): void => {
    onToggleResidue(item.itemId, !checked);
  };

  return (
    <div
      className={cardClassName}
      onClick={() => {
        if (disabled) {
          return;
        }
        toggle();
      }}
    >
      <div className="flex items-start gap-2">
        <SelectionButton checked={checked} disabled={disabled} onClick={toggle} />
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
}

function AppDetailPaneImpl(props: AppDetailPaneProps): ReactElement {
  const { t } = useTranslation("app_manager");
  const {
    detail: {
      selectedApp,
      coreDetail,
      heavyDetail,
      coreLoading,
      heavyLoading,
      deepCompleting,
      detailError,
    },
    cleanup: {
      selectedResidueIds,
      selectedIncludeMain,
      selectedDeleteMode,
      cleanupLoading,
      cleanupResult,
      cleanupError,
    },
    operations: {
      startupLoading,
      uninstallLoading,
      openHelpLoading,
      openPermissionHelpLoading,
      exportLoading,
      openExportDirLoading,
      exportResult,
      exportError,
      actionResult,
      actionError,
    },
    actions: {
      onToggleResidue,
      onSelectAllResidues,
      onToggleIncludeMain,
      onSetDeleteMode,
      onCleanupNow,
      onRetryFailed,
      onRevealPath,
      onScanAgain,
      onToggleStartup,
      onOpenUninstallHelp,
      onOpenPermissionHelp,
      onUninstall,
      onExportScanResult,
      onOpenExportDirectory,
    },
  } = props.model;

  if (!selectedApp) {
    return <DiskPlaceholder title={t("detail.emptyTitle")} desc={t("detail.empty")} />;
  }

  const flatResidues: ResidueItem[] = (heavyDetail?.groups ?? []).flatMap((group) =>
    group.items.map((item) => ({ ...item, groupLabel: group.label })),
  );
  const hasHeavyData = Boolean(heavyDetail);
  const isHeavyPending = !hasHeavyData && !detailError;
  const showResidueEmpty = hasHeavyData && flatResidues.length === 0;
  const showOverlayLoading = (coreLoading && Boolean(coreDetail)) || (heavyLoading && hasHeavyData);
  const selectedResidueCount = isHeavyPending ? 0 : selectedResidueIds.length;
  const residueCount = isHeavyPending ? 0 : flatResidues.length;
  const selectedResidueIdSet = new Set(selectedResidueIds);
  const selectableResidueIds = flatResidues
    .filter((item) => !(item.readonly && item.readonlyReasonCode === "managed_by_policy"))
    .map((item) => item.itemId);
  const allSelectableResiduesSelected =
    selectableResidueIds.length > 0 && selectableResidueIds.every((itemId) => selectedResidueIdSet.has(itemId));
  const mainAppPath = coreDetail?.installPath ?? selectedApp.path;
  const revealPathButtonClass =
    "w-full cursor-pointer truncate text-left text-[11px] text-text-muted underline-offset-2 focus-visible:underline hover:underline disabled:cursor-not-allowed disabled:no-underline";
  const deleteModeOptions: RadioOption[] = [
    { value: "trash", label: t("cleanup.deleteModeTrash") },
    { value: "permanent", label: t("cleanup.deleteModePermanent") },
  ];
  const includeMainCardClassName = `rounded-lg border px-3 py-2.5 shadow-inset-soft transition-colors ${getSelectableCardClass(selectedIncludeMain)}`;
  const cleanupErrorText =
    cleanupError === "app_manager_cleanup_selection_required" ? t("cleanup.selectOneRequired") : cleanupError;
  const exportErrorText = exportError === "app_manager_export_missing_result" ? t("cleanup.exportMissing") : exportError;
  const scanWarnings = heavyDetail?.warnings ?? [];
  const hasFileProviderPermissionWarning = scanWarnings.some(isFileProviderPermissionWarning);
  const cleanupSections: CleanupSectionView[] = [];
  if (cleanupResult) {
    for (const sectionMeta of CLEANUP_SECTION_META) {
      const items = cleanupResult[sectionMeta.key];
      if (items.length === 0) {
        continue;
      }
      cleanupSections.push({
        key: sectionMeta.key,
        title: t(sectionMeta.titleKey),
        items,
        panelClassName: sectionMeta.panelClassName,
        labelClassName: sectionMeta.labelClassName,
      });
    }
  }
  const resolveCleanupReasonText = (reasonCode: AppManagerCleanupItemResult["reasonCode"]): string =>
    t(`result.reason.${reasonCode}`, { defaultValue: t("result.reason.unknown") });
  const resolveWarningText = (warning: AppManagerScanWarning): string => {
    const path = warning.path ?? "-";
    return t(`cleanup.warning.${warning.code}`, {
      path,
      defaultValue: t("cleanup.warning.unknown", { path }),
    });
  };
  const resolveWarningDetailText = (warning: AppManagerScanWarning): string | null => {
    if (!warning.detailCode) {
      return null;
    }
    return t(`cleanup.warningDetail.${warning.detailCode}`, {
      defaultValue: t("cleanup.warningDetail.unknown"),
    });
  };

  const toggleIncludeMain = (): void => {
    onToggleIncludeMain(!selectedIncludeMain);
  };

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
              <span className="min-w-0 max-w-[9rem] truncate">
                {t("meta.version", { value: selectedApp.version ?? "-" })}
              </span>
              <span className="min-w-0 max-w-[12rem] truncate">
                {t("meta.publisher", { value: selectedApp.publisher ?? "-" })}
              </span>
              <span className="shrink-0">
                {t("detail.mainProgramSize", { value: formatBytes(coreDetail?.sizeSummary.appBytes ?? selectedApp.sizeBytes) })}
              </span>
            </div>
          </div>
          <Button size="sm" variant="secondary" disabled={heavyLoading && !deepCompleting} onClick={() => void onScanAgain()}>
            {heavyLoading ? t("cleanup.scanning") : t("cleanup.scan")}
          </Button>
        </div>

        <div className="min-h-4 text-xs text-text-secondary" aria-live="polite">
          {deepCompleting ? t("detail.deepCompleting") : "\u00a0"}
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="xs"
            variant="secondary"
            disabled={startupLoading || !selectedApp.capabilities.startup || !selectedApp.startupEditable}
            onClick={() => void onToggleStartup()}
          >
            {startupLoading
              ? t("status.processing")
              : selectedApp.startupEnabled
                ? t("actions.disableStartup")
                : t("actions.enableStartup")}
          </Button>
          <Button
            size="xs"
            variant="secondary"
            disabled={openHelpLoading || !selectedApp.capabilities.uninstall}
            onClick={() => void onOpenUninstallHelp()}
          >
            {openHelpLoading ? t("status.processing") : t("actions.uninstallGuide")}
          </Button>
          <Button
            size="xs"
            variant="danger"
            disabled={uninstallLoading || !selectedApp.capabilities.uninstall || !selectedApp.uninstallSupported}
            onClick={() => {
              const confirmed = window.confirm(
                `${t("uninstallDialog.title")}\n${t("uninstallDialog.appName", { value: selectedApp.name })}\n${t("uninstallDialog.appPath", {
                  value: selectedApp.path,
                })}`,
              );
              if (confirmed) {
                void onUninstall();
              }
            }}
          >
            {uninstallLoading ? t("status.processing") : t("actions.uninstall")}
          </Button>
          <Button size="xs" variant="secondary" disabled={exportLoading} onClick={() => void onExportScanResult()}>
            {exportLoading ? t("cleanup.exporting") : t("cleanup.exportScan")}
          </Button>
          <Button
            size="xs"
            variant="secondary"
            disabled={!exportResult || openExportDirLoading}
            onClick={() => void onOpenExportDirectory()}
          >
            {openExportDirLoading ? t("cleanup.openingDir") : t("cleanup.openExportDir")}
          </Button>
        </div>

        <div className="rounded-md border border-border-glass bg-surface-glass-soft px-2.5 py-2 shadow-inset-soft">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex shrink-0 items-center gap-1 text-[11px] text-text-secondary tabular-nums">
              <span className="inline-flex min-w-[84px] justify-center rounded-full border border-border-glass bg-surface-glass px-1.5 py-0.5">
                {t("cleanup.selectedCount", { count: selectedResidueCount })}
              </span>
              <span className="inline-flex min-w-[82px] justify-center rounded-full border border-border-glass bg-surface-glass px-1.5 py-0.5">
                {t("cleanup.cleanableCount", { count: residueCount })}
              </span>
            </div>

            <div className="flex items-center justify-end gap-1.5">
              <Button
                size="xs"
                variant="secondary"
                disabled={cleanupLoading || selectableResidueIds.length === 0}
                onClick={() => onSelectAllResidues(allSelectableResiduesSelected ? [] : selectableResidueIds)}
              >
                {allSelectableResiduesSelected ? t("cleanup.clearSelection") : t("cleanup.selectAll")}
              </Button>
              <Button
                size="xs"
                variant="secondary"
                disabled={cleanupLoading || !cleanupResult || cleanupResult.failed.length === 0}
                onClick={() => void onRetryFailed()}
              >
                {t("result.retryFailed")}
              </Button>
              <Button size="xs" variant="danger" disabled={cleanupLoading} onClick={() => void onCleanupNow()}>
                {cleanupLoading ? t("cleanup.cleaning") : t("cleanup.cleanNow")}
              </Button>
            </div>
          </div>

          <div className="mt-2 border-t border-border-glass/70 pt-2">
            <div className="flex min-w-[176px] items-center gap-1.5 overflow-x-auto">
              <span className="shrink-0 text-[11px] text-text-secondary">{t("cleanup.deleteModeTitle")}</span>
              <RadioGroup
                name="app-manager-delete-mode"
                value={selectedDeleteMode}
                options={deleteModeOptions}
                orientation="horizontal"
                size="sm"
                variant="card"
                onValueChange={(value) => onSetDeleteMode(value as AppManagerCleanupDeleteMode)}
                className="w-full flex-nowrap items-stretch gap-1"
                optionClassName="w-fit min-h-8 shrink-0 items-center overflow-visible rounded-md border border-border-glass bg-surface-glass px-2 py-1 text-[11px] text-text-secondary shadow-inset-soft transition-colors duration-150 hover:border-border-glass-strong hover:bg-surface-glass"
              />
            </div>
          </div>
        </div>
      </div>

      {detailError ? <Message className="mt-3" type="error" description={detailError} /> : null}
      {cleanupErrorText ? <Message className="mt-3" type="error" description={cleanupErrorText} /> : null}
      {actionError ? <Message className="mt-3" type="error" description={actionError} /> : null}
      {exportErrorText ? <Message className="mt-3" type="error" description={exportErrorText} /> : null}
      {actionResult ? (
        <Message
          className="mt-3"
          type="info"
          title={actionResult.message}
          description={actionResult.detail ? <span className="break-all">{actionResult.detail}</span> : undefined}
        />
      ) : null}
      {exportResult ? (
        <Message className="mt-3" type="info" description={t("cleanup.exportedPath", { value: exportResult.filePath })} />
      ) : null}
      {scanWarnings.length > 0 ? (
        <Message className="mt-3" type="warning" title={t("cleanup.warningTitle", { count: scanWarnings.length })}>
          <div className="space-y-1.5 text-xs text-warning">
            {scanWarnings.map((warning, index) => {
              const detailText = resolveWarningDetailText(warning);
              return (
                <div key={`${warning.code}-${warning.path ?? "pathless"}-${warning.detailCode ?? "none"}-${index}`}>
                  <div>{resolveWarningText(warning)}</div>
                  {detailText ? <div className="text-[11px] text-text-secondary">{detailText}</div> : null}
                </div>
              );
            })}
          </div>
          {hasFileProviderPermissionWarning ? (
            <div className="mt-2 rounded border border-warning/40 bg-warning/5 px-2 py-2">
              <div className="text-[11px] text-text-secondary">{t("cleanup.permissionHelp.hint")}</div>
              <div className="mt-2 flex flex-wrap items-center gap-2">
                <Button
                  size="xs"
                  variant="secondary"
                  disabled={openPermissionHelpLoading}
                  onClick={() => void onOpenPermissionHelp()}
                >
                  {openPermissionHelpLoading ? t("status.processing") : t("actions.permissionGuide")}
                </Button>
                <span className="text-[11px] text-text-secondary">{t("cleanup.permissionHelp.manualRescan")}</span>
              </div>
            </div>
          ) : null}
        </Message>
      ) : null}
      {cleanupResult ? (
        <div className="mt-3 rounded-md border border-border-glass bg-surface-glass-soft px-3 py-2 text-xs text-text-secondary">
          <div className="text-sm font-semibold text-text-primary">{t("result.title")}</div>
          <div className="mt-1 flex flex-wrap gap-2 text-[11px] tabular-nums">
            <span className="rounded-full border border-border-glass bg-surface-glass px-2 py-0.5">
              {t("result.released", { value: formatBytes(cleanupResult.releasedSizeBytes) })}
            </span>
            <span className="rounded-full border border-success/35 bg-success/10 px-2 py-0.5 text-success">
              {t("result.deleted", { count: cleanupResult.deleted.length })}
            </span>
            <span className="rounded-full border border-border-glass bg-surface-glass px-2 py-0.5">
              {t("result.skipped", { count: cleanupResult.skipped.length })}
            </span>
            <span className="rounded-full border border-danger/35 bg-danger/10 px-2 py-0.5 text-danger">
              {t("result.failed", { count: cleanupResult.failed.length })}
            </span>
          </div>

          <div className="mt-2 space-y-2">
            {cleanupSections.map((section) => (
              <div key={section.key} className={`rounded-md border px-2.5 py-2 ${section.panelClassName}`}>
                <div className={`text-[11px] font-medium ${section.labelClassName}`}>{section.title}</div>
                <div className="mt-1 space-y-1.5">
                  {section.items.map((item) => (
                    <div key={`${section.key}-${item.itemId}-${item.path}`} className="rounded border border-border-glass/70 bg-surface-glass/80 px-2 py-1.5">
                      <div className="truncate text-[11px] font-medium text-text-primary">{toBreadcrumb(item.path)}</div>
                      <div className="mt-0.5 text-[11px] text-text-secondary">
                        {resolveCleanupReasonText(item.reasonCode)}
                        {item.message ? ` · ${item.message}` : ""}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : null}

      <div className="mt-3 min-h-0 flex-1 overflow-y-auto">
        <LoadingIndicator
          mode="overlay"
          loading={showOverlayLoading}
          text={deepCompleting ? t("detail.deepCompletingLoading") : t("detail.loading")}
          containerClassName="min-h-24"
          showMask={false}
        >
          <div className="space-y-2">
            <div className={includeMainCardClassName} onClick={toggleIncludeMain}>
              <div className="flex items-start gap-2">
                <SelectionButton checked={selectedIncludeMain} onClick={toggleIncludeMain} />
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
                {t("cleanup.empty")}
              </div>
            ) : (
              <div className="space-y-2">
                {flatResidues.map((item) => {
                  const checked = selectedResidueIdSet.has(item.itemId);
                  const disabled = item.readonly && item.readonlyReasonCode === "managed_by_policy";
                  return (
                    <ResidueCard
                      key={item.itemId}
                      item={item}
                      checked={checked}
                      disabled={disabled}
                      revealPathButtonClass={revealPathButtonClass}
                      onToggleResidue={onToggleResidue}
                      onRevealPath={onRevealPath}
                    />
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

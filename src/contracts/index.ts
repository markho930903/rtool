/* eslint-disable */
// 前后端合同类型单一事实源。
// 该文件由 `pnpm contracts:generate` 全量生成，请勿手工修改。

// <generated-models:start>
export type JsonValue = number | string | boolean | Array<JsonValue> | { [key in string]?: JsonValue } | null;

export type ErrorContextItem = {
  key: string;
  value: string;
};

export type AppErrorPayload = {
  code: string;
  message: string;
  context: Array<ErrorContextItem>;
  causes: Array<string>;
  requestId: string | null;
};

export type InvokeError = {
  code: string;
  message: string;
  context: Array<ErrorContextItem>;
  causes: Array<string>;
  requestId: string | null;
};

export type ActionResultDto = {
  ok: boolean;
  message: string;
};

export type SettingsDto = {
  theme: ThemeSettingsDto;
  layout: LayoutSettingsDto;
  locale: LocaleSettingsDto;
  clipboard: SettingsClipboardDto;
  screenshot: SettingsScreenshotDto;
};

export type ThemeSettingsDto = {
  preference: string;
  transparentWindowBackground: boolean;
};

export type LayoutSettingsDto = {
  preference: string;
};

export type LocaleSettingsDto = {
  preference: string;
};

export type SettingsUpdateInputDto = {
  theme: ThemeSettingsUpdateInputDto | null;
  layout: LayoutSettingsUpdateInputDto | null;
  locale: LocaleSettingsUpdateInputDto | null;
  clipboard: SettingsClipboardUpdateInputDto | null;
  screenshot: SettingsScreenshotUpdateInputDto | null;
};

export type ThemeSettingsUpdateInputDto = {
  preference: string | null;
  transparentWindowBackground: boolean | null;
};

export type LayoutSettingsUpdateInputDto = {
  preference: string | null;
};

export type LocaleSettingsUpdateInputDto = {
  preference: string | null;
};

export type SettingsClipboardDto = {
  maxItems: number;
  sizeCleanupEnabled: boolean;
  maxTotalSizeMb: number;
};

export type SettingsClipboardUpdateInputDto = {
  maxItems: number | null;
  sizeCleanupEnabled: boolean | null;
  maxTotalSizeMb: number | null;
};

export type SettingsScreenshotDto = {
  shortcut: string;
  autoSaveEnabled: boolean;
  maxItems: number;
  maxTotalSizeMb: number;
  pinMaxInstances: number;
};

export type SettingsScreenshotUpdateInputDto = {
  shortcut: string | null;
  autoSaveEnabled: boolean | null;
  maxItems: number | null;
  maxTotalSizeMb: number | null;
  pinMaxInstances: number | null;
};

export type LauncherActionDto =
  | { kind: "open_builtin_route"; route: string }
  | { kind: "open_builtin_tool"; toolId: string }
  | { kind: "open_builtin_window"; windowLabel: string }
  | { kind: "open_directory"; path: string }
  | { kind: "open_file"; path: string }
  | { kind: "open_application"; path: string };

export type LauncherItemDto = {
  id: string;
  title: string;
  subtitle: string;
  category: string;
  group: string;
  source: string | null;
  shortcut: string | null;
  score: number;
  iconKind: string;
  iconValue: string;
  action: LauncherActionDto;
};

export type LauncherSearchSettingsDto = {
  roots: Array<string>;
  excludePatterns: Array<string>;
  maxScanDepth: number;
  maxItemsPerRoot: number;
  maxTotalItems: number;
  refreshIntervalSecs: number;
};

export type LauncherSearchDiagnosticsDto = {
  indexUsed: boolean;
  fallbackToLike: boolean;
  queryDurationMs: number | null;
};

export type LauncherSearchIndexStateDto = {
  ready: boolean;
  building: boolean;
  indexedItems: number;
  truncated: boolean;
  lastBuildMs: number | null;
  lastError: string | null;
};

export type LauncherSearchResponseDto = {
  query: string;
  limit: number;
  items: Array<LauncherItemDto>;
  index: LauncherSearchIndexStateDto;
  diagnostics: LauncherSearchDiagnosticsDto;
};

export type LauncherUpdateSearchSettingsInputDto = {
  roots: Array<string> | null;
  excludePatterns: Array<string> | null;
  maxScanDepth: number | null;
  maxItemsPerRoot: number | null;
  maxTotalItems: number | null;
  refreshIntervalSecs: number | null;
};

export type LauncherIndexStatusDto = {
  ready: boolean;
  building: boolean;
  indexedItems: number;
  indexedRoots: number;
  lastBuildMs: number | null;
  lastDurationMs: number | null;
  lastError: string | null;
  refreshIntervalSecs: number;
  indexVersion: string;
  truncated: boolean;
};

export type LauncherRebuildResultDto = {
  success: boolean;
  durationMs: number;
  indexedItems: number;
  indexedRoots: number;
  truncated: boolean;
  ready: boolean;
};

export type LauncherStatusDto = {
  runtime: LauncherRuntimeStatusDto;
  index: LauncherIndexStatusDto;
  settings: LauncherSearchSettingsDto;
};

export type AppManagerQueryDto = {
  keyword: string | null;
  category: AppManagerCategory;
  limit: number | null;
  cursor: string | null;
};

export type AppManagerSnapshotMetaDto = {
  indexedAt: number;
  revision: number;
  totalCount: number;
  indexState: AppManagerIndexState;
};

export type AppManagerCapabilitiesDto = {
  startup: boolean;
  uninstall: boolean;
  residueScan: boolean;
};

export type AppManagerIdentityDto = {
  primaryId: string;
  aliases: Array<string>;
  identitySource: AppManagerIdentitySource;
};

export type AppReadonlyReasonCode =
  | "permission_denied"
  | "managed_by_policy"
  | "feature_disabled";

export type AppManagerScope =
  | "user"
  | "system";

export type AppManagerPathType =
  | "file"
  | "directory";

export type AppManagerStartupScope =
  | "user"
  | "system"
  | "none";

export type AppManagerResidueKind =
  | "install"
  | "app_support"
  | "cache"
  | "preferences"
  | "logs"
  | "startup"
  | "app_script"
  | "container"
  | "group_container"
  | "saved_state"
  | "webkit_data"
  | "launch_agent"
  | "launch_daemon"
  | "helper_tool"
  | "app_data"
  | "registry_key"
  | "registry_value"
  | "main_app";

export type AppManagerResidueConfidence =
  | "exact"
  | "high"
  | "medium";

export type AppManagerRiskLevel =
  | "low"
  | "medium"
  | "high";

export type AppManagerIdentitySource =
  | "bundle_id"
  | "registry"
  | "path";

export type AppManagerSource =
  | "rtool"
  | "application";

export type AppManagerSizeAccuracy =
  | "exact"
  | "estimated";

export type AppManagerSizeSource =
  | "app_bundle"
  | "parent_directory"
  | "path"
  | "registry_estimated";

export type AppManagerIndexState =
  | "ready"
  | "building"
  | "degraded";

export type AppManagerIndexUpdateReason =
  | "manual"
  | "auto_change"
  | "startup";

export type AppManagerCategory =
  | "all"
  | "rtool"
  | "application"
  | "startup";

export type AppManagerPlatform =
  | "macos"
  | "windows"
  | "linux";

export type AppManagerIconKind =
  | "raster"
  | "iconify";

export type AppManagerUninstallKind =
  | "finder_trash"
  | "registry_command";

export type AppManagerResidueMatchReason =
  | "related_root"
  | "bundle_id"
  | "extension_bundle"
  | "entitlement_group"
  | "identifier_pattern"
  | "keyword_token"
  | "startup_label"
  | "startup_shortcut"
  | "uninstall_registry"
  | "startup_registry"
  | "run_registry";

export type ManagedAppDto = {
  id: string;
  name: string;
  path: string;
  bundleOrAppId: string | null;
  version: string | null;
  publisher: string | null;
  platform: AppManagerPlatform;
  source: AppManagerSource;
  iconKind: AppManagerIconKind;
  iconValue: string;
  sizeBytes: number | null;
  sizeAccuracy: AppManagerSizeAccuracy;
  sizeSource: AppManagerSizeSource;
  sizeComputedAt: number | null;
  startupEnabled: boolean;
  startupScope: AppManagerStartupScope;
  startupEditable: boolean;
  readonlyReasonCode: AppReadonlyReasonCode | null;
  uninstallSupported: boolean;
  uninstallKind: AppManagerUninstallKind | null;
  capabilities: AppManagerCapabilitiesDto;
  identity: AppManagerIdentityDto;
  riskLevel: AppManagerRiskLevel;
  fingerprint: string;
};

export type AppManagerPageDto = {
  items: Array<ManagedAppDto>;
  nextCursor: string | null;
  totalCount: number;
  indexedAt: number;
  revision: number;
  indexState: AppManagerIndexState;
};

export type AppManagerIndexUpdatedPayloadDto = {
  revision: number;
  indexedAt: number;
  changedCount: number;
  reason: AppManagerIndexUpdateReason;
};

export type AppManagerStartupUpdateInputDto = {
  appId: string;
  enabled: boolean;
};

export type AppManagerUninstallInputDto = {
  appId: string;
  confirmedFingerprint: string;
};

export type AppManagerDetailQueryDto = {
  appId: string;
};

export type AppRelatedRootDto = {
  id: string;
  label: string;
  path: string;
  pathType: AppManagerPathType;
  scope: AppManagerScope;
  kind: AppManagerResidueKind;
  exists: boolean;
  readonly: boolean;
  readonlyReasonCode: AppReadonlyReasonCode | null;
};

export type AppSizeSummaryDto = {
  appBytes: number | null;
  residueBytes: number | null;
  totalBytes: number | null;
};

export type ManagedAppDetailDto = {
  app: ManagedAppDto;
  installPath: string;
  relatedRoots: Array<AppRelatedRootDto>;
  sizeSummary: AppSizeSummaryDto;
};

export type AppManagerResidueScanInputDto = {
  appId: string;
  mode: AppManagerResidueScanMode | null;
};

export type AppManagerResidueScanMode =
  | "quick"
  | "deep";

export type AppManagerResolveSizesInputDto = {
  appIds: Array<string>;
};

export type AppManagerResolvedSizeDto = {
  appId: string;
  sizeBytes: number | null;
  sizeAccuracy: AppManagerSizeAccuracy;
  sizeSource: AppManagerSizeSource;
  sizeComputedAt: number | null;
};

export type AppManagerResolveSizesResultDto = {
  items: Array<AppManagerResolvedSizeDto>;
};

export type AppManagerResidueItemDto = {
  itemId: string;
  path: string;
  pathType: AppManagerPathType;
  kind: AppManagerResidueKind;
  scope: AppManagerScope;
  sizeBytes: number;
  matchReason: AppManagerResidueMatchReason;
  confidence: AppManagerResidueConfidence;
  evidence: Array<string>;
  riskLevel: AppManagerRiskLevel;
  recommended: boolean;
  readonly: boolean;
  readonlyReasonCode: AppReadonlyReasonCode | null;
};

export type AppManagerResidueGroupDto = {
  groupId: string;
  label: string;
  scope: AppManagerScope;
  kind: AppManagerResidueKind;
  totalSizeBytes: number;
  items: Array<AppManagerResidueItemDto>;
};

export type AppManagerScanWarningDto = {
  code: AppManagerScanWarningCode;
  path: string | null;
  detailCode: AppManagerScanWarningDetailCode | null;
};

export type AppManagerScanWarningCode =
  | "app_manager_size_metadata_read_failed"
  | "app_manager_size_estimate_truncated"
  | "app_manager_size_read_dir_failed"
  | "app_manager_size_read_dir_entry_failed"
  | "app_manager_size_read_file_type_failed"
  | "app_manager_size_read_metadata_failed";

export type AppManagerScanWarningDetailCode =
  | "permission_denied"
  | "not_found"
  | "interrupted"
  | "invalid_data"
  | "timed_out"
  | "would_block"
  | "limit_reached"
  | "io_other";

export type AppManagerResidueScanResultDto = {
  appId: string;
  scanMode: AppManagerResidueScanMode;
  totalSizeBytes: number;
  groups: Array<AppManagerResidueGroupDto>;
  warnings: Array<AppManagerScanWarningDto>;
};

export type AppManagerCleanupInputDto = {
  appId: string;
  selectedItemIds: Array<string>;
  deleteMode: AppManagerCleanupDeleteMode;
  includeMainApp: boolean;
  skipOnError: boolean | null;
  confirmedFingerprint: string | null;
};

export type AppManagerCleanupDeleteMode =
  | "trash"
  | "permanent";

export type AppManagerCleanupStatus =
  | "deleted"
  | "skipped"
  | "failed";

export type AppManagerCleanupReasonCode =
  | "ok"
  | "self_uninstall_forbidden"
  | "managed_by_policy"
  | "not_found"
  | "app_manager_cleanup_delete_failed"
  | "app_manager_cleanup_not_found"
  | "app_manager_cleanup_path_invalid"
  | "app_manager_cleanup_not_supported"
  | "app_manager_uninstall_failed";

export type AppManagerCleanupItemResultDto = {
  itemId: string;
  path: string;
  kind: AppManagerResidueKind;
  status: AppManagerCleanupStatus;
  reasonCode: AppManagerCleanupReasonCode;
  message: string;
  sizeBytes: number | null;
};

export type AppManagerCleanupResultDto = {
  appId: string;
  deleteMode: AppManagerCleanupDeleteMode;
  releasedSizeBytes: number;
  deleted: Array<AppManagerCleanupItemResultDto>;
  skipped: Array<AppManagerCleanupItemResultDto>;
  failed: Array<AppManagerCleanupItemResultDto>;
};

export type AppManagerExportScanInputDto = {
  appId: string;
};

export type AppManagerExportScanResultDto = {
  appId: string;
  filePath: string;
  directoryPath: string;
};

export type AppManagerActionResultDto = {
  ok: boolean;
  code: AppManagerActionCode;
  message: string;
  detail: string | null;
};

export type AppManagerActionCode =
  | "app_manager_refreshed"
  | "app_manager_startup_updated"
  | "app_manager_uninstall_started"
  | "app_manager_uninstall_help_opened"
  | "app_manager_permission_help_opened";

export type ClipboardFilterDto = {
  query: string | null;
  itemType: string | null;
  onlyPinned: boolean | null;
  limit: number | null;
};

export type ClipboardItemDto = {
  id: string;
  contentKey: string;
  itemType: string;
  plainText: string;
  sourceApp: string | null;
  previewPath: string | null;
  previewDataUrl: string | null;
  createdAt: number;
  pinned: boolean;
};

export type ClipboardSettingsDto = {
  maxItems: number;
  sizeCleanupEnabled: boolean;
  maxTotalSizeMb: number;
};

export type ClipboardWindowOpenedPayload = {
  compact: boolean;
};

export type ClipboardWindowModeAppliedDto = {
  compact: boolean;
  appliedWidthLogical: number;
  appliedHeightLogical: number;
  scaleFactor: number;
};

export type ClipboardImageExportResultDto = {
  saved: boolean;
  path: string | null;
};

export type ClipboardSyncPayload = {
  upsert: Array<ClipboardItemDto>;
  removedIds: Array<string>;
  clearAll: boolean;
  reason: string | null;
};

export type ScreenshotDisplayDto = {
  id: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactor: number;
  primary: boolean;
};

export type ScreenshotSessionDto = {
  sessionId: string;
  startedAtMs: number;
  ttlMs: number;
  activeDisplayId: string;
  displays: Array<ScreenshotDisplayDto>;
};

export type ScreenshotStartInputDto = {
  displayId: string | null;
};

export type ScreenshotCommitInputDto = {
  sessionId: string;
  x: number;
  y: number;
  width: number;
  height: number;
  autoSave: boolean | null;
};

export type ScreenshotCancelInputDto = {
  sessionId: string;
};

export type ScreenshotCommitResultDto = {
  sessionId: string;
  clipboardAccepted: boolean;
  clipboardAsync: boolean;
  archivePath: string | null;
  width: number;
  height: number;
  createdAtMs: number;
};

export type ScreenshotPinResultDto = {
  sessionId: string;
  clipboardAccepted: boolean;
  clipboardAsync: boolean;
  windowLabel: string;
  width: number;
  height: number;
  createdAtMs: number;
};

export type ScreenshotOperationResultPayload = {
  sessionId: string;
  operation: string;
  phase: string;
  ok: boolean;
  archivePath: string | null;
  errorCode: string | null;
  errorMessage: string | null;
  createdAtMs: number;
};

export type ScreenshotWindowOpenedPayload = {
  session: ScreenshotSessionDto;
};

export type ScreenshotPinWindowOpenedPayload = {
  targetWindowLabel: string;
  imagePath: string;
  screenX: number;
  screenY: number;
  width: number;
  height: number;
  createdAtMs: number;
};

export type LauncherRuntimeStatusDto = {
  started: boolean;
  building: boolean;
};

export type LogQueryDto = {
  cursor: string | null;
  limit: number;
  levels: Array<string> | null;
  scope: string | null;
  requestId: string | null;
  windowLabel: string | null;
  keyword: string | null;
  startAt: number | null;
  endAt: number | null;
};

export type LogEntryDto = {
  id: number;
  timestamp: number;
  level: string;
  scope: string;
  event: string;
  requestId: string;
  windowLabel: string | null;
  message: string;
  metadata: JsonValue | null;
  rawRef: string | null;
  aggregatedCount: number | null;
};

export type LogPageDto = {
  items: Array<LogEntryDto>;
  nextCursor: string | null;
};

export type LogConfigDto = {
  minLevel: string;
  keepDays: number;
  realtimeEnabled: boolean;
  highFreqWindowMs: number;
  highFreqMaxPerKey: number;
  allowRawView: boolean;
};

export type LocaleStateDto = {
  preference: string;
  resolved: string;
};

// <generated-models:end>

// <generated-contracts:start>
export type AppFeatureKey =
  | "app_manager"
  | "clipboard"
  | "launcher"
  | "locale"
  | "logging"
  | "screenshot"
  | "settings";

type CommandNoPayload<K extends string> = { kind: K };
type CommandWithPayload<K extends string, P> = { kind: K; payload: P };

export type InvokeMetaDto = {
  requestId?: string;
  windowLabel?: string;
};

export type AppFeatureRequestMap = {
  "app_manager": AppManagerRequestDto;
  "clipboard": ClipboardRequestDto;
  "launcher": LauncherRequestDto;
  "locale": LocaleRequestDto;
  "logging": LoggingRequestDto;
  "screenshot": ScreenshotRequestDto;
  "settings": SettingsRequestDto;
};

export type AppManagerRequestDto =
  | CommandWithPayload<"list", { query?: AppManagerQueryDto }>
  | CommandWithPayload<"get_detail", { query: AppManagerDetailQueryDto }>
  | CommandNoPayload<"list_snapshot_meta">
  | CommandWithPayload<"resolve_sizes", { input: AppManagerResolveSizesInputDto }>
  | CommandWithPayload<"get_detail_core", { query: AppManagerDetailQueryDto }>
  | CommandWithPayload<"get_detail_heavy", { input: AppManagerResidueScanInputDto }>
  | CommandWithPayload<"scan_residue", { input: AppManagerResidueScanInputDto }>
  | CommandWithPayload<"cleanup", { input: AppManagerCleanupInputDto }>
  | CommandWithPayload<"export_scan_result", { input: AppManagerExportScanInputDto }>
  | CommandNoPayload<"refresh_index">
  | CommandWithPayload<"set_startup", { input: AppManagerStartupUpdateInputDto }>
  | CommandWithPayload<"uninstall", { input: AppManagerUninstallInputDto }>
  | CommandWithPayload<"open_uninstall_help", { appId: string }>
  | CommandWithPayload<"open_permission_help", { appId: string }>
  | CommandWithPayload<"reveal_path", { path: string }>;

export type ClipboardRequestDto =
  | CommandWithPayload<"list", { filter?: ClipboardFilterDto }>
  | CommandWithPayload<"pin", { id: string; pinned: boolean }>
  | CommandWithPayload<"delete", { id: string }>
  | CommandNoPayload<"clear_all">
  | CommandWithPayload<"save_text", { text: string }>
  | CommandWithPayload<"window_set_mode", { compact: boolean }>
  | CommandWithPayload<"window_apply_mode", { compact: boolean }>
  | CommandWithPayload<"copy_back", { id: string }>
  | CommandWithPayload<"copy_file_paths", { id: string }>
  | CommandWithPayload<"copy_image_back", { id: string }>
  | CommandWithPayload<"export_image", { id: string }>;

export type LauncherRequestDto =
  | CommandWithPayload<"search", { query: string; limit?: number }>
  | CommandWithPayload<"execute", { action: LauncherActionDto }>
  | CommandNoPayload<"get_search_settings">
  | CommandWithPayload<"update_search_settings", { input: LauncherUpdateSearchSettingsInputDto }>
  | CommandNoPayload<"get_status">
  | CommandNoPayload<"rebuild_index">
  | CommandNoPayload<"reset_search_settings">;

export type LocaleRequestDto =
  | CommandNoPayload<"get">
  | CommandWithPayload<"set", { preference: string }>;

export type LoggingRequestDto =
  | CommandWithPayload<"client_log", { level: string; scope: string; message: string; metadata?: JsonValue; requestId?: string }>
  | CommandWithPayload<"query", { query?: LogQueryDto }>
  | CommandNoPayload<"get_config">
  | CommandWithPayload<"update_config", { config: LogConfigDto }>
  | CommandWithPayload<"export_jsonl", { query?: LogQueryDto; outputPath?: string }>;

export type ScreenshotRequestDto =
  | CommandWithPayload<"start_session", { input: ScreenshotStartInputDto }>
  | CommandWithPayload<"commit_selection", { input: ScreenshotCommitInputDto }>
  | CommandWithPayload<"pin_selection", { input: ScreenshotCommitInputDto }>
  | CommandWithPayload<"cancel_session", { input: ScreenshotCancelInputDto }>
  | CommandNoPayload<"get_settings">
  | CommandWithPayload<"update_settings", { input: SettingsScreenshotUpdateInputDto }>;

export type SettingsRequestDto =
  | CommandNoPayload<"get">
  | CommandWithPayload<"update", { input: SettingsUpdateInputDto }>;

// <generated-contracts:end>

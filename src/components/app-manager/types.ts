export type AppReadonlyReasonCode = "permission_denied" | "managed_by_policy" | "feature_disabled";

export type AppManagerScope = "user" | "system";
export type AppManagerPathType = "file" | "directory";
export type AppManagerStartupScope = "user" | "system" | "none";
export type AppManagerIdentitySource = "bundle_id" | "registry" | "path";
export type AppManagerSource = "rtool" | "application";
export type AppManagerCategory = "all" | "rtool" | "application" | "startup";
export type AppManagerQueryCategory = AppManagerCategory;
export type AppManagerPlatform = "macos" | "windows" | "linux";
export type AppManagerIconKind = "raster" | "iconify";
export type AppManagerSizeAccuracy = "exact" | "estimated";
export type AppManagerIndexState = "ready" | "building" | "degraded";
export type AppManagerIndexUpdateReason = "manual" | "auto_change" | "startup";
export type AppManagerUninstallKind = "finder_trash" | "registry_command";
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
export type AppManagerResidueConfidence = "exact" | "high" | "medium";
export type AppManagerRiskLevel = "low" | "medium" | "high";
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
export type AppManagerActionCode =
  | "app_manager_refreshed"
  | "app_manager_startup_updated"
  | "app_manager_uninstall_started"
  | "app_manager_uninstall_help_opened";
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

export interface AppManagerCapabilities {
  startup: boolean;
  uninstall: boolean;
  residueScan: boolean;
}

export interface AppManagerIdentity {
  primaryId: string;
  aliases: string[];
  identitySource: AppManagerIdentitySource;
}

export interface ManagedApp {
  id: string;
  name: string;
  path: string;
  bundleOrAppId?: string;
  version?: string;
  publisher?: string;
  platform: AppManagerPlatform;
  source: AppManagerSource;
  iconKind: AppManagerIconKind;
  iconValue: string;
  sizeBytes: number | null;
  sizeAccuracy: AppManagerSizeAccuracy;
  sizeComputedAt: number | null;
  startupEnabled: boolean;
  startupScope: AppManagerStartupScope;
  startupEditable: boolean;
  readonlyReasonCode?: AppReadonlyReasonCode;
  uninstallSupported: boolean;
  uninstallKind?: AppManagerUninstallKind;
  capabilities: AppManagerCapabilities;
  identity: AppManagerIdentity;
  riskLevel: AppManagerRiskLevel;
  fingerprint: string;
}

export interface AppManagerQuery {
  keyword?: string;
  category?: AppManagerQueryCategory;
  limit?: number;
  cursor?: string;
}

export interface AppManagerPage {
  items: ManagedApp[];
  nextCursor: string | null;
  totalCount: number;
  indexedAt: number;
  revision: number;
  indexState: AppManagerIndexState;
}

export interface AppManagerSnapshotMeta {
  indexedAt: number;
  revision: number;
  totalCount: number;
  indexState: AppManagerIndexState;
}

export interface AppManagerIndexUpdatedPayload {
  revision: number;
  indexedAt: number;
  changedCount: number;
  reason: AppManagerIndexUpdateReason;
}

export interface AppManagerActionResult {
  ok: boolean;
  code: AppManagerActionCode;
  message: string;
  detail?: string;
}

export interface AppManagerStartupUpdateInput {
  appId: string;
  enabled: boolean;
}

export interface AppManagerUninstallInput {
  appId: string;
  confirmedFingerprint: string;
}

export interface AppRelatedRoot {
  id: string;
  label: string;
  path: string;
  pathType: AppManagerPathType;
  scope: AppManagerScope;
  kind: AppManagerResidueKind;
  exists: boolean;
  readonly: boolean;
  readonlyReasonCode?: AppReadonlyReasonCode;
}

export interface AppSizeSummary {
  appBytes?: number | null;
  residueBytes?: number | null;
  totalBytes?: number | null;
}

export interface ManagedAppDetail {
  app: ManagedApp;
  installPath: string;
  relatedRoots: AppRelatedRoot[];
  sizeSummary: AppSizeSummary;
}

export interface AppManagerResidueItem {
  itemId: string;
  path: string;
  pathType: AppManagerPathType;
  kind: AppManagerResidueKind;
  scope: AppManagerScope;
  sizeBytes: number;
  matchReason: AppManagerResidueMatchReason;
  confidence: AppManagerResidueConfidence;
  evidence: string[];
  riskLevel: AppManagerRiskLevel;
  recommended: boolean;
  readonly: boolean;
  readonlyReasonCode?: AppReadonlyReasonCode;
}

export interface AppManagerResidueGroup {
  groupId: string;
  label: string;
  scope: AppManagerScope;
  kind: AppManagerResidueKind;
  totalSizeBytes: number;
  items: AppManagerResidueItem[];
}

export interface AppManagerScanWarning {
  code: AppManagerScanWarningCode;
  path?: string;
  detailCode?: AppManagerScanWarningDetailCode;
}

export interface AppManagerResidueScanResult {
  appId: string;
  scanMode: AppManagerResidueScanMode;
  totalSizeBytes: number;
  groups: AppManagerResidueGroup[];
  warnings: AppManagerScanWarning[];
}

export type AppManagerResidueScanMode = "quick" | "deep";

export interface AppManagerResolveSizesInput {
  appIds: string[];
}

export interface AppManagerResolvedSize {
  appId: string;
  sizeBytes: number | null;
  sizeAccuracy: AppManagerSizeAccuracy;
  sizeComputedAt: number | null;
}

export interface AppManagerResolveSizesResult {
  items: AppManagerResolvedSize[];
}

export interface AppManagerCleanupInput {
  appId: string;
  selectedItemIds: string[];
  deleteMode: AppManagerCleanupDeleteMode;
  includeMainApp: boolean;
  skipOnError?: boolean;
  confirmedFingerprint?: string;
}

export type AppManagerCleanupDeleteMode = "trash" | "permanent";

export interface AppManagerCleanupItemResult {
  itemId: string;
  path: string;
  kind: AppManagerResidueKind;
  status: "deleted" | "skipped" | "failed";
  reasonCode: AppManagerCleanupReasonCode;
  message: string;
  sizeBytes?: number;
}

export interface AppManagerCleanupResult {
  appId: string;
  deleteMode: AppManagerCleanupDeleteMode;
  releasedSizeBytes: number;
  deleted: AppManagerCleanupItemResult[];
  skipped: AppManagerCleanupItemResult[];
  failed: AppManagerCleanupItemResult[];
}

export interface AppManagerExportScanResult {
  appId: string;
  filePath: string;
  directoryPath: string;
}

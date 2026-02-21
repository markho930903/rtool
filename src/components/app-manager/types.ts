export type AppReadonlyReasonCode =
  | "permission_denied"
  | "managed_by_policy"
  | "feature_disabled"
  | "unknown";

export type AppManagerScope = "user" | "system" | "unknown";
export type AppManagerPathType = "file" | "directory" | "unknown";
export type AppManagerStartupScope = "user" | "system" | "none" | "unknown";
export type AppManagerIdentitySource = "bundle_id" | "registry" | "path" | "unknown";
export type AppManagerSource = "rtool" | "application" | "unknown";
export type AppManagerCategory = "all" | "rtool" | "application" | "startup" | "unknown";
export type AppManagerQueryCategory = Exclude<AppManagerCategory, "unknown">;
export type AppManagerPlatform = "macos" | "windows" | "linux" | "unknown";
export type AppManagerIconKind = "raster" | "iconify" | "unknown";
export type AppManagerUninstallKind = "finder_trash" | "registry_command" | "unknown";
export type AppManagerResidueKind =
  | "install"
  | "app_support"
  | "cache"
  | "preferences"
  | "logs"
  | "startup"
  | "app_data"
  | "registry_key"
  | "registry_value"
  | "main_app"
  | "unknown";
export type AppManagerResidueConfidence = "exact" | "high" | "medium" | "unknown";
export type AppManagerRiskLevel = "low" | "medium" | "high" | "unknown";
export type AppManagerResidueMatchReason =
  | "related_root"
  | "bundle_id"
  | "startup_label"
  | "startup_shortcut"
  | "uninstall_registry"
  | "startup_registry"
  | "run_registry"
  | "unknown";
export type AppManagerScanWarningCode =
  | "app_manager_size_metadata_read_failed"
  | "app_manager_size_estimate_truncated"
  | "app_manager_size_read_dir_failed"
  | "app_manager_size_read_dir_entry_failed"
  | "app_manager_size_read_file_type_failed"
  | "app_manager_size_read_metadata_failed"
  | "unknown";
export type AppManagerScanWarningDetailCode =
  | "permission_denied"
  | "not_found"
  | "interrupted"
  | "invalid_data"
  | "timed_out"
  | "would_block"
  | "limit_reached"
  | "io_other"
  | "unknown";
export type AppManagerActionCode =
  | "app_manager_refreshed"
  | "app_manager_startup_updated"
  | "app_manager_uninstall_started"
  | "app_manager_uninstall_help_opened"
  | "unknown";
export type AppManagerCleanupReasonCode =
  | "ok"
  | "self_uninstall_forbidden"
  | "managed_by_policy"
  | "not_found"
  | "app_manager_cleanup_delete_failed"
  | "app_manager_cleanup_not_found"
  | "app_manager_cleanup_path_invalid"
  | "app_manager_cleanup_not_supported"
  | "app_manager_uninstall_failed"
  | "unknown";

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
  estimatedSizeBytes?: number | null;
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
  nextCursor?: string | null;
  indexedAt: number;
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
  pathType?: AppManagerPathType;
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
  pathType?: AppManagerPathType;
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
  totalSizeBytes: number;
  groups: AppManagerResidueGroup[];
  warnings: AppManagerScanWarning[];
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
export type AppManagerCleanupDeleteModeResult = AppManagerCleanupDeleteMode | "unknown";

export interface AppManagerCleanupItemResult {
  itemId: string;
  path: string;
  kind: AppManagerResidueKind;
  status: "deleted" | "skipped" | "failed" | "unknown";
  reasonCode: AppManagerCleanupReasonCode;
  message: string;
  sizeBytes?: number;
}

export interface AppManagerCleanupResult {
  appId: string;
  deleteMode: AppManagerCleanupDeleteModeResult;
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

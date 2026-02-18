export type AppReadonlyReasonCode =
  | "permission_denied"
  | "managed_by_policy"
  | "feature_disabled"
  | "unknown"
  | string;

export interface ManagedApp {
  id: string;
  name: string;
  path: string;
  bundleOrAppId?: string;
  version?: string;
  publisher?: string;
  platform: string;
  source: string;
  iconKind: "raster" | "iconify" | string;
  iconValue: string;
  estimatedSizeBytes?: number | null;
  startupEnabled: boolean;
  startupScope: "user" | "system" | "none" | "unknown" | string;
  startupEditable: boolean;
  readonlyReasonCode?: AppReadonlyReasonCode;
  uninstallSupported: boolean;
  uninstallKind?: string;
  riskLevel: "low" | "medium" | "high" | string;
  fingerprint: string;
}

export interface AppManagerQuery {
  keyword?: string;
  category?: string;
  startupOnly?: boolean;
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
  code: string;
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
  scope: "user" | "system" | string;
  kind: string;
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
  kind: string;
  scope: "user" | "system" | string;
  sizeBytes: number;
  matchReason: string;
  riskLevel: "low" | "medium" | "high" | string;
  recommended: boolean;
  readonly: boolean;
  readonlyReasonCode?: AppReadonlyReasonCode;
}

export interface AppManagerResidueGroup {
  groupId: string;
  label: string;
  scope: "user" | "system" | string;
  kind: string;
  totalSizeBytes: number;
  items: AppManagerResidueItem[];
}

export interface AppManagerScanWarning {
  code: string;
  message: string;
  detail?: string;
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
  deleteMode: "trash" | "permanent";
  includeMainApp: boolean;
  skipOnError?: boolean;
  confirmedFingerprint?: string;
}

export interface AppManagerCleanupItemResult {
  itemId: string;
  path: string;
  kind: string;
  status: "deleted" | "skipped" | "failed" | string;
  reasonCode: string;
  message: string;
  sizeBytes?: number;
}

export interface AppManagerCleanupResult {
  appId: string;
  deleteMode: "trash" | "permanent" | string;
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

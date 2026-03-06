import type {
  AppManagerActionCode,
  AppManagerCleanupDeleteMode,
  AppManagerCleanupReasonCode,
  AppManagerCapabilitiesDto,
  AppManagerCategory,
  AppManagerIconKind,
  AppManagerIdentityDto,
  AppManagerIdentitySource,
  AppManagerIndexState,
  AppManagerIndexUpdateReason,
  AppManagerIndexUpdatedPayloadDto,
  AppManagerPageDto,
  AppManagerPathType,
  AppManagerPlatform,
  AppManagerResidueConfidence,
  AppManagerResidueGroupDto,
  AppManagerResidueItemDto,
  AppManagerResidueKind,
  AppManagerResidueMatchReason,
  AppManagerResidueScanMode,
  AppManagerResidueScanResultDto,
  AppManagerResolveSizesInputDto,
  AppManagerResolvedSizeDto,
  AppManagerResolveSizesResultDto,
  AppManagerRiskLevel,
  AppManagerScanWarningCode,
  AppManagerScanWarningDetailCode,
  AppManagerScanWarningDto,
  AppManagerScope,
  AppManagerSizeAccuracy,
  AppManagerSizeSource,
  AppManagerSnapshotMetaDto,
  AppManagerSource,
  AppManagerStartupScope,
  AppManagerStartupUpdateInputDto,
  AppManagerUninstallInputDto,
  AppManagerUninstallKind,
  AppReadonlyReasonCode,
  AppRelatedRootDto,
  AppSizeSummaryDto,
  ManagedAppDetailDto,
  ManagedAppDto,
  AppManagerActionResultDto,
  AppManagerCleanupItemResultDto,
  AppManagerCleanupResultDto,
  AppManagerExportScanResultDto,
} from "@/contracts";

export type {
  AppManagerActionCode,
  AppManagerCleanupDeleteMode,
  AppManagerCleanupReasonCode,
  AppManagerIndexState,
  AppManagerIndexUpdateReason,
  AppManagerPathType,
  AppManagerPlatform,
  AppManagerResidueConfidence,
  AppManagerResidueKind,
  AppManagerResidueMatchReason,
  AppManagerResidueScanMode,
  AppManagerRiskLevel,
  AppManagerScanWarningCode,
  AppManagerScanWarningDetailCode,
  AppManagerScope,
  AppManagerSizeAccuracy,
  AppManagerSizeSource,
  AppManagerSource,
  AppManagerStartupScope,
  AppManagerUninstallKind,
  AppReadonlyReasonCode,
  AppManagerCategory,
  AppManagerIconKind,
  AppManagerIdentitySource,
};

export type AppManagerQueryCategory = AppManagerCategory;
export type AppManagerCapabilities = AppManagerCapabilitiesDto;
export type AppManagerIdentity = AppManagerIdentityDto;
export type ManagedApp = ManagedAppDto;
export type AppManagerPage = AppManagerPageDto;
export type AppManagerSnapshotMeta = AppManagerSnapshotMetaDto;
export type AppManagerIndexUpdatedPayload = AppManagerIndexUpdatedPayloadDto;
export type AppManagerActionResult = AppManagerActionResultDto;
export type AppManagerStartupUpdateInput = AppManagerStartupUpdateInputDto;
export type AppManagerUninstallInput = AppManagerUninstallInputDto;
export type AppRelatedRoot = AppRelatedRootDto;
export type AppSizeSummary = AppSizeSummaryDto;
export type ManagedAppDetail = ManagedAppDetailDto;
export type AppManagerResidueItem = AppManagerResidueItemDto;
export type AppManagerResidueGroup = AppManagerResidueGroupDto;
export type AppManagerScanWarning = AppManagerScanWarningDto;
export type AppManagerResidueScanResult = AppManagerResidueScanResultDto;
export type AppManagerResolveSizesInput = AppManagerResolveSizesInputDto;
export type AppManagerResolvedSize = AppManagerResolvedSizeDto;
export type AppManagerResolveSizesResult = AppManagerResolveSizesResultDto;
export type AppManagerCleanupItemResult = AppManagerCleanupItemResultDto;
export type AppManagerCleanupResult = AppManagerCleanupResultDto;
export type AppManagerExportScanResult = AppManagerExportScanResultDto;

export interface AppManagerQuery {
  keyword?: string;
  category?: AppManagerQueryCategory;
  limit?: number;
  cursor?: string;
}

export interface AppManagerCleanupInput {
  appId: string;
  selectedItemIds: string[];
  deleteMode: AppManagerCleanupDeleteMode;
  includeMainApp: boolean;
  skipOnError?: boolean;
  confirmedFingerprint?: string;
}

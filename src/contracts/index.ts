/* eslint-disable */
// 手动维护的前后端合同类型单一事实源。
// 修改后请同步检查 tauri-shell 命令注册清单与 services 调用。

export type ActionResultDto = { ok: boolean, message: string, };

export type AppErrorPayload = { code: string, message: string, context: Array<ErrorContextItem>, causes: Array<string>, requestId: string | null, };

export type AppManagerActionCode = "app_manager_refreshed" | "app_manager_startup_updated" | "app_manager_uninstall_started" | "app_manager_uninstall_help_opened" | "unknown";

export type AppManagerActionResultDto = { ok: boolean, code: AppManagerActionCode, message: string, detail: string | null, };

export type AppManagerCapabilitiesDto = { startup: boolean, uninstall: boolean, residueScan: boolean, };

export type AppManagerCategory = "all" | "rtool" | "application" | "startup" | "unknown";

export type AppManagerCleanupDeleteMode = "trash" | "permanent" | "unknown";

export type AppManagerCleanupInputDto = { appId: string, selectedItemIds: Array<string>, deleteMode: AppManagerCleanupDeleteMode, includeMainApp: boolean, skipOnError: boolean | null, confirmedFingerprint: string | null, };

export type AppManagerCleanupItemResultDto = { itemId: string, path: string, kind: AppManagerResidueKind, status: AppManagerCleanupStatus, reasonCode: AppManagerCleanupReasonCode, message: string, sizeBytes: number | null, };

export type AppManagerCleanupReasonCode = "ok" | "self_uninstall_forbidden" | "managed_by_policy" | "not_found" | "app_manager_cleanup_delete_failed" | "app_manager_cleanup_not_found" | "app_manager_cleanup_path_invalid" | "app_manager_cleanup_not_supported" | "app_manager_uninstall_failed" | "unknown";

export type AppManagerCleanupResultDto = { appId: string, deleteMode: AppManagerCleanupDeleteMode, releasedSizeBytes: number, deleted: Array<AppManagerCleanupItemResultDto>, skipped: Array<AppManagerCleanupItemResultDto>, failed: Array<AppManagerCleanupItemResultDto>, };

export type AppManagerCleanupStatus = "deleted" | "skipped" | "failed" | "unknown";

export type AppManagerDetailQueryDto = { appId: string, };

export type AppManagerExportScanInputDto = { appId: string, };

export type AppManagerExportScanResultDto = { appId: string, filePath: string, directoryPath: string, };

export type AppManagerIconKind = "raster" | "iconify" | "unknown";

export type AppManagerIdentityDto = { primaryId: string, aliases: Array<string>, identitySource: AppManagerIdentitySource, };

export type AppManagerIdentitySource = "bundle_id" | "registry" | "path" | "unknown";

export type AppManagerPageDto = { items: Array<ManagedAppDto>, nextCursor: string | null, indexedAt: number, };

export type AppManagerPathType = "file" | "directory" | "unknown";

export type AppManagerPlatform = "macos" | "windows" | "linux" | "unknown";

export type AppManagerQueryDto = { keyword: string | null, category: AppManagerCategory, limit: number | null, cursor: string | null, };

export type AppManagerResidueConfidence = "exact" | "high" | "medium" | "unknown";

export type AppManagerResidueGroupDto = { groupId: string, label: string, scope: AppManagerScope, kind: AppManagerResidueKind, totalSizeBytes: number, items: Array<AppManagerResidueItemDto>, };

export type AppManagerResidueItemDto = { itemId: string, path: string, pathType: AppManagerPathType, kind: AppManagerResidueKind, scope: AppManagerScope, sizeBytes: number, matchReason: AppManagerResidueMatchReason, confidence: AppManagerResidueConfidence, evidence: Array<string>, riskLevel: AppManagerRiskLevel, recommended: boolean, readonly: boolean, readonlyReasonCode: AppReadonlyReasonCode | null, };

export type AppManagerResidueKind = "install" | "app_support" | "cache" | "preferences" | "logs" | "startup" | "app_data" | "registry_key" | "registry_value" | "main_app" | "unknown";

export type AppManagerResidueMatchReason = "related_root" | "bundle_id" | "startup_label" | "startup_shortcut" | "uninstall_registry" | "startup_registry" | "run_registry" | "unknown";

export type AppManagerResidueScanInputDto = { appId: string, };

export type AppManagerResidueScanResultDto = { appId: string, totalSizeBytes: number, groups: Array<AppManagerResidueGroupDto>, warnings: Array<AppManagerScanWarningDto>, };

export type AppManagerRiskLevel = "low" | "medium" | "high" | "unknown";

export type AppManagerScanWarningCode = "app_manager_size_metadata_read_failed" | "app_manager_size_estimate_truncated" | "app_manager_size_read_dir_failed" | "app_manager_size_read_dir_entry_failed" | "app_manager_size_read_file_type_failed" | "app_manager_size_read_metadata_failed" | "unknown";

export type AppManagerScanWarningDetailCode = "permission_denied" | "not_found" | "interrupted" | "invalid_data" | "timed_out" | "would_block" | "limit_reached" | "io_other" | "unknown";

export type AppManagerScanWarningDto = { code: AppManagerScanWarningCode, path: string | null, detailCode: AppManagerScanWarningDetailCode | null, };

export type AppManagerScope = "user" | "system" | "unknown";

export type AppManagerSource = "rtool" | "application" | "unknown";

export type AppManagerStartupScope = "user" | "system" | "none" | "unknown";

export type AppManagerStartupUpdateInputDto = { appId: string, enabled: boolean, };

export type AppManagerUninstallInputDto = { appId: string, confirmedFingerprint: string, };

export type AppManagerUninstallKind = "finder_trash" | "registry_command" | "unknown";

export type AppReadonlyReasonCode = "permission_denied" | "managed_by_policy" | "feature_disabled" | "unknown";

export type AppRelatedRootDto = { id: string, label: string, path: string, pathType: AppManagerPathType, scope: AppManagerScope, kind: AppManagerResidueKind, exists: boolean, readonly: boolean, readonlyReasonCode: AppReadonlyReasonCode | null, };

export type AppRuntimeInfoDto = { appName: string, appVersion: string, buildMode: string, uptimeSeconds: number, processMemoryBytes: number | null, databaseSizeBytes: number | null, };

export type AppSizeSummaryDto = { appBytes: number | null, residueBytes: number | null, totalBytes: number | null, };

export type ClipboardFilterDto = { query: string | null, itemType: string | null, onlyPinned: boolean | null, limit: number | null, };

export type ClipboardItemDto = { id: string, contentKey: string, itemType: string, plainText: string, sourceApp: string | null, previewPath: string | null, previewDataUrl: string | null, createdAt: number, pinned: boolean, };

export type ClipboardSettingsDto = { maxItems: number, sizeCleanupEnabled: boolean, maxTotalSizeMb: number, };

export type ClipboardSyncPayload = { upsert: Array<ClipboardItemDto>, removedIds: Array<string>, clearAll: boolean, reason: string | null, };

export type ClipboardWindowModeAppliedDto = { compact: boolean, appliedWidthLogical: number, appliedHeightLogical: number, scaleFactor: number, };

export type ClipboardWindowOpenedPayload = { compact: boolean, };

export type CommandKey = "clipboard_list" | "clipboard_pin" | "clipboard_delete" | "clipboard_clear_all" | "clipboard_save_text" | "clipboard_copy_back" | "clipboard_copy_file_paths" | "clipboard_copy_image_back" | "clipboard_get_settings" | "clipboard_update_settings" | "clipboard_window_set_mode" | "clipboard_window_apply_mode" | "launcher_search" | "launcher_execute" | "launcher_get_search_settings" | "launcher_update_search_settings" | "launcher_get_index_status" | "launcher_rebuild_index" | "palette_search" | "palette_execute" | "app_get_locale" | "app_set_locale" | "app_list_locales" | "app_reload_locales" | "app_import_locale_file" | "dashboard_snapshot" | "client_log" | "logging_query" | "logging_get_config" | "logging_update_config" | "logging_export_jsonl" | "transfer_get_settings" | "transfer_update_settings" | "transfer_generate_pairing_code" | "transfer_start_discovery" | "transfer_stop_discovery" | "transfer_list_peers" | "transfer_send_files" | "transfer_pause_session" | "transfer_resume_session" | "transfer_cancel_session" | "transfer_retry_session" | "transfer_list_history" | "transfer_clear_history" | "transfer_open_download_dir" | "app_manager_list" | "app_manager_get_detail" | "app_manager_scan_residue" | "app_manager_cleanup" | "app_manager_export_scan_result" | "app_manager_refresh_index" | "app_manager_set_startup" | "app_manager_uninstall" | "app_manager_open_uninstall_help" | "app_manager_reveal_path";

export type DashboardSnapshotDto = { sampledAt: number, app: AppRuntimeInfoDto, system: SystemInfoDto, };

export type ErrorContextItem = { key: string, value: string, };

export type ImportLocaleResult = { success: boolean, locale: string, namespace: string, importedKeys: number, warnings: Array<string>, effectiveLocaleNamespaces: Array<string>, };

export type InvokeError = { code: string, message: string, context: Array<ErrorContextItem>, causes: Array<string>, requestId: string | null, };

export type JsonValue = number | string | boolean | Array<JsonValue> | { [key in string]?: JsonValue } | null;

export type LauncherActionDto = { "kind": "open_builtin_route", route: string, } | { "kind": "open_builtin_tool", toolId: string, } | { "kind": "open_builtin_window", windowLabel: string, } | { "kind": "open_directory", path: string, } | { "kind": "open_file", path: string, } | { "kind": "open_application", path: string, };

export type LauncherIndexStatusDto = { ready: boolean, building: boolean, indexedItems: number, indexedRoots: number, lastBuildMs: number | null, lastDurationMs: number | null, lastError: string | null, refreshIntervalSecs: number, indexVersion: string, truncated: boolean, };

export type LauncherItemDto = { id: string, title: string, subtitle: string, category: string, source: string | null, shortcut: string | null, score: number, iconKind: string, iconValue: string, action: LauncherActionDto, };

export type LauncherRebuildResultDto = { success: boolean, durationMs: number, indexedItems: number, indexedRoots: number, truncated: boolean, ready: boolean, };

export type LauncherSearchSettingsDto = { roots: Array<string>, excludePatterns: Array<string>, maxScanDepth: number, maxItemsPerRoot: number, maxTotalItems: number, refreshIntervalSecs: number, };

export type LauncherUpdateSearchSettingsInputDto = { roots: Array<string> | null, excludePatterns: Array<string> | null, maxScanDepth: number | null, maxItemsPerRoot: number | null, maxTotalItems: number | null, refreshIntervalSecs: number | null, };

export type LocaleCatalogList = { builtinLocales: Array<LocaleNamespaces>, overlayLocales: Array<LocaleNamespaces>, effectiveLocales: Array<LocaleNamespaces>, };

export type LocaleNamespaces = { locale: string, namespaces: Array<string>, };

export type LocaleStateDto = { preference: string, resolved: string, };

export type LogConfigDto = { minLevel: string, keepDays: number, realtimeEnabled: boolean, highFreqWindowMs: number, highFreqMaxPerKey: number, allowRawView: boolean, };

export type LogEntryDto = { id: number, timestamp: number, level: string, scope: string, event: string, requestId: string, windowLabel: string | null, message: string, metadata: JsonValue | null, rawRef: string | null, aggregatedCount: number | null, };

export type LogPageDto = { items: Array<LogEntryDto>, nextCursor: string | null, };

export type LogQueryDto = { cursor: string | null, limit: number, levels: Array<string> | null, scope: string | null, requestId: string | null, windowLabel: string | null, keyword: string | null, startAt: number | null, endAt: number | null, };

export type ManagedAppDetailDto = { app: ManagedAppDto, installPath: string, relatedRoots: Array<AppRelatedRootDto>, sizeSummary: AppSizeSummaryDto, };

export type ManagedAppDto = { id: string, name: string, path: string, bundleOrAppId: string | null, version: string | null, publisher: string | null, platform: AppManagerPlatform, source: AppManagerSource, iconKind: AppManagerIconKind, iconValue: string, estimatedSizeBytes: number | null, startupEnabled: boolean, startupScope: AppManagerStartupScope, startupEditable: boolean, readonlyReasonCode: AppReadonlyReasonCode | null, uninstallSupported: boolean, uninstallKind: AppManagerUninstallKind | null, capabilities: AppManagerCapabilitiesDto, identity: AppManagerIdentityDto, riskLevel: AppManagerRiskLevel, fingerprint: string, };

export type PaletteItemDto = { id: string, title: string, subtitle: string, category: string, };

export type ReloadLocalesResult = { success: boolean, overlayLocales: Array<LocaleNamespaces>, reloadedFiles: number, warnings: Array<string>, };

export type SystemInfoDto = { osName: string | null, osVersion: string | null, kernelVersion: string | null, arch: string | null, hostName: string | null, cpuBrand: string | null, cpuCores: number | null, totalMemoryBytes: number | null, usedMemoryBytes: number | null, };

export type TransferClearHistoryInputDto = { all: boolean | null, olderThanDays: number | null, };

export type TransferDirection = "send" | "receive" | "unknown";

export type TransferFileDto = { id: string, sessionId: string, relativePath: string, sourcePath: string | null, targetPath: string | null, sizeBytes: number, transferredBytes: number, chunkSize: number, chunkCount: number, status: TransferStatus, blake3: string | null, mimeType: string | null, previewKind: string | null, previewData: string | null, isFolderArchive: boolean, };

export type TransferFileInputDto = { path: string, relativePath: string | null, compressFolder: boolean | null, };

export type TransferHistoryFilterDto = { cursor: string | null, limit: number | null, status: TransferStatus | null, peerDeviceId: string | null, };

export type TransferHistoryPageDto = { items: Array<TransferSessionDto>, nextCursor: string | null, };

export type TransferPairingCodeDto = { code: string, expiresAt: number, };

export type TransferPeerDto = { deviceId: string, displayName: string, address: string, listenPort: number, lastSeenAt: number, pairedAt: number | null, trustLevel: TransferPeerTrustLevel, failedAttempts: number, blockedUntil: number | null, pairingRequired: boolean, online: boolean, };

export type TransferPeerTrustLevel = "unknown" | "online" | "trusted" | "other";

export type TransferProgressSnapshotDto = { session: TransferSessionDto, activeFileId: string | null, speedBps: number, etaSeconds: number | null, protocolVersion: number | null, codec: string | null, inflightChunks: number | null, retransmitChunks: number | null, };

export type TransferSendFilesInputDto = { peerDeviceId: string, pairCode: string, files: Array<TransferFileInputDto>, direction: TransferDirection | null, sessionId: string | null, };

export type TransferSessionDto = { id: string, direction: TransferDirection, peerDeviceId: string, peerName: string, status: TransferStatus, totalBytes: number, transferredBytes: number, avgSpeedBps: number, saveDir: string, createdAt: number, startedAt: number | null, finishedAt: number | null, errorCode: string | null, errorMessage: string | null, cleanupAfterAt: number | null, files: Array<TransferFileDto>, };

export type TransferSettingsDto = { defaultDownloadDir: string, maxParallelFiles: number, maxInflightChunks: number, chunkSizeKb: number, autoCleanupDays: number, resumeEnabled: boolean, discoveryEnabled: boolean, pairingRequired: boolean, pipelineV2Enabled: boolean, codecV2Enabled: boolean, dbFlushIntervalMs: number, eventEmitIntervalMs: number, ackBatchSize: number, ackFlushIntervalMs: number, };

export type TransferStatus = "queued" | "running" | "paused" | "failed" | "interrupted" | "canceled" | "success" | "unknown";

export type TransferUpdateSettingsInputDto = { defaultDownloadDir: string | null, maxParallelFiles: number | null, maxInflightChunks: number | null, chunkSizeKb: number | null, autoCleanupDays: number | null, resumeEnabled: boolean | null, discoveryEnabled: boolean | null, pairingRequired: boolean | null, };

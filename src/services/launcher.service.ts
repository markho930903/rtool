import { invokeWithLog } from "@/services/invoke";

export interface LauncherSearchSettings {
  roots: string[];
  excludePatterns: string[];
  maxScanDepth: number;
  maxItemsPerRoot: number;
  maxTotalItems: number;
  refreshIntervalSecs: number;
}

export interface LauncherUpdateSearchSettingsInput {
  roots?: string[];
  excludePatterns?: string[];
  maxScanDepth?: number;
  maxItemsPerRoot?: number;
  maxTotalItems?: number;
  refreshIntervalSecs?: number;
}

export interface LauncherIndexStatus {
  ready: boolean;
  building: boolean;
  indexedItems: number;
  indexedRoots: number;
  lastBuildMs?: number | null;
  lastDurationMs?: number | null;
  lastError?: string | null;
  refreshIntervalSecs: number;
  indexVersion: string;
  truncated: boolean;
}

export interface LauncherRebuildResult {
  success: boolean;
  durationMs: number;
  indexedItems: number;
  indexedRoots: number;
  truncated: boolean;
  ready: boolean;
}

export async function launcherGetSearchSettings(): Promise<LauncherSearchSettings> {
  return invokeWithLog<LauncherSearchSettings>("launcher_get_search_settings");
}

export async function launcherUpdateSearchSettings(
  input: LauncherUpdateSearchSettingsInput,
): Promise<LauncherSearchSettings> {
  return invokeWithLog<LauncherSearchSettings>("launcher_update_search_settings", { input });
}

export async function launcherGetIndexStatus(): Promise<LauncherIndexStatus> {
  return invokeWithLog<LauncherIndexStatus>("launcher_get_index_status");
}

export async function launcherRebuildIndex(): Promise<LauncherRebuildResult> {
  return invokeWithLog<LauncherRebuildResult>("launcher_rebuild_index");
}

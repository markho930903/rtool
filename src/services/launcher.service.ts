import type {
  LauncherIndexStatusDto as LauncherIndexStatus,
  LauncherRebuildResultDto as LauncherRebuildResult,
  LauncherSearchSettingsDto as LauncherSearchSettings,
  LauncherUpdateSearchSettingsInputDto as LauncherUpdateSearchSettingsInput,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export type { LauncherIndexStatus, LauncherRebuildResult, LauncherSearchSettings, LauncherUpdateSearchSettingsInput };

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

export async function launcherResetSearchSettings(): Promise<LauncherSearchSettings> {
  return invokeWithLog<LauncherSearchSettings>("launcher_reset_search_settings");
}

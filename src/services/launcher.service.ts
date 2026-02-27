import type {
  ActionResultDto as LauncherActionResult,
  CommandRequestDto,
  LauncherActionDto as LauncherAction,
  LauncherIndexStatusDto as LauncherIndexStatus,
  LauncherItemDto as LauncherItem,
  LauncherRebuildResultDto as LauncherRebuildResult,
  LauncherSearchSettingsDto as LauncherSearchSettings,
  LauncherUpdateSearchSettingsInputDto as LauncherUpdateSearchSettingsInput,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

function invokeLauncher<T>(kind: string, payload?: Record<string, unknown>): Promise<T> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload;
  }
  return invokeWithLog<T>("launcher_handle", { request });
}

export type {
  LauncherAction,
  LauncherActionResult,
  LauncherIndexStatus,
  LauncherItem,
  LauncherRebuildResult,
  LauncherSearchSettings,
  LauncherUpdateSearchSettingsInput,
};

export async function launcherSearch(query: string, limit?: number): Promise<LauncherItem[]> {
  return invokeLauncher<LauncherItem[]>("search", { query, limit });
}

export async function launcherExecute(action: LauncherAction): Promise<LauncherActionResult> {
  return invokeLauncher<LauncherActionResult>("execute", { action });
}

export async function launcherGetSearchSettings(): Promise<LauncherSearchSettings> {
  return invokeLauncher<LauncherSearchSettings>("get_search_settings");
}

export async function launcherUpdateSearchSettings(
  input: LauncherUpdateSearchSettingsInput,
): Promise<LauncherSearchSettings> {
  return invokeLauncher<LauncherSearchSettings>("update_search_settings", { input });
}

export async function launcherGetIndexStatus(): Promise<LauncherIndexStatus> {
  return invokeLauncher<LauncherIndexStatus>("get_index_status");
}

export async function launcherRebuildIndex(): Promise<LauncherRebuildResult> {
  return invokeLauncher<LauncherRebuildResult>("rebuild_index");
}

export async function launcherResetSearchSettings(): Promise<LauncherSearchSettings> {
  return invokeLauncher<LauncherSearchSettings>("reset_search_settings");
}

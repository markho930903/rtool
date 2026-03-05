import type {
  ActionResultDto as LauncherActionResult,
  LauncherRequestDto,
  LauncherActionDto as LauncherAction,
  LauncherIndexStatusDto as LauncherIndexStatus,
  LauncherItemDto as LauncherItem,
  LauncherRebuildResultDto as LauncherRebuildResult,
  LauncherSearchSettingsDto as LauncherSearchSettings,
  LauncherUpdateSearchSettingsInputDto as LauncherUpdateSearchSettingsInput,
} from "@/contracts";
import { invokeFeature } from "@/services/invoke";

function invokeLauncher<T>(request: LauncherRequestDto): Promise<T> {
  return invokeFeature<T>("launcher", request);
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
  return invokeLauncher<LauncherItem[]>({ kind: "search", payload: { query, limit } });
}

export async function launcherExecute(action: LauncherAction): Promise<LauncherActionResult> {
  return invokeLauncher<LauncherActionResult>({ kind: "execute", payload: { action } });
}

export async function launcherGetSearchSettings(): Promise<LauncherSearchSettings> {
  return invokeLauncher<LauncherSearchSettings>({ kind: "get_search_settings" });
}

export async function launcherUpdateSearchSettings(
  input: LauncherUpdateSearchSettingsInput,
): Promise<LauncherSearchSettings> {
  return invokeLauncher<LauncherSearchSettings>({
    kind: "update_search_settings",
    payload: { input },
  });
}

export async function launcherGetIndexStatus(): Promise<LauncherIndexStatus> {
  return invokeLauncher<LauncherIndexStatus>({ kind: "get_index_status" });
}

export async function launcherRebuildIndex(): Promise<LauncherRebuildResult> {
  return invokeLauncher<LauncherRebuildResult>({ kind: "rebuild_index" });
}

export async function launcherResetSearchSettings(): Promise<LauncherSearchSettings> {
  return invokeLauncher<LauncherSearchSettings>({ kind: "reset_search_settings" });
}

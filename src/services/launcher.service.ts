import type {
  ActionResultDto as LauncherActionResult,
  LauncherActionDto as LauncherAction,
  LauncherItemDto as LauncherItem,
  LauncherRebuildResultDto as LauncherRebuildResult,
  LauncherRequestDto,
  LauncherSearchDiagnosticsDto as LauncherSearchDiagnostics,
  LauncherSearchIndexStateDto as LauncherSearchIndexState,
  LauncherSearchResponseDto as LauncherSearchResponse,
  LauncherSearchSettingsDto as LauncherSearchSettings,
  LauncherStatusDto as LauncherStatus,
  LauncherUpdateSearchSettingsInputDto as LauncherUpdateSearchSettingsInput,
} from "@/contracts";
import { invokeFeature } from "@/services/invoke";

function invokeLauncher<T>(request: LauncherRequestDto): Promise<T> {
  return invokeFeature<T>("launcher", request);
}

export type {
  LauncherAction,
  LauncherActionResult,
  LauncherItem,
  LauncherRebuildResult,
  LauncherSearchDiagnostics,
  LauncherSearchIndexState,
  LauncherSearchResponse,
  LauncherSearchSettings,
  LauncherStatus,
  LauncherUpdateSearchSettingsInput,
};

export async function launcherSearch(query: string, limit?: number): Promise<LauncherSearchResponse> {
  return invokeLauncher<LauncherSearchResponse>({ kind: "search", payload: { query, limit } });
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

export async function launcherGetStatus(): Promise<LauncherStatus> {
  return invokeLauncher<LauncherStatus>({ kind: "get_status" });
}

export async function launcherRebuildIndex(): Promise<LauncherRebuildResult> {
  return invokeLauncher<LauncherRebuildResult>({ kind: "rebuild_index" });
}

export async function launcherResetSearchSettings(): Promise<LauncherSearchSettings> {
  return invokeLauncher<LauncherSearchSettings>({ kind: "reset_search_settings" });
}

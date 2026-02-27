import type {
  ActionResultDto as ActionResult,
  CommandRequestDto,
  ResourceHistoryDto as ResourceHistory,
  ResourceSnapshotDto as ResourceSnapshot,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export type { ResourceHistory, ResourceSnapshot };

function invokeResourceMonitor<T>(kind: string, payload?: Record<string, unknown>): Promise<T> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload;
  }
  return invokeWithLog<T>("resource_monitor_handle", { request });
}

export async function fetchResourceMonitorSnapshot(): Promise<ResourceSnapshot> {
  return invokeResourceMonitor<ResourceSnapshot>("snapshot");
}

export async function fetchResourceMonitorHistory(limit?: number): Promise<ResourceHistory> {
  return invokeResourceMonitor<ResourceHistory>("history", {
    limit: typeof limit === "number" && Number.isFinite(limit) && limit > 0 ? Math.floor(limit) : null,
  });
}

export async function resetResourceMonitorSession(): Promise<ActionResult> {
  return invokeResourceMonitor<ActionResult>("reset_session");
}

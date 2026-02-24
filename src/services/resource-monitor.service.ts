import type {
  ActionResultDto as ActionResult,
  ResourceHistoryDto as ResourceHistory,
  ResourceSnapshotDto as ResourceSnapshot,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export type { ResourceHistory, ResourceSnapshot };

export async function fetchResourceMonitorSnapshot(): Promise<ResourceSnapshot> {
  return invokeWithLog<ResourceSnapshot>("resource_monitor_snapshot");
}

export async function fetchResourceMonitorHistory(limit?: number): Promise<ResourceHistory> {
  return invokeWithLog<ResourceHistory>("resource_monitor_history", {
    limit: typeof limit === "number" && Number.isFinite(limit) && limit > 0 ? Math.floor(limit) : null,
  });
}

export async function resetResourceMonitorSession(): Promise<ActionResult> {
  return invokeWithLog<ActionResult>("resource_monitor_reset_session");
}

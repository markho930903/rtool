import type {
  AppRuntimeInfoDto as AppRuntimeInfo,
  DashboardSnapshotDto as DashboardSnapshot,
  SystemInfoDto as SystemInfo,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export type { AppRuntimeInfo, DashboardSnapshot, SystemInfo };

export async function fetchDashboardSnapshot(): Promise<DashboardSnapshot> {
  return invokeWithLog<DashboardSnapshot>("dashboard_snapshot");
}

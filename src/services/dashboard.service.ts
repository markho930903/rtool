import type {
  AppHealthSnapshotDto as AppHealthSnapshot,
  AppRuntimeInfoDto as AppRuntimeInfo,
  DashboardSnapshotDto as DashboardSnapshot,
  SystemInfoDto as SystemInfo,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export type { AppHealthSnapshot, AppRuntimeInfo, DashboardSnapshot, SystemInfo };

export async function fetchDashboardSnapshot(): Promise<DashboardSnapshot> {
  return invokeWithLog<DashboardSnapshot>("dashboard_snapshot");
}

export async function fetchAppHealthSnapshot(): Promise<AppHealthSnapshot> {
  return invokeWithLog<AppHealthSnapshot>("app_get_health_snapshot");
}

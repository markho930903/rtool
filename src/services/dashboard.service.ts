import type {
  AppHealthSnapshotDto as AppHealthSnapshot,
  CommandRequestDto,
  AppRuntimeInfoDto as AppRuntimeInfo,
  DashboardSnapshotDto as DashboardSnapshot,
  SystemInfoDto as SystemInfo,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export type { AppHealthSnapshot, AppRuntimeInfo, DashboardSnapshot, SystemInfo };

function invokeDashboard<T>(kind: string): Promise<T> {
  const request: CommandRequestDto = { kind };
  return invokeWithLog<T>("dashboard_handle", { request });
}

export async function fetchDashboardSnapshot(): Promise<DashboardSnapshot> {
  return invokeDashboard<DashboardSnapshot>("snapshot");
}

export async function fetchAppHealthSnapshot(): Promise<AppHealthSnapshot> {
  return invokeDashboard<AppHealthSnapshot>("health_snapshot");
}

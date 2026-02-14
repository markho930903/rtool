import { invokeWithLog } from "@/services/invoke";

export interface AppRuntimeInfo {
  appName: string;
  appVersion: string;
  buildMode: string;
  uptimeSeconds: number;
  processMemoryBytes: number | null;
  databaseSizeBytes: number | null;
}

export interface SystemInfo {
  osName: string | null;
  osVersion: string | null;
  kernelVersion: string | null;
  arch: string | null;
  hostName: string | null;
  cpuBrand: string | null;
  cpuCores: number | null;
  totalMemoryBytes: number | null;
  usedMemoryBytes: number | null;
}

export interface DashboardSnapshot {
  sampledAt: number;
  app: AppRuntimeInfo;
  system: SystemInfo;
}

export async function fetchDashboardSnapshot(): Promise<DashboardSnapshot> {
  return invokeWithLog<DashboardSnapshot>("dashboard_snapshot");
}

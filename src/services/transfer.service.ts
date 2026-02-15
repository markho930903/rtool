import { invokeWithLog } from "@/services/invoke";
import type {
  TransferFileInput,
  TransferHistoryFilter,
  TransferHistoryPage,
  TransferPairingCode,
  TransferPeer,
  TransferSession,
  TransferSettings,
} from "@/components/transfer/types";

export interface TransferUpdateSettingsInput {
  defaultDownloadDir?: string;
  maxParallelFiles?: number;
  maxInflightChunks?: number;
  chunkSizeKb?: number;
  autoCleanupDays?: number;
  resumeEnabled?: boolean;
  discoveryEnabled?: boolean;
  pairingRequired?: boolean;
}

export interface TransferSendFilesInput {
  peerDeviceId: string;
  pairCode: string;
  files: TransferFileInput[];
  direction?: string;
  sessionId?: string;
}

export async function transferGetSettings(): Promise<TransferSettings> {
  return invokeWithLog<TransferSettings>("transfer_get_settings");
}

export async function transferUpdateSettings(input: TransferUpdateSettingsInput): Promise<TransferSettings> {
  return invokeWithLog<TransferSettings>("transfer_update_settings", { input });
}

export async function transferGeneratePairingCode(): Promise<TransferPairingCode> {
  return invokeWithLog<TransferPairingCode>("transfer_generate_pairing_code");
}

export async function transferStartDiscovery(): Promise<void> {
  await invokeWithLog("transfer_start_discovery");
}

export async function transferStopDiscovery(): Promise<void> {
  await invokeWithLog("transfer_stop_discovery");
}

export async function transferListPeers(): Promise<TransferPeer[]> {
  return invokeWithLog<TransferPeer[]>("transfer_list_peers");
}

export async function transferSendFiles(input: TransferSendFilesInput): Promise<TransferSession> {
  return invokeWithLog<TransferSession>("transfer_send_files", { input });
}

export async function transferPauseSession(sessionId: string): Promise<void> {
  await invokeWithLog("transfer_pause_session", { sessionId });
}

export async function transferResumeSession(sessionId: string): Promise<void> {
  await invokeWithLog("transfer_resume_session", { sessionId });
}

export async function transferCancelSession(sessionId: string): Promise<void> {
  await invokeWithLog("transfer_cancel_session", { sessionId });
}

export async function transferRetrySession(sessionId: string): Promise<TransferSession> {
  return invokeWithLog<TransferSession>("transfer_retry_session", { sessionId });
}

export async function transferListHistory(filter?: TransferHistoryFilter): Promise<TransferHistoryPage> {
  return invokeWithLog<TransferHistoryPage>("transfer_list_history", {
    filter,
  });
}

export async function transferClearHistory(all = false, olderThanDays = 30): Promise<void> {
  await invokeWithLog("transfer_clear_history", {
    input: {
      all,
      olderThanDays,
    },
  });
}

export async function transferOpenDownloadDir(path?: string): Promise<void> {
  await invokeWithLog("transfer_open_download_dir", { path });
}

import type {
  TransferHistoryFilterDto,
  TransferHistoryPageDto,
  TransferPairingCodeDto,
  TransferPeerDto,
  TransferSendFilesInputDto,
  TransferSessionDto,
  TransferSettingsDto,
  TransferUpdateSettingsInputDto,
} from "@/contracts";
import type {
  TransferDirection,
  TransferFileInput,
  TransferHistoryFilter,
  TransferHistoryPage,
  TransferPairingCode,
  TransferPeer,
  TransferSession,
  TransferSettings,
} from "@/components/transfer/types";
import { invokeWithLog } from "@/services/invoke";

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
  direction?: TransferDirection;
  sessionId?: string;
}

export async function transferGetSettings(): Promise<TransferSettings> {
  const dto = await invokeWithLog<TransferSettingsDto>("transfer_get_settings");
  return dto as TransferSettings;
}

export async function transferUpdateSettings(input: TransferUpdateSettingsInput): Promise<TransferSettings> {
  const dto = await invokeWithLog<TransferSettingsDto>("transfer_update_settings", {
    input: input as TransferUpdateSettingsInputDto,
  });
  return dto as TransferSettings;
}

export async function transferGeneratePairingCode(): Promise<TransferPairingCode> {
  const dto = await invokeWithLog<TransferPairingCodeDto>("transfer_generate_pairing_code");
  return dto as TransferPairingCode;
}

export async function transferStartDiscovery(): Promise<void> {
  await invokeWithLog("transfer_start_discovery");
}

export async function transferStopDiscovery(): Promise<void> {
  await invokeWithLog("transfer_stop_discovery");
}

export async function transferListPeers(): Promise<TransferPeer[]> {
  const dto = await invokeWithLog<TransferPeerDto[]>("transfer_list_peers");
  return dto as TransferPeer[];
}

export async function transferSendFiles(input: TransferSendFilesInput): Promise<TransferSession> {
  const dto = await invokeWithLog<TransferSessionDto>("transfer_send_files", {
    input: input as TransferSendFilesInputDto,
  });
  return dto as TransferSession;
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
  const dto = await invokeWithLog<TransferSessionDto>("transfer_retry_session", { sessionId });
  return dto as TransferSession;
}

export async function transferListHistory(filter?: TransferHistoryFilter): Promise<TransferHistoryPage> {
  const dto = await invokeWithLog<TransferHistoryPageDto>("transfer_list_history", {
    filter: filter as TransferHistoryFilterDto | undefined,
  });
  return dto as TransferHistoryPage;
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

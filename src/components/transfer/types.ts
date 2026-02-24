export interface TransferSettings {
  defaultDownloadDir: string;
  maxParallelFiles: number;
  maxInflightChunks: number;
  chunkSizeKb: number;
  autoCleanupDays: number;
  resumeEnabled: boolean;
  discoveryEnabled: boolean;
  pairingRequired: boolean;
}

export type TransferPeerTrustLevel = "online" | "trusted" | "other";

export interface TransferPeer {
  deviceId: string;
  displayName: string;
  address: string;
  listenPort: number;
  lastSeenAt: number;
  pairedAt: number | null;
  trustLevel: TransferPeerTrustLevel;
  failedAttempts: number;
  blockedUntil: number | null;
  pairingRequired: boolean;
  online: boolean;
}

export interface TransferFileInput {
  path: string;
  relativePath?: string;
  compressFolder?: boolean;
}

export type TransferDirection = "send" | "receive";

export type TransferStatus = "queued" | "running" | "paused" | "failed" | "interrupted" | "canceled" | "success";

export function isTransferRunningLikeStatus(status: TransferStatus): boolean {
  return status === "queued" || status === "running";
}

export function isTransferRetryableStatus(status: TransferStatus): boolean {
  return status === "failed" || status === "interrupted" || status === "canceled";
}

export interface TransferFile {
  id: string;
  sessionId: string;
  relativePath: string;
  sourcePath: string | null;
  targetPath: string | null;
  sizeBytes: number;
  transferredBytes: number;
  chunkSize: number;
  chunkCount: number;
  status: TransferStatus;
  blake3: string | null;
  mimeType: string | null;
  previewKind: string | null;
  previewData: string | null;
  isFolderArchive: boolean;
}

export interface TransferSession {
  id: string;
  direction: TransferDirection;
  peerDeviceId: string;
  peerName: string;
  status: TransferStatus;
  totalBytes: number;
  transferredBytes: number;
  avgSpeedBps: number;
  saveDir: string;
  createdAt: number;
  startedAt: number | null;
  finishedAt: number | null;
  errorCode: string | null;
  errorMessage: string | null;
  cleanupAfterAt: number | null;
  files: TransferFile[];
}

export interface TransferProgressSnapshot {
  session: TransferSession;
  activeFileId: string | null;
  speedBps: number;
  etaSeconds: number | null;
  protocolVersion?: number;
  codec?: "bin";
  inflightChunks?: number;
  retransmitChunks?: number;
}

export interface TransferHistoryFilter {
  cursor?: string;
  limit?: number;
  status?: TransferStatus;
  peerDeviceId?: string;
}

export interface TransferHistoryPage {
  items: TransferSession[];
  nextCursor: string | null;
}

export interface TransferPairingCode {
  code: string;
  expiresAt: number;
}

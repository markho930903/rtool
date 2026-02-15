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

export interface TransferPeer {
  deviceId: string;
  displayName: string;
  address: string;
  listenPort: number;
  lastSeenAt: number;
  pairedAt: number | null;
  trustLevel: string;
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
  status: string;
  blake3: string | null;
  mimeType: string | null;
  previewKind: string | null;
  previewData: string | null;
  isFolderArchive: boolean;
}

export interface TransferSession {
  id: string;
  direction: string;
  peerDeviceId: string;
  peerName: string;
  status: string;
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
  codec?: "json-v1" | "bin-v2";
  inflightChunks?: number;
  retransmitChunks?: number;
}

export interface TransferHistoryFilter {
  cursor?: string;
  limit?: number;
  status?: string;
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

import { create } from "zustand";

import type {
  TransferFileInput,
  TransferHistoryPage,
  TransferPairingCode,
  TransferPeer,
  TransferProgressSnapshot,
  TransferSession,
  TransferSettings,
} from "@/components/transfer/types";
import {
  transferCancelSession,
  transferClearHistory,
  transferGeneratePairingCode,
  transferGetSettings,
  transferListHistory,
  transferListPeers,
  transferOpenDownloadDir,
  transferPauseSession,
  transferResumeSession,
  transferRetrySession,
  transferSendFiles,
  transferStartDiscovery,
  transferStopDiscovery,
  transferUpdateSettings,
} from "@/services/transfer.service";

interface TransferState {
  initialized: boolean;
  loading: boolean;
  error: string | null;
  settings: TransferSettings | null;
  peers: TransferPeer[];
  sessions: TransferSession[];
  history: TransferSession[];
  nextCursor: string | null;
  selectedPeerId: string;
  pairCode: string;
  pendingFiles: TransferFileInput[];
  pairingCode: TransferPairingCode | null;
}

interface TransferActions {
  initialize: () => Promise<void>;
  dispose: () => Promise<void>;
  refreshPeers: () => Promise<void>;
  refreshHistory: () => Promise<void>;
  setSelectedPeerId: (peerDeviceId: string) => void;
  setPairCode: (pairCode: string) => void;
  setPendingFiles: (files: TransferFileInput[]) => void;
  generatePairingCode: () => Promise<void>;
  sendPendingFiles: () => Promise<void>;
  pauseSession: (sessionId: string) => Promise<void>;
  resumeSession: (sessionId: string) => Promise<void>;
  cancelSession: (sessionId: string) => Promise<void>;
  retrySession: (sessionId: string) => Promise<void>;
  clearHistory: () => Promise<void>;
  openDownloadDir: () => Promise<void>;
  updateSettings: (input: Partial<TransferSettings>) => Promise<void>;
  applyPeerSync: (peers: TransferPeer[]) => void;
  applySessionSync: (snapshot: TransferProgressSnapshot) => void;
  removeHistoryBySessionId: (sessionId: string) => void;
}

type TransferStore = TransferState & TransferActions;

function upsertSession(sessions: TransferSession[], session: TransferSession): TransferSession[] {
  const next = [...sessions];
  const index = next.findIndex((item) => item.id === session.id);
  if (index === -1) {
    next.unshift(session);
    next.sort((left, right) => right.createdAt - left.createdAt);
    return next;
  }
  next[index] = session;
  return next;
}

export const useTransferStore = create<TransferStore>((set, get) => ({
  initialized: false,
  loading: false,
  error: null,
  settings: null,
  peers: [],
  sessions: [],
  history: [],
  nextCursor: null,
  selectedPeerId: "",
  pairCode: "",
  pendingFiles: [],
  pairingCode: null,

  async initialize() {
    if (get().initialized) {
      return;
    }

    set({ loading: true, error: null });
    try {
      const [settings, peers, historyPage] = await Promise.all([
        transferGetSettings(),
        transferListPeers(),
        transferListHistory({ limit: 30 }),
      ]);

      await transferStartDiscovery();

      set({
        initialized: true,
        loading: false,
        settings,
        peers,
        history: historyPage.items,
        nextCursor: historyPage.nextCursor,
        selectedPeerId: peers[0]?.deviceId ?? "",
      });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  },

  async dispose() {
    await transferStopDiscovery();
    set({ initialized: false });
  },

  async refreshPeers() {
    try {
      const peers = await transferListPeers();
      set((state) => ({
        peers,
        selectedPeerId:
          state.selectedPeerId && peers.some((peer) => peer.deviceId === state.selectedPeerId)
            ? state.selectedPeerId
            : (peers[0]?.deviceId ?? ""),
      }));
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async refreshHistory() {
    try {
      const page: TransferHistoryPage = await transferListHistory({ limit: 30 });
      set({ history: page.items, nextCursor: page.nextCursor });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  setSelectedPeerId(selectedPeerId) {
    set({ selectedPeerId });
  },

  setPairCode(pairCode) {
    set({ pairCode });
  },

  setPendingFiles(pendingFiles) {
    set({ pendingFiles });
  },

  async generatePairingCode() {
    try {
      const pairingCode = await transferGeneratePairingCode();
      set({ pairingCode, pairCode: pairingCode.code });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async sendPendingFiles() {
    const state = get();
    if (!state.selectedPeerId) {
      set({ error: "请选择目标设备" });
      return;
    }
    if (!state.pairCode.trim()) {
      set({ error: "请输入配对码" });
      return;
    }
    if (state.pendingFiles.length === 0) {
      set({ error: "请先选择文件" });
      return;
    }

    set({ error: null });
    try {
      const session = await transferSendFiles({
        peerDeviceId: state.selectedPeerId,
        pairCode: state.pairCode.trim(),
        files: state.pendingFiles,
        direction: "send",
      });

      set((prev) => ({
        sessions: upsertSession(prev.sessions, session),
      }));
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async pauseSession(sessionId) {
    try {
      await transferPauseSession(sessionId);
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async resumeSession(sessionId) {
    try {
      await transferResumeSession(sessionId);
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async cancelSession(sessionId) {
    try {
      await transferCancelSession(sessionId);
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async retrySession(sessionId) {
    try {
      const session = await transferRetrySession(sessionId);
      set((state) => ({ sessions: upsertSession(state.sessions, session) }));
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async clearHistory() {
    try {
      await transferClearHistory(false, get().settings?.autoCleanupDays ?? 30);
      await get().refreshHistory();
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async openDownloadDir() {
    const path = get().settings?.defaultDownloadDir;
    try {
      await transferOpenDownloadDir(path);
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  async updateSettings(input) {
    const current = get().settings;
    if (!current) {
      return;
    }

    try {
      const settings = await transferUpdateSettings({
        defaultDownloadDir: input.defaultDownloadDir ?? current.defaultDownloadDir,
        maxParallelFiles: input.maxParallelFiles ?? current.maxParallelFiles,
        maxInflightChunks: input.maxInflightChunks ?? current.maxInflightChunks,
        chunkSizeKb: input.chunkSizeKb ?? current.chunkSizeKb,
        autoCleanupDays: input.autoCleanupDays ?? current.autoCleanupDays,
        resumeEnabled: input.resumeEnabled ?? current.resumeEnabled,
        discoveryEnabled: input.discoveryEnabled ?? current.discoveryEnabled,
        pairingRequired: input.pairingRequired ?? current.pairingRequired,
      });
      set({ settings });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
    }
  },

  applyPeerSync(peers) {
    set((state) => ({
      peers,
      selectedPeerId:
        state.selectedPeerId && peers.some((peer) => peer.deviceId === state.selectedPeerId)
          ? state.selectedPeerId
          : (peers[0]?.deviceId ?? ""),
    }));
  },

  applySessionSync(snapshot) {
    set((state) => ({
      sessions: upsertSession(state.sessions, snapshot.session),
      history: upsertSession(state.history, snapshot.session),
    }));
  },

  removeHistoryBySessionId(sessionId) {
    set((state) => ({ history: state.history.filter((item) => item.id !== sessionId) }));
  },
}));

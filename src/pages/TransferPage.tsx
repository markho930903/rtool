import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import TransferDropzone from "@/components/transfer/TransferDropzone";
import TransferHistoryPanel from "@/components/transfer/TransferHistoryPanel";
import TransferPeerPanel from "@/components/transfer/TransferPeerPanel";
import TransferSessionList from "@/components/transfer/TransferSessionList";
import { Button, Input } from "@/components/ui";
import { useTransferStore } from "@/stores/transfer.store";

export default function TransferPage() {
  const { t } = useTranslation("transfer");

  const initialized = useTransferStore((state) => state.initialized);
  const loading = useTransferStore((state) => state.loading);
  const error = useTransferStore((state) => state.error);
  const settings = useTransferStore((state) => state.settings);
  const peers = useTransferStore((state) => state.peers);
  const selectedPeerId = useTransferStore((state) => state.selectedPeerId);
  const pairCode = useTransferStore((state) => state.pairCode);
  const pendingFiles = useTransferStore((state) => state.pendingFiles);
  const pairingCode = useTransferStore((state) => state.pairingCode);
  const sessions = useTransferStore((state) => state.sessions);
  const history = useTransferStore((state) => state.history);

  const initialize = useTransferStore((state) => state.initialize);
  const dispose = useTransferStore((state) => state.dispose);
  const setSelectedPeerId = useTransferStore((state) => state.setSelectedPeerId);
  const setPairCode = useTransferStore((state) => state.setPairCode);
  const setPendingFiles = useTransferStore((state) => state.setPendingFiles);
  const sendPendingFiles = useTransferStore((state) => state.sendPendingFiles);
  const refreshPeers = useTransferStore((state) => state.refreshPeers);
  const refreshHistory = useTransferStore((state) => state.refreshHistory);
  const clearHistory = useTransferStore((state) => state.clearHistory);
  const openDownloadDir = useTransferStore((state) => state.openDownloadDir);
  const generatePairingCode = useTransferStore((state) => state.generatePairingCode);
  const updateSettings = useTransferStore((state) => state.updateSettings);

  const [downloadDirInput, setDownloadDirInput] = useState("");
  const [autoCleanupDaysInput, setAutoCleanupDaysInput] = useState("30");

  useEffect(() => {
    if (!settings) {
      return;
    }
    setDownloadDirInput(settings.defaultDownloadDir);
    setAutoCleanupDaysInput(String(settings.autoCleanupDays));
  }, [settings]);

  const pauseSession = useTransferStore((state) => state.pauseSession);
  const resumeSession = useTransferStore((state) => state.resumeSession);
  const cancelSession = useTransferStore((state) => state.cancelSession);
  const retrySession = useTransferStore((state) => state.retrySession);

  useEffect(() => {
    void initialize();
    return () => {
      void dispose();
    };
  }, [dispose, initialize]);

  return (
    <div className="space-y-4">
      <header className="rounded-4 border border-border-muted bg-surface p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h1 className="ui-section-title">{t("page.title")}</h1>
            <p className="mt-1 text-sm text-text-secondary">{t("page.subtitle")}</p>
            {settings ? (
              <p className="mt-1 text-xs text-text-secondary">
                {t("page.downloadDir")}: {settings.defaultDownloadDir}
              </p>
            ) : null}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              type="button"
              size="sm"
              variant="secondary"
              className="text-ui-xs"
              onClick={() => {
                void generatePairingCode();
              }}
            >
              {t("page.generatePairCode")}
            </Button>
            <Button
              type="button"
              size="sm"
              variant="secondary"
              className="text-ui-xs"
              onClick={() => {
                void openDownloadDir();
              }}
            >
              {t("page.openDownloadDir")}
            </Button>
          </div>
        </div>

        {pairingCode ? (
          <div className="mt-2 text-xs text-text-secondary">
            {t("page.currentPairCode")}: <span className="font-semibold text-text-primary">{pairingCode.code}</span>
          </div>
        ) : null}

        {settings ? (
          <div className="mt-3 grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_120px_auto]">
            <Input
              variant="tool"
              className="bg-elevated text-ui-xs"
              value={downloadDirInput}
              onChange={(event) => setDownloadDirInput(event.currentTarget.value)}
              placeholder={t("page.downloadDir")}
            />
            <Input
              variant="tool"
              className="bg-elevated text-ui-xs"
              value={autoCleanupDaysInput}
              onChange={(event) => setAutoCleanupDaysInput(event.currentTarget.value)}
              placeholder={t("page.autoCleanupDays")}
            />
            <Button
              type="button"
              size="sm"
              variant="secondary"
              className="text-ui-xs"
              onClick={() => {
                const parsedDays = Number.parseInt(autoCleanupDaysInput, 10);
                void updateSettings({
                  defaultDownloadDir: downloadDirInput.trim(),
                  autoCleanupDays: Number.isFinite(parsedDays) ? Math.max(1, parsedDays) : settings.autoCleanupDays,
                });
              }}
            >
              {t("page.saveSettings")}
            </Button>
          </div>
        ) : null}

        {error ? <div className="mt-2 text-xs text-danger">{error}</div> : null}
      </header>

      {!initialized && loading ? <p className="text-xs text-text-secondary">{t("page.loading")}</p> : null}

      <div className="grid grid-cols-1 gap-3 xl:grid-cols-[280px_minmax(0,1fr)_320px]">
        <TransferPeerPanel
          peers={peers}
          selectedPeerId={selectedPeerId}
          pairCode={pairCode}
          onSelectPeer={setSelectedPeerId}
          onPairCodeChange={setPairCode}
          onRefresh={refreshPeers}
        />

        <div className="space-y-3">
          <TransferDropzone
            pendingFiles={pendingFiles}
            onChangeFiles={setPendingFiles}
            onSend={sendPendingFiles}
          />
          <TransferSessionList
            sessions={sessions}
            onPause={pauseSession}
            onResume={resumeSession}
            onCancel={cancelSession}
            onRetry={retrySession}
          />
        </div>

        <TransferHistoryPanel history={history} onRefresh={refreshHistory} onClear={clearHistory} />
      </div>
    </div>
  );
}

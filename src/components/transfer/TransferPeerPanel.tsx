import { useTranslation } from "react-i18next";

import { Button, Input, Select } from "@/components/ui";
import type { TransferPeer } from "@/components/transfer/types";

interface TransferPeerPanelProps {
  peers: TransferPeer[];
  selectedPeerId: string;
  pairCode: string;
  onSelectPeer: (peerDeviceId: string) => void;
  onPairCodeChange: (value: string) => void;
  onRefresh: () => Promise<void>;
}

export default function TransferPeerPanel(props: TransferPeerPanelProps) {
  const { t } = useTranslation("transfer");

  const peerOptions = props.peers.map((peer) => ({
    value: peer.deviceId,
    label: `${peer.displayName}${peer.online ? "" : t("peer.offlineSuffix")}`,
  }));

  return (
    <section className="rounded-4 border border-border-muted bg-surface p-4">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-text-primary">{t("peer.title")}</h2>
        <Button
          type="button"
          size="xs"
          variant="secondary"
          className="text-ui-xs"
          onClick={() => {
            void props.onRefresh();
          }}
        >
          {t("peer.refresh")}
        </Button>
      </div>

      <div className="mt-3 space-y-2">
        <label className="text-xs text-text-secondary" htmlFor="transfer-peer-select">
          {t("peer.select")}
        </label>
        <Select
          id="transfer-peer-select"
          value={props.selectedPeerId}
          options={peerOptions}
          onChange={(event) => props.onSelectPeer(event.currentTarget.value)}
        />
      </div>

      <div className="mt-3 space-y-2">
        <label className="text-xs text-text-secondary" htmlFor="transfer-pair-code-input">
          {t("peer.pairCode")}
        </label>
        <Input
          id="transfer-pair-code-input"
          variant="tool"
          className="w-full bg-elevated"
          value={props.pairCode}
          onChange={(event) => props.onPairCodeChange(event.currentTarget.value)}
          placeholder={t("peer.pairCodePlaceholder")}
        />
      </div>

      <div className="mt-4 max-h-60 overflow-auto space-y-1 rounded-2 border border-border-muted p-2">
        {props.peers.length === 0 ? <p className="text-xs text-text-secondary">{t("peer.empty")}</p> : null}
        {props.peers.map((peer) => (
          <div key={peer.deviceId} className="rounded-2 px-2 py-1.5 text-xs text-text-secondary">
            <div className="font-medium text-text-primary">{peer.displayName}</div>
            <div>
              {peer.address}:{peer.listenPort}
            </div>
            <div>{peer.online ? t("peer.online") : t("peer.offline")}</div>
          </div>
        ))}
      </div>
    </section>
  );
}

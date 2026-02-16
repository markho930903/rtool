import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui";
import type { TransferSession } from "@/components/transfer/types";

interface TransferSessionListProps {
  sessions: TransferSession[];
  onPause: (sessionId: string) => Promise<void>;
  onResume: (sessionId: string) => Promise<void>;
  onCancel: (sessionId: string) => Promise<void>;
  onRetry: (sessionId: string) => Promise<void>;
}

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = value;
  let index = 0;
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024;
    index += 1;
  }

  return `${size.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

function progressPercent(session: TransferSession): number {
  if (session.totalBytes <= 0) {
    return 0;
  }
  return Math.min(100, Math.round((session.transferredBytes / session.totalBytes) * 100));
}

export default function TransferSessionList(props: TransferSessionListProps) {
  const { t } = useTranslation("transfer");

  return (
    <section className="rounded-4 border border-border-muted bg-surface p-4">
      <h2 className="text-sm font-semibold text-text-primary">{t("session.title")}</h2>

      <div className="mt-3 max-h-[28rem] space-y-2 overflow-auto">
        {props.sessions.length === 0 ? (
          <p className="text-xs text-text-secondary">{t("session.empty")}</p>
        ) : null}

        {props.sessions.map((session) => {
          const percent = progressPercent(session);
          const isRunning = session.status === "running" || session.status === "queued";
          const isPaused = session.status === "paused";
          const canRetry = session.status === "failed" || session.status === "interrupted" || session.status === "canceled";

          return (
            <article key={session.id} className="rounded-3 border border-border-muted p-3">
              <div className="flex items-center justify-between gap-2">
                <div className="truncate text-xs font-semibold text-text-primary">{session.peerName}</div>
                <div className="text-[10px] uppercase tracking-wide text-text-secondary">{session.status}</div>
              </div>

              <div className="mt-1 text-[11px] text-text-secondary">
                {formatBytes(session.transferredBytes)} / {formatBytes(session.totalBytes)}
                {session.avgSpeedBps > 0 ? ` · ${formatBytes(session.avgSpeedBps)}/s` : ""}
              </div>

              <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-border-muted">
                <div className="h-full rounded-full bg-accent transition-all duration-200" style={{ width: `${percent}%` }} />
              </div>

              <div className="mt-2 flex flex-wrap gap-1">
                {isRunning ? (
                  <Button
                    type="button"
                    size="xs"
                    variant="secondary"
                    className="ui-text-micro"
                    onClick={() => {
                      void props.onPause(session.id);
                    }}
                  >
                    {t("session.pause")}
                  </Button>
                ) : null}

                {isPaused ? (
                  <Button
                    type="button"
                    size="xs"
                    variant="secondary"
                    className="ui-text-micro"
                    onClick={() => {
                      void props.onResume(session.id);
                    }}
                  >
                    {t("session.resume")}
                  </Button>
                ) : null}

                {(isRunning || isPaused) ? (
                  <Button
                    type="button"
                    size="xs"
                    variant="secondary"
                    className="ui-text-micro"
                    onClick={() => {
                      void props.onCancel(session.id);
                    }}
                  >
                    {t("session.cancel")}
                  </Button>
                ) : null}

                {canRetry ? (
                  <Button
                    type="button"
                    size="xs"
                    variant="secondary"
                    className="ui-text-micro"
                    onClick={() => {
                      void props.onRetry(session.id);
                    }}
                  >
                    {t("session.retry")}
                  </Button>
                ) : null}
              </div>

              {session.errorMessage ? (
                <div className="mt-2 text-[11px] text-danger">{session.errorMessage}</div>
              ) : null}
            </article>
          );
        })}
      </div>
    </section>
  );
}

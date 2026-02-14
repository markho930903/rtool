import dayjs from "dayjs";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Input } from "@/components/ui";

function normalizeTimestamp(value: string): number {
  const numeric = Number(value.trim());
  if (!Number.isFinite(numeric)) {
    return Number.NaN;
  }

  if (numeric > 10_000_000_000) {
    return numeric;
  }

  return numeric * 1000;
}

export default function TimestampTool() {
  const { t } = useTranslation("tools");
  const [input, setInput] = useState(() => String(Date.now()));

  const now = useMemo(() => Date.now(), []);
  const converted = useMemo(() => {
    const timestamp = normalizeTimestamp(input);
    if (Number.isNaN(timestamp)) {
      return null;
    }

    return {
      local: dayjs(timestamp).format("YYYY-MM-DD HH:mm:ss"),
      iso: dayjs(timestamp).toISOString(),
      seconds: Math.floor(timestamp / 1000),
      milliseconds: timestamp,
    };
  }, [input]);

  return (
    <article className="flex flex-col gap-2.5 rounded-lg border border-border-muted bg-surface-soft p-3">
      <header className="flex items-center justify-between gap-2">
        <h3 className="m-0 text-sm font-semibold text-text-primary">{t("timestamp.title")}</h3>
      </header>

      <Input variant="tool" value={input} onChange={(event) => setInput(event.currentTarget.value)} />
      <div className="text-xs text-text-muted">{t("timestamp.now", { value: now })}</div>

      {!converted ? (
        <div className="text-xs text-danger">{t("timestamp.invalid")}</div>
      ) : (
        <ul className="m-0 list-disc pl-[18px] text-xs text-text-secondary">
          <li>{t("timestamp.local", { value: converted.local })}</li>
          <li>{t("timestamp.iso", { value: converted.iso })}</li>
          <li>{t("timestamp.seconds", { value: converted.seconds })}</li>
          <li>{t("timestamp.milliseconds", { value: converted.milliseconds })}</li>
        </ul>
      )}
    </article>
  );
}

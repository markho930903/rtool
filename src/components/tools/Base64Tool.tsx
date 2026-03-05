import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button, Textarea } from "@/components/ui";

function utf8ToBase64(value: string): string {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return window.btoa(binary);
}

function base64ToUtf8(value: string): string {
  const binary = window.atob(value);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

export default function Base64Tool() {
  const { t } = useTranslation("tools");
  const [input, setInput] = useState("hello rtool");
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);

  const onEncode = () => {
    setError(null);
    try {
      setOutput(utf8ToBase64(input));
    } catch (encodeError) {
      const message = encodeError instanceof Error ? encodeError.message : String(encodeError);
      setError(message);
    }
  };

  const onDecode = () => {
    setError(null);
    try {
      setOutput(base64ToUtf8(input));
    } catch (decodeError) {
      const message = decodeError instanceof Error ? decodeError.message : String(decodeError);
      setError(message);
    }
  };

  return (
    <article className="flex flex-col gap-2.5 rounded-lg border border-border-glass bg-surface-glass-soft p-3 shadow-inset-soft">
      <header className="flex items-center justify-between gap-2">
        <h3 className="m-0 text-sm font-semibold text-text-primary">Base64</h3>
      </header>

      <Textarea variant="tool" value={input} onChange={(event) => setInput(event.currentTarget.value)} />

      <div className="flex gap-2">
        <Button size="xs" variant="secondary" onClick={onEncode}>
          <span className="btn-icon i-noto:up-arrow" aria-hidden="true" />
          <span>{t("base64.encode")}</span>
        </Button>
        <Button size="xs" variant="secondary" onClick={onDecode}>
          <span className="btn-icon i-noto:down-arrow" aria-hidden="true" />
          <span>{t("base64.decode")}</span>
        </Button>
      </div>

      {error ? (
        <div className="text-xs text-danger">{error}</div>
      ) : (
        <pre className="m-0 max-h-[180px] overflow-auto whitespace-pre-wrap break-words text-xs text-text-secondary">
          {output || t("base64.waitResult")}
        </pre>
      )}
    </article>
  );
}

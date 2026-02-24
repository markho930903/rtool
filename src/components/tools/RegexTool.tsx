import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Input, Textarea } from "@/components/ui";

export default function RegexTool() {
  const { t } = useTranslation("tools");
  const [pattern, setPattern] = useState("rtool");
  const [flags, setFlags] = useState("gi");
  const [source, setSource] = useState(() => t("regex.defaultSource"));
  const [replacement, setReplacement] = useState("RTOOL");

  const analysis = useMemo(() => {
    try {
      const regexp = new RegExp(pattern, flags);
      const matches = source.match(regexp) ?? [];
      const replaced = source.replace(regexp, replacement);
      return {
        error: null,
        matches,
        replaced,
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return {
        error: message,
        matches: [] as string[],
        replaced: source,
      };
    }
  }, [flags, pattern, replacement, source]);

  return (
    <article className="flex flex-col gap-2.5 rounded-lg border border-border-glass bg-surface-glass-soft p-3 shadow-inset-soft">
      <header className="flex items-center justify-between gap-2">
        <h3 className="m-0 text-sm font-semibold text-text-primary">{t("regex.title")}</h3>
      </header>

      <div className="grid grid-cols-2 gap-2">
        <Input
          variant="tool"
          value={pattern}
          onChange={(event) => setPattern(event.currentTarget.value)}
          placeholder="pattern"
        />
        <Input
          variant="tool"
          value={flags}
          onChange={(event) => setFlags(event.currentTarget.value)}
          placeholder="flags"
        />
      </div>

      <Textarea variant="tool" value={source} onChange={(event) => setSource(event.currentTarget.value)} />
      <Input
        variant="tool"
        value={replacement}
        onChange={(event) => setReplacement(event.currentTarget.value)}
        placeholder={t("regex.replacementPlaceholder")}
      />

      {analysis.error ? <div className="text-xs text-danger">{analysis.error}</div> : null}

      <div className="text-xs text-text-muted">{t("regex.matchCount", { count: analysis.matches.length })}</div>
      <pre className="m-0 max-h-[180px] overflow-auto whitespace-pre-wrap break-words text-xs text-text-secondary">
        {analysis.replaced}
      </pre>
    </article>
  );
}

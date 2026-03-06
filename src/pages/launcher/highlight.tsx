import type { ReactNode } from "react";

export interface HighlightContext {
  matcher: RegExp | null;
  tokenSet: Set<string>;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function createHighlightContext(query: string): HighlightContext {
  const tokens = query
    .trim()
    .split(/\s+/)
    .map((token) => token.trim())
    .filter(Boolean);

  if (tokens.length === 0) {
    return {
      matcher: null,
      tokenSet: new Set(),
    };
  }

  return {
    matcher: new RegExp(`(${tokens.map(escapeRegExp).join("|")})`, "ig"),
    tokenSet: new Set(tokens.map((token) => token.toLowerCase())),
  };
}

export function renderHighlightedText(text: string, context: HighlightContext): ReactNode {
  if (!context.matcher || context.tokenSet.size === 0) {
    return text;
  }

  return text.split(context.matcher).map((part, index) => {
    const lower = part.toLowerCase();
    if (context.tokenSet.has(lower)) {
      return (
        <mark key={`${lower}-${index}`} className="rounded bg-accent-soft px-[1px] font-semibold text-accent">
          {part}
        </mark>
      );
    }

    return <span key={`${lower}-${index}`}>{part}</span>;
  });
}

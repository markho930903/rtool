type MaybePromise<T> = T | Promise<T>;

export type CleanupFn = () => MaybePromise<void>;

interface CleanupEntry {
  fn: CleanupFn;
  scope?: string;
}

function isPromiseLike(value: unknown): value is Promise<unknown> {
  return typeof value === "object" && value !== null && "then" in value;
}

function formatScope(scope?: string): string {
  return scope ? ` (${scope})` : "";
}

function runCleanup(entry: CleanupEntry, baseScope?: string): void {
  const scope = baseScope && entry.scope ? `${baseScope}:${entry.scope}` : entry.scope ?? baseScope;

  try {
    const result = entry.fn();
    if (isPromiseLike(result)) {
      void result.catch((error) => {
        if (import.meta.env.DEV) {
          console.warn(`[cleanup-stack] cleanup failed${formatScope(scope)}`, error);
        }
      });
    }
  } catch (error) {
    if (import.meta.env.DEV) {
      console.warn(`[cleanup-stack] cleanup failed${formatScope(scope)}`, error);
    }
  }
}

export class CleanupStack {
  private readonly entries: CleanupEntry[] = [];
  private flushed = false;

  constructor(private readonly scope?: string) {}

  add(fn: CleanupFn | null | undefined, scope?: string): void {
    if (!fn) {
      return;
    }

    const entry: CleanupEntry = { fn, scope };
    if (this.flushed) {
      runCleanup(entry, this.scope);
      return;
    }

    this.entries.push(entry);
  }

  flush(): void {
    if (this.flushed) {
      return;
    }
    this.flushed = true;

    while (this.entries.length > 0) {
      const entry = this.entries.pop();
      if (!entry) {
        continue;
      }
      runCleanup(entry, this.scope);
    }
  }
}

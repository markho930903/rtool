type MaybePromise<T> = T | Promise<T>;
type MaybeUnlisten = (() => MaybePromise<void>) | null | undefined;

function isPromiseLike(value: unknown): value is Promise<unknown> {
  return typeof value === "object" && value !== null && "then" in value;
}

function formatScope(scope?: string): string {
  return scope ? ` (${scope})` : "";
}

export function safeUnlisten(unlisten: MaybeUnlisten, scope?: string): void {
  if (!unlisten) {
    return;
  }

  try {
    const result = unlisten();
    if (isPromiseLike(result)) {
      void result.catch((error) => {
        if (import.meta.env.DEV) {
          console.warn(`[tauri-event] unlisten failed${formatScope(scope)}`, error);
        }
      });
    }
  } catch (error) {
    if (import.meta.env.DEV) {
      console.warn(`[tauri-event] unlisten failed${formatScope(scope)}`, error);
    }
  }
}

export function safeResolveUnlisten(
  unlistenPromise: Promise<MaybeUnlisten> | null | undefined,
  scope?: string,
): void {
  if (!unlistenPromise) {
    return;
  }

  void unlistenPromise
    .then((unlisten) => {
      safeUnlisten(unlisten, scope);
    })
    .catch((error) => {
      if (import.meta.env.DEV) {
        console.warn(`[tauri-event] resolve unlisten failed${formatScope(scope)}`, error);
      }
    });
}

import { useEffect, type DependencyList } from "react";

import { CleanupStack } from "@/services/cleanup-stack";

export interface AsyncEffectContext {
  stack: CleanupStack;
  isDisposed: () => boolean;
}

interface UseAsyncEffectOptions {
  scope?: string;
  onError?: (error: unknown) => void;
}

function formatScope(scope?: string): string {
  return scope ? ` (${scope})` : "";
}

export function useAsyncEffect(
  effect: (context: AsyncEffectContext) => Promise<void> | void,
  deps: DependencyList,
  options?: UseAsyncEffectOptions,
): void {
  const { scope, onError } = options ?? {};

  useEffect(() => {
    const stack = new CleanupStack(scope);
    let disposed = false;

    void Promise.resolve(
      effect({
        stack,
        isDisposed: () => disposed,
      }),
    ).catch((error) => {
      if (onError) {
        onError(error);
        return;
      }
      if (import.meta.env.DEV) {
        console.warn(`[async-effect] setup failed${formatScope(scope)}`, error);
      }
    });

    return () => {
      disposed = true;
      stack.flush();
    };
  }, deps);
}

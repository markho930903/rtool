import { useCallback, useRef } from "react";

import { useAsyncEffect } from "@/hooks/useAsyncEffect";

interface WindowFocusChangeEvent {
  payload: boolean;
}

interface WindowFocusAutoHideWindow {
  hide: () => Promise<void>;
  onFocusChanged: (handler: (event: WindowFocusChangeEvent) => void) => Promise<() => void>;
}

interface UseWindowFocusAutoHideOptions {
  appWindow: WindowFocusAutoHideWindow;
  enabled?: boolean;
  delayMs?: number;
  shouldSkipHide?: () => boolean;
  onFocus?: () => void;
}

interface UseWindowFocusAutoHideResult {
  cancelScheduledHide: () => void;
}

export function useWindowFocusAutoHide(options: UseWindowFocusAutoHideOptions): UseWindowFocusAutoHideResult {
  const { appWindow, enabled = true, delayMs = 80, shouldSkipHide, onFocus } = options;
  const hideTimerRef = useRef<number | null>(null);

  const cancelScheduledHide = useCallback(() => {
    if (hideTimerRef.current !== null) {
      window.clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
  }, []);

  const scheduleHide = useCallback(() => {
    if (shouldSkipHide?.()) {
      return;
    }

    cancelScheduledHide();
    hideTimerRef.current = window.setTimeout(() => {
      void appWindow.hide();
    }, delayMs);
  }, [appWindow, cancelScheduledHide, delayMs, shouldSkipHide]);

  useAsyncEffect(
    async ({ stack }) => {
      if (!enabled) {
        return;
      }

      stack.add(cancelScheduledHide, "cancel-timer");

      const unlistenFocus = await appWindow.onFocusChanged(({ payload: focused }) => {
        if (!focused) {
          scheduleHide();
          return;
        }

        cancelScheduledHide();
        onFocus?.();
      });
      stack.add(unlistenFocus, "onFocusChanged");
    },
    [appWindow, cancelScheduledHide, enabled, onFocus, scheduleHide],
    {
      scope: "window-focus-auto-hide",
      onError: (error) => {
        if (import.meta.env.DEV) {
          console.warn("[window-focus-auto-hide] setup failed", error);
        }
      },
    },
  );

  return { cancelScheduledHide };
}

import { useCallback, useEffect, useRef } from "react";

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

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const setup = async () => {
      const unlistenFocus = await appWindow.onFocusChanged(({ payload: focused }) => {
        if (!focused) {
          scheduleHide();
          return;
        }

        cancelScheduledHide();
        onFocus?.();
      });

      return () => {
        unlistenFocus();
      };
    };

    let cleanup: (() => void) | undefined;
    let disposed = false;
    void setup().then((fn) => {
      if (disposed) {
        fn?.();
        return;
      }
      cleanup = fn;
    });

    return () => {
      disposed = true;
      cleanup?.();
      cancelScheduledHide();
    };
  }, [appWindow, cancelScheduledHide, enabled, onFocus, scheduleHide]);

  return { cancelScheduledHide };
}

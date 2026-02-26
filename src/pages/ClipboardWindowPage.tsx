import { LogicalSize } from "@tauri-apps/api/dpi";
import { listen } from "@tauri-apps/api/event";
import { currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from "react";

import ClipboardPanel from "@/components/clipboard/ClipboardPanelContainer";
import { useWindowFocusAutoHide } from "@/hooks/window/useWindowFocusAutoHide";
import { useWindowLayoutPersistence } from "@/hooks/window/useWindowLayoutPersistence";
import type { StoredWindowLayout, WindowLayoutBounds } from "@/hooks/window/window-layout.types";
import { useAsyncEffect } from "@/hooks/useAsyncEffect";
import { useLocaleStore } from "@/i18n/store";
import { invokeWithLog } from "@/services/invoke";
import { runRecoverable } from "@/services/recoverable";
import { useThemeStore } from "@/theme/store";

const WINDOW_LAYOUT_KEY = "clipboard-window-layout";
const CLIPBOARD_WINDOW_LABEL = "clipboard_history";
const COMPACT_WIDTH_LOGICAL = 560;
const REGULAR_WIDTH_LOGICAL = 960;
const MIN_HEIGHT_LOGICAL = 520;

interface ClipboardWindowOpenedPayload {
  compact?: boolean;
}

interface ClipboardWindowModeAppliedPayload {
  compact: boolean;
  appliedWidthLogical: number;
  appliedHeightLogical: number;
  scaleFactor: number;
}

export default function ClipboardWindowPage() {
  const searchInputRef = useRef<HTMLInputElement>(null);
  const modeResizeTimerRef = useRef<number | null>(null);
  const [compactMode, setCompactMode] = useState(false);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const alwaysOnTopRef = useRef(false);
  const appWindow = useMemo(() => getCurrentWindow(), []);
  const syncFromStorage = useThemeStore((state) => state.syncFromStorage);
  const syncLocaleFromBackend = useLocaleStore((state) => state.syncFromBackend);
  const isCompact = compactMode;
  const enabled = appWindow.label === CLIPBOARD_WINDOW_LABEL;

  const resolveScaleFactor = useCallback(async () => {
    const result = await runRecoverable(() => appWindow.scaleFactor(), {
      scope: "clipboard-window",
      action: "resolve_scale_factor",
      message: "resolve scale factor failed",
    });

    if (!result.ok) {
      return 1;
    }

    return Math.max(0.1, result.data);
  }, [appWindow]);
  const resolveLayoutBounds = useCallback(async () => {
    const scaleFactor = await resolveScaleFactor();
    const compactWidthPx = Math.max(1, Math.round(COMPACT_WIDTH_LOGICAL * scaleFactor));
    const minHeightPx = Math.max(1, Math.round(MIN_HEIGHT_LOGICAL * scaleFactor));

    const monitorResult = await runRecoverable(() => currentMonitor(), {
      scope: "clipboard-window",
      action: "read_current_monitor",
      message: "read monitor failed",
    });

    if (!monitorResult.ok) {
      return null;
    }

    const monitorSize = monitorResult.data?.size;
    if (!monitorSize) {
      return null;
    }

    return {
      monitorWidth: monitorSize.width,
      monitorHeight: monitorSize.height,
      minWidth: compactWidthPx,
      minHeight: minHeightPx,
    };
  }, [resolveScaleFactor]);
  const handleLayoutRestored = useCallback((layout: StoredWindowLayout, bounds: WindowLayoutBounds) => {
    setCompactMode(layout.width <= bounds.minWidth);
  }, []);

  useWindowLayoutPersistence({
    appWindow,
    storageKey: WINDOW_LAYOUT_KEY,
    scope: "clipboard-window",
    enabled,
    persistOnInit: true,
    resolveBounds: resolveLayoutBounds,
    onRestored: handleLayoutRestored,
  });

  const { cancelScheduledHide } = useWindowFocusAutoHide({
    appWindow,
    enabled,
    shouldSkipHide: () => alwaysOnTopRef.current,
    onFocus: () => {
      void syncFromStorage();
      void syncLocaleFromBackend();
    },
  });

  useAsyncEffect(
    async ({ isDisposed }) => {
      if (!enabled) {
        return;
      }

      const result = await runRecoverable(() => appWindow.isAlwaysOnTop(), {
        scope: "clipboard-window",
        action: "read_always_on_top",
        message: "read always-on-top failed",
      });

      if (!result.ok || isDisposed()) {
        return;
      }

      alwaysOnTopRef.current = result.data;
      setAlwaysOnTop(result.data);
    },
    [appWindow, enabled],
    {
      scope: "clipboard-window",
      onError: (error) => {
        if (import.meta.env.DEV) {
          console.warn("[clipboard-window] sync always-on-top failed", error);
        }
      },
    },
  );

  useEffect(() => {
    alwaysOnTopRef.current = alwaysOnTop;
  }, [alwaysOnTop]);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    void invokeWithLog("clipboard_window_set_mode", { compact: isCompact }, { silent: true }).catch((error) => {
      console.warn("[clipboard-window] sync mode state failed", {
        compact: isCompact,
        error,
      });
    });
  }, [enabled, isCompact]);

  const applyModeSizeFallback = useCallback(
    async (compact: boolean) => {
      const scaleFactor = await resolveScaleFactor();
      const targetWidthLogical = compact ? COMPACT_WIDTH_LOGICAL : REGULAR_WIDTH_LOGICAL;

      const result = await runRecoverable(
        async () => {
          const size = await appWindow.outerSize();
          const targetHeightLogical = Math.max(size.height / scaleFactor, MIN_HEIGHT_LOGICAL);
          await appWindow.setSize(new LogicalSize(targetWidthLogical, targetHeightLogical));
          return { targetHeightLogical };
        },
        {
          scope: "clipboard-window",
          action: "fallback_resize",
          message: "fallback resize failed",
          metadata: {
            compact,
            targetWidthLogical,
            scaleFactor,
          },
        },
      );

      if (!result.ok) {
        return;
      }

      if (import.meta.env.DEV) {
        console.debug("[clipboard-window] fallback resize applied", {
          compact,
          targetWidthLogical,
          targetHeightLogical: result.data.targetHeightLogical,
          scaleFactor,
        });
      }
    },
    [appWindow, resolveScaleFactor],
  );

  const applyModeSize = useCallback(
    async (compact: boolean) => {
      const targetWidthLogical = compact ? COMPACT_WIDTH_LOGICAL : REGULAR_WIDTH_LOGICAL;
      const modeResult = await runRecoverable(
        () =>
          invokeWithLog<ClipboardWindowModeAppliedPayload>(
            "clipboard_window_apply_mode",
            { compact },
            { silent: true },
          ),
        {
          scope: "clipboard-window",
          action: "apply_mode_resize",
          message: "backend resize failed",
          metadata: { compact, targetWidthLogical },
        },
      );

      if (!modeResult.ok) {
        await applyModeSizeFallback(compact);
        return;
      }

      const payload = modeResult.data;
      const widthDelta = Math.abs(payload.appliedWidthLogical - targetWidthLogical);
      if (widthDelta > 1) {
        console.warn("[clipboard-window] backend resize mismatch, using fallback", {
          compact,
          appliedWidthLogical: payload.appliedWidthLogical,
          targetWidthLogical,
          widthDelta,
        });
        await applyModeSizeFallback(compact);
        return;
      }

      if (import.meta.env.DEV) {
        console.debug("[clipboard-window] backend resize applied", payload);
      }
    },
    [applyModeSizeFallback],
  );

  const handleCompactModeToggle = useCallback(() => {
    const nextCompact = !isCompact;
    setCompactMode(nextCompact);
    void applyModeSize(nextCompact);
    if (modeResizeTimerRef.current !== null) {
      window.clearTimeout(modeResizeTimerRef.current);
    }
    modeResizeTimerRef.current = window.setTimeout(() => {
      void applyModeSize(nextCompact);
      modeResizeTimerRef.current = null;
    }, 80);
  }, [applyModeSize, isCompact]);

  const handleAlwaysOnTopToggle = useCallback(() => {
    const next = !alwaysOnTop;
    void runRecoverable(() => appWindow.setAlwaysOnTop(next), {
      scope: "clipboard-window",
      action: "toggle_always_on_top",
      message: "toggle always-on-top failed",
      metadata: { next },
    }).then((result) => {
      if (!result.ok) {
        return;
      }

      if (next) {
        cancelScheduledHide();
      }
      alwaysOnTopRef.current = next;
      setAlwaysOnTop(next);
    });
  }, [alwaysOnTop, appWindow, cancelScheduledHide]);

  useAsyncEffect(
    async ({ stack }) => {
      if (!enabled) {
        return;
      }

      stack.add(() => {
        if (modeResizeTimerRef.current !== null) {
          window.clearTimeout(modeResizeTimerRef.current);
          modeResizeTimerRef.current = null;
        }
      }, "clear-mode-resize-timer");

      const unlistenOpened = await listen<ClipboardWindowOpenedPayload>(
        "rtool://clipboard-window/opened",
        ({ payload }) => {
          cancelScheduledHide();

          const compact = payload?.compact === true;
          setCompactMode(compact);
          void applyModeSize(compact);
          if (modeResizeTimerRef.current !== null) {
            window.clearTimeout(modeResizeTimerRef.current);
          }
          modeResizeTimerRef.current = window.setTimeout(() => {
            void applyModeSize(compact);
            modeResizeTimerRef.current = null;
          }, 80);

          void syncFromStorage();
          void syncLocaleFromBackend();
          window.setTimeout(() => {
            searchInputRef.current?.focus();
          }, 40);
        },
      );
      stack.add(unlistenOpened, "opened");

      const onKeyDown = (event: KeyboardEvent) => {
        if (event.key === "Escape") {
          event.preventDefault();
          void appWindow.hide();
        }
      };

      window.addEventListener("keydown", onKeyDown);
      stack.add(() => {
        window.removeEventListener("keydown", onKeyDown);
      }, "remove-keydown-listener");

      searchInputRef.current?.focus();
    },
    [appWindow, applyModeSize, cancelScheduledHide, enabled, syncFromStorage, syncLocaleFromBackend],
    {
      scope: "clipboard-window",
      onError: (error) => {
        if (import.meta.env.DEV) {
          console.warn("[clipboard-window] event setup failed", error);
        }
      },
    },
  );

  const handleDrag = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) {
      return;
    }

    const target = event.target as HTMLElement | null;
    if (!target) {
      return;
    }

    if (target.closest("button, input, select, textarea, a, [role='button']")) {
      return;
    }

    void appWindow.startDragging();
  };

  if (!enabled) {
    return null;
  }

  return (
    <div className="h-screen w-screen overflow-hidden rounded-md bg-transparent p-0 text-text-primary">
      <main
        className="rtool-glass-sheen-clip flex h-full w-full overflow-hidden rounded-md border border-border-glass bg-surface-glass-strong p-0 shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]"
        onMouseDown={handleDrag}
      >
        <ClipboardPanel
          className="h-full w-full flex-1 rounded-none border-none bg-transparent p-0"
          searchInputRef={searchInputRef}
          compactMode={isCompact}
          onCompactModeToggle={handleCompactModeToggle}
          alwaysOnTop={alwaysOnTop}
          onAlwaysOnTopToggle={handleAlwaysOnTopToggle}
        />
      </main>
    </div>
  );
}

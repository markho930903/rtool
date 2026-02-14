import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import { useEffect } from "react";

import type { StoredWindowLayout, WindowLayoutBounds } from "@/hooks/window/window-layout.types";
import { clampStoredWindowLayout, parseStoredWindowLayout } from "@/hooks/window/window-layout.utils";
import { runRecoverable } from "@/services/recoverable";

interface WindowLayoutPersistenceWindow {
  outerSize: () => Promise<{ width: number; height: number }>;
  outerPosition: () => Promise<{ x: number; y: number }>;
  setSize: (size: PhysicalSize) => Promise<void>;
  setPosition: (position: PhysicalPosition) => Promise<void>;
  onMoved: (handler: () => void) => Promise<() => void>;
  onResized: (handler: () => void) => Promise<() => void>;
}

interface UseWindowLayoutPersistenceOptions {
  appWindow: WindowLayoutPersistenceWindow;
  storageKey: string;
  scope: string;
  enabled?: boolean;
  persistOnInit?: boolean;
  resolveBounds: (stored: StoredWindowLayout) => Promise<WindowLayoutBounds | null>;
  onRestored?: (layout: StoredWindowLayout, bounds: WindowLayoutBounds) => void;
  onPersisted?: (layout: StoredWindowLayout) => void;
}

export function useWindowLayoutPersistence(options: UseWindowLayoutPersistenceOptions) {
  const {
    appWindow,
    storageKey,
    scope,
    enabled = true,
    persistOnInit = false,
    resolveBounds,
    onRestored,
    onPersisted,
  } = options;

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const persistLayout = async () => {
      const result = await runRecoverable(
        async () => {
          const [size, position] = await Promise.all([appWindow.outerSize(), appWindow.outerPosition()]);
          const layout: StoredWindowLayout = {
            width: size.width,
            height: size.height,
            x: position.x,
            y: position.y,
          };
          window.localStorage.setItem(storageKey, JSON.stringify(layout));
          return layout;
        },
        {
          scope,
          action: "persist_window_layout",
          message: "persist layout failed",
          metadata: { storageKey },
        },
      );

      if (!result.ok) {
        return;
      }

      onPersisted?.(result.data);
    };

    const restoreLayout = async () => {
      const stored = parseStoredWindowLayout(window.localStorage.getItem(storageKey));
      if (!stored) {
        return;
      }

      const boundsResult = await runRecoverable(
        () => resolveBounds(stored),
        {
          scope,
          action: "resolve_window_bounds",
          message: "resolve window bounds failed",
          metadata: { storageKey },
        },
      );

      if (!boundsResult.ok || !boundsResult.data) {
        return;
      }

      const next = clampStoredWindowLayout(stored, boundsResult.data);
      const applyResult = await runRecoverable(
        async () => {
          await appWindow.setSize(new PhysicalSize(next.width, next.height));
          await appWindow.setPosition(new PhysicalPosition(next.x, next.y));
          return next;
        },
        {
          scope,
          action: "restore_window_layout",
          message: "restore layout failed",
          metadata: { storageKey, next },
        },
      );

      if (!applyResult.ok) {
        return;
      }

      onRestored?.(applyResult.data, boundsResult.data);
    };

    const setup = async () => {
      await restoreLayout();

      const unlistenMoved = await appWindow.onMoved(() => {
        void persistLayout();
      });

      const unlistenResized = await appWindow.onResized(() => {
        void persistLayout();
      });

      if (persistOnInit) {
        void persistLayout();
      }

      return () => {
        unlistenMoved();
        unlistenResized();
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
    };
  }, [appWindow, enabled, onPersisted, onRestored, persistOnInit, resolveBounds, scope, storageKey]);
}

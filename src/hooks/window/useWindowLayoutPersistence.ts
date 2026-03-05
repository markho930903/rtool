import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";

import { useAsyncEffect } from "@/hooks/useAsyncEffect";
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
  persistThrottleMs?: number;
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
    persistThrottleMs = 200,
    resolveBounds,
    onRestored,
    onPersisted,
  } = options;

  useAsyncEffect(
    async ({ stack }) => {
      if (!enabled) {
        return;
      }

      let persistTimer: number | null = null;

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

      const schedulePersistLayout = () => {
        if (persistTimer !== null) {
          window.clearTimeout(persistTimer);
        }

        persistTimer = window.setTimeout(() => {
          persistTimer = null;
          void persistLayout();
        }, persistThrottleMs);
      };

      stack.add(() => {
        if (persistTimer !== null) {
          window.clearTimeout(persistTimer);
          persistTimer = null;
        }
      }, "clear-persist-timer");

      const restoreLayout = async () => {
        const stored = parseStoredWindowLayout(window.localStorage.getItem(storageKey));
        if (!stored) {
          return;
        }

        const boundsResult = await runRecoverable(() => resolveBounds(stored), {
          scope,
          action: "resolve_window_bounds",
          message: "resolve window bounds failed",
          metadata: { storageKey },
        });

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

      await restoreLayout();

      const unlistenMoved = await appWindow.onMoved(() => {
        schedulePersistLayout();
      });
      stack.add(unlistenMoved, "onMoved");

      const unlistenResized = await appWindow.onResized(() => {
        schedulePersistLayout();
      });
      stack.add(unlistenResized, "onResized");

      if (persistOnInit) {
        void persistLayout();
      }
    },
    [appWindow, enabled, onPersisted, onRestored, persistOnInit, persistThrottleMs, resolveBounds, scope, storageKey],
    {
      scope: `window-layout:${scope}`,
      onError: (error) => {
        if (import.meta.env.DEV) {
          console.warn(`[${scope}] window layout setup failed`, error);
        }
      },
    },
  );
}

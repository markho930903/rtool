import type { StoredWindowLayout, WindowLayoutBounds } from "@/hooks/window/window-layout.types";
import { runRecoverableSync } from "@/services/recoverable";

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

export function parseStoredWindowLayout(raw: string | null): StoredWindowLayout | null {
  if (!raw) {
    return null;
  }

  const parsed = runRecoverableSync(() => JSON.parse(raw) as Partial<StoredWindowLayout>, {
    scope: "window-layout",
    action: "parse_stored_layout",
    message: "invalid stored layout",
    silent: true,
  });

  if (!parsed.ok) {
    return null;
  }

  const value = parsed.data;
  if (
    !isFiniteNumber(value.width) ||
    !isFiniteNumber(value.height) ||
    !isFiniteNumber(value.x) ||
    !isFiniteNumber(value.y)
  ) {
    return null;
  }

  return {
    width: value.width,
    height: value.height,
    x: value.x,
    y: value.y,
  };
}

export function clampStoredWindowLayout(layout: StoredWindowLayout, bounds: WindowLayoutBounds): StoredWindowLayout {
  const width = Math.min(Math.max(layout.width, bounds.minWidth), bounds.monitorWidth);
  const height = Math.min(Math.max(layout.height, bounds.minHeight), bounds.monitorHeight);
  const maxX = Math.max(0, bounds.monitorWidth - width);
  const maxY = Math.max(0, bounds.monitorHeight - height);

  return {
    width,
    height,
    x: Math.min(Math.max(layout.x, 0), maxX),
    y: Math.min(Math.max(layout.y, 0), maxY),
  };
}

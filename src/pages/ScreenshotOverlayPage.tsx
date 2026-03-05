import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type MouseEvent } from "react";

import type {
  ScreenshotOperationResultPayload,
  ScreenshotSessionDto,
  ScreenshotWindowOpenedPayload,
} from "@/contracts";
import { safeResolveUnlisten, safeUnlisten } from "@/services/tauri-event";
import {
  screenshotCancelSession,
  screenshotCommitSelection,
  screenshotPinSelection,
  screenshotStartSession,
} from "@/services/screenshot.service";

interface Point {
  x: number;
  y: number;
}

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface Size {
  width: number;
  height: number;
}

type ResizeHandle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";
type ToolbarPlacement =
  | "outside-top-right"
  | "outside-bottom-right"
  | "inside-top-right"
  | "inside-bottom-right";

interface ToolbarPosition {
  left: number;
  top: number;
  placement: ToolbarPlacement;
}

interface PointerSession {
  startPoint: Point;
  mode: "draw" | "move" | "resize";
  moved: boolean;
  baseRect: Rect | null;
  resizeHandle: ResizeHandle | null;
}

interface InvokeErrorPayload {
  code?: string;
  message?: string;
}

const DRAG_THRESHOLD_PX = 3;
const MIN_RECT_SIZE_PX = 2;
const FLOATING_GAP_PX = 8;
const FLOATING_PADDING_PX = 8;
const FLOATING_INSIDE_OFFSET_PX = 4;
const ERROR_GAP_PX = 6;
const TOOLBAR_FALLBACK_SIZE: Size = { width: 248, height: 36 };
const ERROR_FALLBACK_SIZE: Size = { width: 220, height: 28 };
const SCREENSHOT_CONTROL_SELECTOR = "[data-screenshot-controls]";
const SCREENSHOT_RESIZE_HANDLE_ATTR = "data-screenshot-resize-handle";
const SCREENSHOT_RESIZE_HANDLE_SELECTOR = `[${SCREENSHOT_RESIZE_HANDLE_ATTR}]`;
const RESIZE_HANDLES: ReadonlyArray<{
  handle: ResizeHandle;
  className: string;
  cursor: string;
}> = [
  { handle: "nw", className: "-left-1 -top-1 h-3 w-3", cursor: "nwse-resize" },
  { handle: "n", className: "left-1/2 -top-1 h-2 w-6 -translate-x-1/2", cursor: "ns-resize" },
  { handle: "ne", className: "-right-1 -top-1 h-3 w-3", cursor: "nesw-resize" },
  { handle: "e", className: "-right-1 top-1/2 h-6 w-2 -translate-y-1/2", cursor: "ew-resize" },
  { handle: "se", className: "-bottom-1 -right-1 h-3 w-3", cursor: "nwse-resize" },
  { handle: "s", className: "-bottom-1 left-1/2 h-2 w-6 -translate-x-1/2", cursor: "ns-resize" },
  { handle: "sw", className: "-bottom-1 -left-1 h-3 w-3", cursor: "nesw-resize" },
  { handle: "w", className: "-left-1 top-1/2 h-6 w-2 -translate-y-1/2", cursor: "ew-resize" },
];

function isResizeHandle(value: string | null): value is ResizeHandle {
  return (
    value === "n" ||
    value === "s" ||
    value === "e" ||
    value === "w" ||
    value === "ne" ||
    value === "nw" ||
    value === "se" ||
    value === "sw"
  );
}

function getResizeHandleFromTarget(target: EventTarget | null): ResizeHandle | null {
  if (!(target instanceof Element)) {
    return null;
  }
  const handleElement = target.closest(SCREENSHOT_RESIZE_HANDLE_SELECTOR);
  if (!handleElement) {
    return null;
  }
  const handle = handleElement.getAttribute(SCREENSHOT_RESIZE_HANDLE_ATTR);
  return isResizeHandle(handle) ? handle : null;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function normalizeRect(start: Point, end: Point): Rect {
  const left = Math.min(start.x, end.x);
  const top = Math.min(start.y, end.y);
  const right = Math.max(start.x, end.x);
  const bottom = Math.max(start.y, end.y);
  return {
    x: Math.round(left),
    y: Math.round(top),
    width: Math.max(0, Math.round(right - left)),
    height: Math.max(0, Math.round(bottom - top)),
  };
}

function hasValidRect(rect: Rect | null): rect is Rect {
  return Boolean(rect && rect.width >= MIN_RECT_SIZE_PX && rect.height >= MIN_RECT_SIZE_PX);
}

function isPointInRect(point: Point, rect: Rect): boolean {
  return (
    point.x >= rect.x &&
    point.x <= rect.x + rect.width &&
    point.y >= rect.y &&
    point.y <= rect.y + rect.height
  );
}

function hasMeaningfulMovement(start: Point, end: Point): boolean {
  return Math.abs(end.x - start.x) > DRAG_THRESHOLD_PX || Math.abs(end.y - start.y) > DRAG_THRESHOLD_PX;
}

function translateRect(rect: Rect, dx: number, dy: number, maxWidth: number, maxHeight: number): Rect {
  const nextX = Math.round(rect.x + dx);
  const nextY = Math.round(rect.y + dy);
  const limitX = Math.max(0, Math.round(maxWidth) - rect.width);
  const limitY = Math.max(0, Math.round(maxHeight) - rect.height);
  return {
    x: clamp(nextX, 0, limitX),
    y: clamp(nextY, 0, limitY),
    width: rect.width,
    height: rect.height,
  };
}

function resizeRect(
  rect: Rect,
  handle: ResizeHandle,
  dx: number,
  dy: number,
  maxWidth: number,
  maxHeight: number,
): Rect {
  const roundedMaxWidth = Math.max(MIN_RECT_SIZE_PX, Math.round(maxWidth));
  const roundedMaxHeight = Math.max(MIN_RECT_SIZE_PX, Math.round(maxHeight));
  const minWidth = Math.min(MIN_RECT_SIZE_PX, roundedMaxWidth);
  const minHeight = Math.min(MIN_RECT_SIZE_PX, roundedMaxHeight);

  let left = clamp(Math.round(rect.x), 0, roundedMaxWidth - minWidth);
  let top = clamp(Math.round(rect.y), 0, roundedMaxHeight - minHeight);
  let right = clamp(Math.round(rect.x + rect.width), left + minWidth, roundedMaxWidth);
  let bottom = clamp(Math.round(rect.y + rect.height), top + minHeight, roundedMaxHeight);

  if (handle.includes("w")) {
    left = clamp(Math.round(rect.x + dx), 0, right - minWidth);
  }
  if (handle.includes("e")) {
    right = clamp(Math.round(rect.x + rect.width + dx), left + minWidth, roundedMaxWidth);
  }
  if (handle.includes("n")) {
    top = clamp(Math.round(rect.y + dy), 0, bottom - minHeight);
  }
  if (handle.includes("s")) {
    bottom = clamp(Math.round(rect.y + rect.height + dy), top + minHeight, roundedMaxHeight);
  }

  return {
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  };
}

function clampFloatingCoordinate(value: number, viewportSpan: number, floatSpan: number, padding: number): number {
  const max = Math.max(padding, viewportSpan - floatSpan - padding);
  return clamp(value, padding, max);
}

function clampFloatingPosition(
  left: number,
  top: number,
  size: Size,
  viewport: Size,
  padding: number,
): Pick<ToolbarPosition, "left" | "top"> {
  return {
    left: clampFloatingCoordinate(left, viewport.width, size.width, padding),
    top: clampFloatingCoordinate(top, viewport.height, size.height, padding),
  };
}

function estimateOverflow(left: number, top: number, size: Size, viewport: Size, padding: number): number {
  const overflowLeft = Math.max(0, padding - left);
  const overflowTop = Math.max(0, padding - top);
  const overflowRight = Math.max(0, left + size.width - (viewport.width - padding));
  const overflowBottom = Math.max(0, top + size.height - (viewport.height - padding));
  return overflowLeft + overflowTop + overflowRight + overflowBottom;
}

function resolveToolbarRawPosition(rect: Rect, size: Size, placement: ToolbarPlacement): Pick<ToolbarPosition, "left" | "top"> {
  switch (placement) {
    case "outside-top-right":
      return {
        left: rect.x + rect.width - size.width,
        top: rect.y - FLOATING_GAP_PX - size.height,
      };
    case "outside-bottom-right":
      return {
        left: rect.x + rect.width - size.width,
        top: rect.y + rect.height + FLOATING_GAP_PX,
      };
    case "inside-top-right":
      return {
        left: rect.x + rect.width - FLOATING_INSIDE_OFFSET_PX - size.width,
        top: rect.y + FLOATING_INSIDE_OFFSET_PX,
      };
    case "inside-bottom-right":
      return {
        left: rect.x + rect.width - FLOATING_INSIDE_OFFSET_PX - size.width,
        top: rect.y + rect.height - FLOATING_INSIDE_OFFSET_PX - size.height,
      };
  }
}

function estimatePlacementSpace(rect: Rect, viewport: Size, placement: ToolbarPlacement): number {
  const outsideTopSpace = rect.y - FLOATING_GAP_PX;
  const outsideBottomSpace = viewport.height - (rect.y + rect.height) - FLOATING_GAP_PX;
  switch (placement) {
    case "outside-top-right":
      return outsideTopSpace;
    case "outside-bottom-right":
      return outsideBottomSpace;
    case "inside-top-right":
    case "inside-bottom-right":
      return rect.width - FLOATING_INSIDE_OFFSET_PX * 2;
  }
}

function compareToolbarPlacement(
  a: ToolbarPosition & { overflow: number; space: number },
  b: ToolbarPosition & { overflow: number; space: number },
): number {
  if (a.overflow !== b.overflow) {
    return a.overflow - b.overflow;
  }
  if (a.space !== b.space) {
    return b.space - a.space;
  }
  return 0;
}

function resolveToolbarPosition(rect: Rect, size: Size, viewport: Size): ToolbarPosition {
  const outsidePlacements: ToolbarPlacement[] = ["outside-top-right", "outside-bottom-right"];
  const insidePlacements: ToolbarPlacement[] = ["inside-top-right", "inside-bottom-right"];

  const buildCandidates = (placements: ToolbarPlacement[]) =>
    placements.map((placement) => {
      const raw = resolveToolbarRawPosition(rect, size, placement);
      const clamped = clampFloatingPosition(raw.left, raw.top, size, viewport, FLOATING_PADDING_PX);
      return {
        placement,
        left: clamped.left,
        top: clamped.top,
        overflow: estimateOverflow(raw.left, raw.top, size, viewport, FLOATING_PADDING_PX),
        space: estimatePlacementSpace(rect, viewport, placement),
      };
    });

  const outside = buildCandidates(outsidePlacements);
  const fitOutside = outside.find((item) => item.overflow === 0);
  if (fitOutside) {
    return { left: fitOutside.left, top: fitOutside.top, placement: fitOutside.placement };
  }

  const inside = buildCandidates(insidePlacements);
  const fitInside = inside.find((item) => item.overflow === 0);
  if (fitInside) {
    return { left: fitInside.left, top: fitInside.top, placement: fitInside.placement };
  }

  const bestOutside = [...outside].sort(compareToolbarPlacement)[0];
  const bestInside = [...inside].sort(compareToolbarPlacement)[0];
  if (!bestOutside && !bestInside) {
    return { left: FLOATING_PADDING_PX, top: FLOATING_PADDING_PX, placement: "outside-top-right" };
  }
  if (!bestInside && bestOutside) {
    return { left: bestOutside.left, top: bestOutside.top, placement: bestOutside.placement };
  }
  if (!bestOutside && bestInside) {
    return { left: bestInside.left, top: bestInside.top, placement: bestInside.placement };
  }
  if (bestInside.overflow < bestOutside.overflow) {
    return { left: bestInside.left, top: bestInside.top, placement: bestInside.placement };
  }
  return { left: bestOutside.left, top: bestOutside.top, placement: bestOutside.placement };
}

function resolveErrorPosition(toolbar: ToolbarPosition, toolbarSize: Size, errorSize: Size, viewport: Size): Point {
  const spaceBelow = viewport.height - (toolbar.top + toolbarSize.height) - FLOATING_PADDING_PX;
  const spaceAbove = toolbar.top - FLOATING_PADDING_PX;
  const renderBelow = spaceBelow >= errorSize.height + ERROR_GAP_PX || spaceBelow >= spaceAbove;
  const rawTop = renderBelow
    ? toolbar.top + toolbarSize.height + ERROR_GAP_PX
    : toolbar.top - ERROR_GAP_PX - errorSize.height;
  const clamped = clampFloatingPosition(toolbar.left, rawTop, errorSize, viewport, FLOATING_PADDING_PX);
  return { x: clamped.left, y: clamped.top };
}

function isEventFromControls(target: EventTarget | null): boolean {
  return target instanceof Element && Boolean(target.closest(SCREENSHOT_CONTROL_SELECTOR));
}

function isObjectRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function toInvokeErrorPayload(error: unknown): InvokeErrorPayload | null {
  if (isObjectRecord(error)) {
    return error as InvokeErrorPayload;
  }

  if (!(error instanceof Error)) {
    return null;
  }

  try {
    const parsed = JSON.parse(error.message);
    if (isObjectRecord(parsed)) {
      return parsed as InvokeErrorPayload;
    }
  } catch {
    return null;
  }

  return null;
}

export default function ScreenshotOverlayPage() {
  const appWindow = useMemo(() => getCurrentWindow(), []);
  const [session, setSession] = useState<ScreenshotSessionDto | null>(null);
  const [selectionRect, setSelectionRect] = useState<Rect | null>(null);
  const [pending, setPending] = useState(false);
  const [pinErrorMessage, setPinErrorMessage] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const toolbarRef = useRef<HTMLDivElement>(null);
  const pinErrorRef = useRef<HTMLDivElement>(null);
  const pointerSessionRef = useRef<PointerSession | null>(null);
  const sessionRef = useRef<ScreenshotSessionDto | null>(null);
  const [toolbarSize, setToolbarSize] = useState<Size>(TOOLBAR_FALLBACK_SIZE);
  const [pinErrorSize, setPinErrorSize] = useState<Size>(ERROR_FALLBACK_SIZE);

  const activeDisplay =
    session?.displays.find((item) => item.id === session.activeDisplayId) ?? session?.displays[0] ?? null;
  const validSelection = hasValidRect(selectionRect) ? selectionRect : null;

  const resetSelection = useCallback(() => {
    pointerSessionRef.current = null;
    setSelectionRect(null);
    setPinErrorMessage(null);
  }, []);

  const closeOverlay = useCallback(() => {
    void appWindow.hide();
  }, [appWindow]);

  const cancelCurrentSession = useCallback(async () => {
    if (session) {
      await screenshotCancelSession(session.sessionId).catch(() => undefined);
    }
    setSession(null);
    resetSelection();
  }, [resetSelection, session]);

  const handleCommit = useCallback(async () => {
    if (!session || !hasValidRect(selectionRect) || pending) {
      return;
    }
    setPinErrorMessage(null);
    setPending(true);
    try {
      await screenshotCommitSelection({
        sessionId: session.sessionId,
        x: selectionRect.x,
        y: selectionRect.y,
        width: selectionRect.width,
        height: selectionRect.height,
        autoSave: null,
      });
      setSession(null);
      resetSelection();
      closeOverlay();
    } catch (error) {
      if (import.meta.env.DEV) {
        console.warn("[screenshot-overlay] commit failed", error);
      }
    } finally {
      setPending(false);
    }
  }, [closeOverlay, pending, resetSelection, selectionRect, session]);

  const handlePin = useCallback(async () => {
    if (!session || !hasValidRect(selectionRect) || pending) {
      return;
    }
    setPinErrorMessage(null);
    setPending(true);
    try {
      await screenshotPinSelection({
        sessionId: session.sessionId,
        x: selectionRect.x,
        y: selectionRect.y,
        width: selectionRect.width,
        height: selectionRect.height,
        autoSave: null,
      });
      setSession(null);
      resetSelection();
      closeOverlay();
    } catch (error) {
      const payload = toInvokeErrorPayload(error);
      if (payload?.code === "screenshot_pin_limit_reached") {
        setPinErrorMessage(payload.message ?? "Pinned screenshot limit reached.");
        return;
      }
      if (import.meta.env.DEV) {
        console.warn("[screenshot-overlay] pin failed", error);
      }
      setPinErrorMessage("Pin failed, please try again.");
    } finally {
      setPending(false);
    }
  }, [closeOverlay, pending, resetSelection, selectionRect, session]);

  useEffect(() => {
    sessionRef.current = session;
  }, [session]);

  useEffect(() => {
    if (appWindow.label !== "screenshot_overlay") {
      return;
    }

    const releaseSession = () => {
      const current = sessionRef.current;
      if (!current) {
        return;
      }
      sessionRef.current = null;
      void screenshotCancelSession(current.sessionId).catch(() => undefined);
    };

    let mounted = true;
    let unlistenClose: (() => void) | null = null;
    void appWindow.onCloseRequested(() => {
      releaseSession();
    }).then((unlisten) => {
      if (!mounted) {
        safeUnlisten(unlisten, "screenshot-overlay:close-requested:late");
        return;
      }
      unlistenClose = unlisten;
    });

    const onBeforeUnload = () => {
      releaseSession();
    };
    window.addEventListener("beforeunload", onBeforeUnload);

    return () => {
      mounted = false;
      if (unlistenClose) {
        safeUnlisten(unlistenClose, "screenshot-overlay:close-requested");
      }
      window.removeEventListener("beforeunload", onBeforeUnload);
      releaseSession();
    };
  }, [appWindow]);

  useEffect(() => {
    if (appWindow.label !== "screenshot_overlay") {
      return;
    }

    const unlistenPromise = listen<ScreenshotWindowOpenedPayload>(
      "rtool://screenshot-window/opened",
      (event) => {
        const payload = event.payload?.session;
        if (!payload) {
          return;
        }
        setSession(payload);
        resetSelection();
      },
    );

    return () => {
      safeResolveUnlisten(unlistenPromise, "screenshot-overlay:window-opened");
    };
  }, [appWindow, resetSelection]);

  useEffect(() => {
    if (appWindow.label !== "screenshot_overlay") {
      return;
    }

    const unlistenPromise = listen<ScreenshotOperationResultPayload>(
      "rtool://screenshot/operation-result",
      (event) => {
        const payload = event.payload;
        if (!payload || payload.ok) {
          return;
        }
        if (import.meta.env.DEV) {
          console.warn("[screenshot-overlay] async operation failed", payload);
        }
      },
    );

    return () => {
      safeResolveUnlisten(unlistenPromise, "screenshot-overlay:operation-result");
    };
  }, [appWindow]);

  useEffect(() => {
    if (appWindow.label !== "screenshot_overlay") {
      return;
    }

    if (session) {
      return;
    }

    let disposed = false;
    void appWindow
      .isVisible()
      .then((visible) => {
        if (!visible || disposed) {
          return;
        }
        return screenshotStartSession()
          .then((next) => {
            if (!disposed) {
              setSession(next);
            }
          })
          .catch((error) => {
            if (import.meta.env.DEV) {
              console.warn("[screenshot-overlay] start session failed", error);
            }
          });
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
    };
  }, [appWindow, session]);

  useEffect(() => {
    if (appWindow.label !== "screenshot_overlay") {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void cancelCurrentSession().finally(() => closeOverlay());
        return;
      }
      if (event.key === "Enter") {
        event.preventDefault();
        void handleCommit();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [appWindow, cancelCurrentSession, closeOverlay, handleCommit]);

  const resolveSelectionBounds = useCallback(() => {
    const host = containerRef.current;
    if (activeDisplay) {
      return { width: activeDisplay.width, height: activeDisplay.height };
    }
    if (!host) {
      return { width: 0, height: 0 };
    }
    const rect = host.getBoundingClientRect();
    return { width: rect.width, height: rect.height };
  }, [activeDisplay]);

  useLayoutEffect(() => {
    if (!validSelection) {
      return;
    }

    const nextToolbarRect = toolbarRef.current?.getBoundingClientRect();
    if (nextToolbarRect && nextToolbarRect.width > 0 && nextToolbarRect.height > 0) {
      const nextToolbarSize = {
        width: Math.ceil(nextToolbarRect.width),
        height: Math.ceil(nextToolbarRect.height),
      };
      setToolbarSize((current) => {
        if (current.width === nextToolbarSize.width && current.height === nextToolbarSize.height) {
          return current;
        }
        return nextToolbarSize;
      });
    }

    if (!pinErrorMessage) {
      return;
    }
    const nextErrorRect = pinErrorRef.current?.getBoundingClientRect();
    if (nextErrorRect && nextErrorRect.width > 0 && nextErrorRect.height > 0) {
      const nextErrorSize = {
        width: Math.ceil(nextErrorRect.width),
        height: Math.ceil(nextErrorRect.height),
      };
      setPinErrorSize((current) => {
        if (current.width === nextErrorSize.width && current.height === nextErrorSize.height) {
          return current;
        }
        return nextErrorSize;
      });
    }
  }, [pending, pinErrorMessage, validSelection]);

  const floatingViewport = useMemo(() => {
    const bounds = resolveSelectionBounds();
    return {
      width: Math.max(0, Math.round(bounds.width)),
      height: Math.max(0, Math.round(bounds.height)),
    };
  }, [resolveSelectionBounds]);

  const toolbarPosition = useMemo(() => {
    if (!validSelection || floatingViewport.width <= 0 || floatingViewport.height <= 0) {
      return null;
    }
    return resolveToolbarPosition(validSelection, toolbarSize, floatingViewport);
  }, [floatingViewport, toolbarSize, validSelection]);

  const pinErrorPosition = useMemo(() => {
    if (!pinErrorMessage || !toolbarPosition || floatingViewport.width <= 0 || floatingViewport.height <= 0) {
      return null;
    }
    return resolveErrorPosition(toolbarPosition, toolbarSize, pinErrorSize, floatingViewport);
  }, [floatingViewport, pinErrorMessage, pinErrorSize, toolbarPosition, toolbarSize]);

  const toLocalPoint = useCallback(
    (event: MouseEvent<HTMLDivElement>): Point | null => {
      const host = containerRef.current;
      if (!host) {
        return null;
      }
      const rect = host.getBoundingClientRect();
      const maxWidth = activeDisplay?.width ?? rect.width;
      const maxHeight = activeDisplay?.height ?? rect.height;
      return {
        x: clamp(event.clientX - rect.left, 0, maxWidth),
        y: clamp(event.clientY - rect.top, 0, maxHeight),
      };
    },
    [activeDisplay?.height, activeDisplay?.width],
  );

  const onMouseDown = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      if (event.button !== 0 || pending) {
        return;
      }
      const point = toLocalPoint(event);
      if (!point) {
        return;
      }
      const resizeHandle = getResizeHandleFromTarget(event.target);
      if (resizeHandle && validSelection) {
        pointerSessionRef.current = {
          startPoint: point,
          mode: "resize",
          moved: false,
          baseRect: validSelection,
          resizeHandle,
        };
        return;
      }
      if (isEventFromControls(event.target)) {
        return;
      }
      const startedInsideSelection = validSelection ? isPointInRect(point, validSelection) : false;
      pointerSessionRef.current = {
        startPoint: point,
        mode: startedInsideSelection ? "move" : "draw",
        moved: false,
        baseRect: startedInsideSelection ? validSelection : null,
        resizeHandle: null,
      };
    },
    [pending, toLocalPoint, validSelection],
  );

  const onMouseMove = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      const pointerSession = pointerSessionRef.current;
      if (!pointerSession) {
        return;
      }
      const point = toLocalPoint(event);
      if (!point) {
        return;
      }
      if (!pointerSession.moved && hasMeaningfulMovement(pointerSession.startPoint, point)) {
        pointerSession.moved = true;
      }
      if (!pointerSession.moved) {
        return;
      }
      if (pointerSession.mode === "draw") {
        setSelectionRect(normalizeRect(pointerSession.startPoint, point));
        return;
      }
      if (pointerSession.mode === "move" && pointerSession.baseRect) {
        const bounds = resolveSelectionBounds();
        setSelectionRect(
          translateRect(
            pointerSession.baseRect,
            point.x - pointerSession.startPoint.x,
            point.y - pointerSession.startPoint.y,
            bounds.width,
            bounds.height,
          ),
        );
        return;
      }
      if (pointerSession.mode === "resize" && pointerSession.baseRect && pointerSession.resizeHandle) {
        const bounds = resolveSelectionBounds();
        setSelectionRect(
          resizeRect(
            pointerSession.baseRect,
            pointerSession.resizeHandle,
            point.x - pointerSession.startPoint.x,
            point.y - pointerSession.startPoint.y,
            bounds.width,
            bounds.height,
          ),
        );
      }
    },
    [resolveSelectionBounds, toLocalPoint],
  );

  const onMouseUp = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      const pointerSession = pointerSessionRef.current;
      if (!pointerSession) {
        return;
      }
      pointerSessionRef.current = null;

      const point = toLocalPoint(event);
      if (!point) {
        return;
      }

      if (pointerSession.mode === "draw") {
        setSelectionRect(normalizeRect(pointerSession.startPoint, point));
        return;
      }
      if (pointerSession.mode === "move" && pointerSession.baseRect) {
        const bounds = resolveSelectionBounds();
        setSelectionRect(
          translateRect(
            pointerSession.baseRect,
            point.x - pointerSession.startPoint.x,
            point.y - pointerSession.startPoint.y,
            bounds.width,
            bounds.height,
          ),
        );
        return;
      }
      if (pointerSession.mode === "resize" && pointerSession.baseRect && pointerSession.resizeHandle) {
        const bounds = resolveSelectionBounds();
        setSelectionRect(
          resizeRect(
            pointerSession.baseRect,
            pointerSession.resizeHandle,
            point.x - pointerSession.startPoint.x,
            point.y - pointerSession.startPoint.y,
            bounds.width,
            bounds.height,
          ),
        );
      }
    },
    [resolveSelectionBounds, toLocalPoint],
  );

  return (
    <div
      ref={containerRef}
      className="relative h-full w-full select-none overflow-hidden bg-black/28"
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
    >
      {activeDisplay ? (
        <div className="pointer-events-none absolute left-4 top-4 rounded-md border border-white/30 bg-black/45 px-3 py-2 text-xs text-white/95 shadow-lg backdrop-blur-sm">
          <div className="font-semibold">Screenshot</div>
          <div className="opacity-85">
            {activeDisplay.width} × {activeDisplay.height} · {activeDisplay.name}
          </div>
          <div className="mt-1 opacity-70">Drag to select, move, or resize · Click confirm · Enter copy · Esc cancel</div>
        </div>
      ) : null}

      {validSelection ? (
        <>
          <div
            className="pointer-events-none absolute border-2 border-white/95 shadow-[0_0_0_1px_rgba(0,0,0,0.35)]"
            style={{
              left: `${validSelection.x}px`,
              top: `${validSelection.y}px`,
              width: `${validSelection.width}px`,
              height: `${validSelection.height}px`,
            }}
          />
          <div
            className="absolute"
            style={{
              left: `${validSelection.x}px`,
              top: `${validSelection.y}px`,
              width: `${validSelection.width}px`,
              height: `${validSelection.height}px`,
            }}
          >
            {RESIZE_HANDLES.map((item) => (
              <button
                key={item.handle}
                type="button"
                tabIndex={-1}
                aria-label={`Resize selection ${item.handle}`}
                title="Resize selection"
                className={`absolute z-10 rounded-[2px] border border-white/80 bg-black/40 transition hover:bg-white/35 ${item.className}`}
                data-screenshot-resize-handle={item.handle}
                style={{ cursor: item.cursor }}
              />
            ))}
          </div>
          <div
            ref={toolbarRef}
            className="absolute z-20 flex items-center gap-2 rounded-lg border border-white/35 bg-black/72 px-2 py-1.5 text-xs text-white shadow-xl backdrop-blur-md"
            data-screenshot-controls="true"
            style={{
              left: `${toolbarPosition?.left ?? Math.max(FLOATING_PADDING_PX, validSelection.x)}px`,
              top: `${toolbarPosition?.top ?? Math.max(FLOATING_PADDING_PX, validSelection.y - 44)}px`,
            }}
          >
            <button
              type="button"
              className="inline-flex h-7 w-7 items-center justify-center rounded bg-white/18 text-white transition hover:bg-white/28 disabled:cursor-not-allowed disabled:opacity-60"
              disabled={pending}
              aria-label={pending ? "Processing" : "Confirm"}
              title={pending ? "Processing" : "Confirm"}
              onClick={() => void handleCommit()}
            >
              {pending ? (
                <span className="btn-icon i-noto:hourglass-not-done animate-spin text-[1rem]" aria-hidden="true" />
              ) : (
                <span className="btn-icon i-lucide:check text-[1rem]" aria-hidden="true" />
              )}
            </button>
            <button
              type="button"
              className="inline-flex h-7 w-7 items-center justify-center rounded bg-white/18 text-white transition hover:bg-white/28 disabled:cursor-not-allowed disabled:opacity-60"
              disabled={pending}
              aria-label="Pin to screen"
              title="Pin to screen"
              onClick={() => void handlePin()}
            >
              <span className="btn-icon i-noto:pushpin text-[1rem]" aria-hidden="true" />
            </button>
            <button
              type="button"
              className="inline-flex h-7 w-7 items-center justify-center rounded bg-transparent text-white/90 transition hover:bg-white/18"
              disabled={pending}
              aria-label="Cancel"
              title="Cancel"
              onClick={() => {
                void cancelCurrentSession().finally(() => closeOverlay());
              }}
            >
              <span className="btn-icon i-noto:cross-mark text-[1rem]" aria-hidden="true" />
            </button>
            <span className="rounded bg-white/10 px-2 py-0.5 font-mono text-[11px] text-white/85">
              {validSelection.width}×{validSelection.height}
            </span>
          </div>
          {pinErrorMessage ? (
            <div
              ref={pinErrorRef}
              className="absolute z-20 rounded-md border border-amber-300/35 bg-black/72 px-2.5 py-1 text-xs text-amber-100 shadow-xl backdrop-blur-md"
              data-screenshot-controls="true"
              style={{
                left: `${pinErrorPosition?.x ?? (toolbarPosition?.left ?? Math.max(FLOATING_PADDING_PX, validSelection.x))}px`,
                top: `${pinErrorPosition?.y ?? Math.max(FLOATING_PADDING_PX, validSelection.y - 16)}px`,
              }}
            >
              {pinErrorMessage}
            </div>
          ) : null}
        </>
      ) : null}
    </div>
  );
}

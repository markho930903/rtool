import { convertFileSrc } from "@tauri-apps/api/core";
import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent, type WheelEvent } from "react";

import type { ScreenshotPinWindowOpenedPayload } from "@/contracts";
import { safeResolveUnlisten } from "@/services/tauri-event";

const CLOSE_SELECTOR = "[data-screenshot-pin-close]";
const ZOOM_STEP = 0.1;
const ZOOM_MIN = 0.3;
const ZOOM_MAX = 4.0;

function isFromCloseControl(target: EventTarget | null): boolean {
  return target instanceof Element && Boolean(target.closest(CLOSE_SELECTOR));
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export default function ScreenshotPinPage() {
  const appWindow = useMemo(() => getCurrentWindow(), []);
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const baseSizeRef = useRef<{ width: number; height: number } | null>(null);
  const currentScaleRef = useRef(1);
  const zoomingRef = useRef(false);

  const hideWindow = useCallback(() => {
    void appWindow.hide();
  }, [appWindow]);

  const handleDrag = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      if (event.button !== 0 || isFromCloseControl(event.target)) {
        return;
      }
      void appWindow.startDragging();
    },
    [appWindow],
  );

  const handleWheel = useCallback(
    async (event: WheelEvent<HTMLDivElement>) => {
      if (isFromCloseControl(event.target) || zoomingRef.current) {
        return;
      }
      if (event.deltaY === 0) {
        return;
      }
      event.preventDefault();

      const baseSize = baseSizeRef.current;
      if (!baseSize) {
        return;
      }

      const previousScale = currentScaleRef.current;
      const deltaScale = event.deltaY < 0 ? 1 + ZOOM_STEP : 1 - ZOOM_STEP;
      const nextScale = clamp(previousScale * deltaScale, ZOOM_MIN, ZOOM_MAX);
      if (Math.abs(nextScale - previousScale) < 0.0001) {
        return;
      }

      const previousWidth = Math.max(1, Math.round(baseSize.width * previousScale));
      const previousHeight = Math.max(1, Math.round(baseSize.height * previousScale));
      const nextWidth = Math.max(1, Math.round(baseSize.width * nextScale));
      const nextHeight = Math.max(1, Math.round(baseSize.height * nextScale));

      zoomingRef.current = true;
      try {
        const position = await appWindow.outerPosition();
        const centerX = position.x + previousWidth / 2;
        const centerY = position.y + previousHeight / 2;
        const nextX = Math.round(centerX - nextWidth / 2);
        const nextY = Math.round(centerY - nextHeight / 2);

        await appWindow.setSize(new PhysicalSize(nextWidth, nextHeight));
        await appWindow.setPosition(new PhysicalPosition(nextX, nextY));
        currentScaleRef.current = nextScale;
      } catch (error) {
        if (import.meta.env.DEV) {
          console.warn("[screenshot-pin] zoom failed", error);
        }
      } finally {
        zoomingRef.current = false;
      }
    },
    [appWindow],
  );

  useEffect(() => {
    const unlistenPromise = listen<ScreenshotPinWindowOpenedPayload>(
      "rtool://screenshot-pin-window/opened",
      (event) => {
        const payload = event.payload;
        if (!payload || payload.targetWindowLabel !== appWindow.label) {
          return;
        }
        try {
          baseSizeRef.current = {
            width: Math.max(1, payload.width),
            height: Math.max(1, payload.height),
          };
          currentScaleRef.current = 1;
          setImageSrc(convertFileSrc(payload.imagePath));
        } catch {
          setImageSrc(null);
        }
      },
    );

    return () => {
      safeResolveUnlisten(unlistenPromise, "screenshot-pin:window-opened");
    };
  }, [appWindow]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") {
        return;
      }
      event.preventDefault();
      hideWindow();
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [hideWindow]);

  return (
    <div
      className="relative h-full w-full overflow-hidden rounded-[10px] border border-white/35 bg-black/25 shadow-2xl"
      onMouseDown={handleDrag}
      onWheel={(event) => {
        void handleWheel(event);
      }}
    >
      {imageSrc ? (
        <img
          src={imageSrc}
          alt="Pinned screenshot"
          className="block h-full w-full select-none object-fill"
          draggable={false}
        />
      ) : null}

      <button
        type="button"
        data-screenshot-pin-close="true"
        className="absolute right-2 top-2 inline-flex h-6 w-6 items-center justify-center rounded bg-black/45 text-white/90 transition hover:bg-black/65"
        aria-label="Close pinned screenshot"
        title="Close"
        onClick={hideWindow}
      >
        <span className="btn-icon i-noto:cross-mark text-[0.95rem]" aria-hidden="true" />
      </button>
    </div>
  );
}

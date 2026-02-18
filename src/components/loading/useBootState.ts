import { useCallback, useEffect, useRef, useState, type MutableRefObject } from "react";

export interface UseBootStateOptions {
  cycleKey: number | string;
  ready: boolean;
  delayMs?: number;
  minVisibleMs?: number;
  maxWaitMs?: number;
  exitMs?: number;
}

interface UseBootStateResult {
  mounted: boolean;
  visible: boolean;
}

type BootPhase = "pending" | "visible" | "exiting" | "done";

function clearTimer(timerRef: MutableRefObject<number | null>) {
  if (timerRef.current === null) {
    return;
  }

  window.clearTimeout(timerRef.current);
  timerRef.current = null;
}

export function useBootState(options: UseBootStateOptions): UseBootStateResult {
  const delayMs = options.delayMs ?? 80;
  const minVisibleMs = options.minVisibleMs ?? 220;
  const maxWaitMs = options.maxWaitMs ?? 1200;
  const exitMs = options.exitMs ?? 160;

  const [mounted, setMounted] = useState(false);
  const [visible, setVisible] = useState(false);

  const cycleIdRef = useRef(0);
  const phaseRef = useRef<BootPhase>("done");
  const shownAtRef = useRef(0);
  const readyRef = useRef(options.ready);

  const delayTimerRef = useRef<number | null>(null);
  const minVisibleTimerRef = useRef<number | null>(null);
  const maxWaitTimerRef = useRef<number | null>(null);
  const exitTimerRef = useRef<number | null>(null);

  const clearAllTimers = useCallback(() => {
    clearTimer(delayTimerRef);
    clearTimer(minVisibleTimerRef);
    clearTimer(maxWaitTimerRef);
    clearTimer(exitTimerRef);
  }, []);

  const startExit = useCallback(
    (cycleId: number) => {
      if (cycleIdRef.current !== cycleId || phaseRef.current !== "visible") {
        return;
      }

      phaseRef.current = "exiting";
      setVisible(false);

      exitTimerRef.current = window.setTimeout(() => {
        if (cycleIdRef.current !== cycleId) {
          return;
        }

        phaseRef.current = "done";
        setMounted(false);
      }, exitMs);
    },
    [exitMs],
  );

  const completeCycle = useCallback(
    (cycleId: number) => {
      if (cycleIdRef.current !== cycleId) {
        return;
      }

      if (phaseRef.current === "done" || phaseRef.current === "exiting") {
        return;
      }

      clearTimer(delayTimerRef);
      clearTimer(maxWaitTimerRef);

      if (phaseRef.current === "pending") {
        phaseRef.current = "done";
        setVisible(false);
        setMounted(false);
        return;
      }

      const elapsed = Date.now() - shownAtRef.current;
      const remain = Math.max(0, minVisibleMs - elapsed);

      if (remain === 0) {
        startExit(cycleId);
        return;
      }

      minVisibleTimerRef.current = window.setTimeout(() => {
        startExit(cycleId);
      }, remain);
    },
    [minVisibleMs, startExit],
  );

  useEffect(() => {
    readyRef.current = options.ready;
  }, [options.ready]);

  useEffect(() => {
    cycleIdRef.current += 1;
    const cycleId = cycleIdRef.current;

    clearAllTimers();

    phaseRef.current = "pending";
    shownAtRef.current = 0;
    readyRef.current = options.ready;
    setMounted(false);
    setVisible(false);

    if (options.ready) {
      phaseRef.current = "done";
      return () => {
        clearAllTimers();
      };
    }

    delayTimerRef.current = window.setTimeout(() => {
      if (cycleIdRef.current !== cycleId || phaseRef.current !== "pending") {
        return;
      }

      if (readyRef.current) {
        completeCycle(cycleId);
        return;
      }

      shownAtRef.current = Date.now();
      phaseRef.current = "visible";
      setMounted(true);
      setVisible(true);
    }, delayMs);

    maxWaitTimerRef.current = window.setTimeout(() => {
      completeCycle(cycleId);
    }, maxWaitMs);

    return () => {
      clearAllTimers();
    };
  }, [options.cycleKey, options.ready, delayMs, maxWaitMs, clearAllTimers, completeCycle]);

  useEffect(() => {
    if (!options.ready) {
      return;
    }

    completeCycle(cycleIdRef.current);
  }, [options.ready, completeCycle]);

  useEffect(() => {
    return () => {
      clearAllTimers();
    };
  }, [clearAllTimers]);

  return {
    mounted,
    visible,
  };
}

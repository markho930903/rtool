import { useMemo, useRef, useEffect, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { Message } from "@ui/message/Message";
import type { MessagePlacement, MessageToastItem } from "@ui/message/types";
import { cx } from "@/components/ui/utils";

const placementClassMap: Record<MessagePlacement, string> = {
  "top-left": "top-4 left-4 items-start",
  "top-center": "top-4 left-1/2 -translate-x-1/2 items-center",
  "top-right": "top-4 right-4 items-end",
  "bottom-left": "bottom-4 left-4 items-start",
  "bottom-center": "bottom-4 left-1/2 -translate-x-1/2 items-center",
  "bottom-right": "bottom-4 right-4 items-end",
};

const placementOrder: MessagePlacement[] = [
  "top-left",
  "top-center",
  "top-right",
  "bottom-left",
  "bottom-center",
  "bottom-right",
];

interface ToastItemProps {
  item: MessageToastItem;
  closeLabel: string;
  onClose: (key: string) => void;
}

function ToastItem(props: ToastItemProps): ReactElement {
  const { item, onClose, closeLabel } = props;
  const timerRef = useRef<number | null>(null);
  const startedAtRef = useRef<number>(0);
  const remainingMsRef = useRef<number | null>(item.duration);

  useEffect(() => {
    const clearTimer = () => {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };

    const scheduleTimer = () => {
      if (item.duration === null || item.duration <= 0) {
        remainingMsRef.current = item.duration;
        return;
      }

      if (remainingMsRef.current === null || remainingMsRef.current <= 0) {
        remainingMsRef.current = item.duration;
      }

      const remaining = remainingMsRef.current;
      if (remaining === null || remaining <= 0) {
        return;
      }

      startedAtRef.current = Date.now();
      timerRef.current = window.setTimeout(() => {
        onClose(item.key);
      }, remaining);
    };

    remainingMsRef.current = item.duration;
    clearTimer();
    scheduleTimer();

    return clearTimer;
  }, [item.duration, item.key, onClose]);

  const pauseIfNeeded = () => {
    if (!item.pauseOnHover || item.duration === null || item.duration <= 0) {
      return;
    }

    if (timerRef.current === null) {
      return;
    }

    window.clearTimeout(timerRef.current);
    timerRef.current = null;

    const elapsed = Date.now() - startedAtRef.current;
    const remaining = Math.max(0, (remainingMsRef.current ?? item.duration) - elapsed);
    remainingMsRef.current = remaining;
  };

  const resumeIfNeeded = () => {
    if (!item.pauseOnHover || item.duration === null || item.duration <= 0) {
      return;
    }

    if (timerRef.current !== null) {
      return;
    }

    const remaining = remainingMsRef.current;
    if (remaining === null || remaining <= 0) {
      onClose(item.key);
      return;
    }

    startedAtRef.current = Date.now();
    timerRef.current = window.setTimeout(() => {
      onClose(item.key);
    }, remaining);
  };

  return (
    <div className="pointer-events-auto transition-transform duration-180 ease-out" onMouseEnter={pauseIfNeeded} onMouseLeave={resumeIfNeeded}>
      <Message
        mode="toast"
        type={item.type}
        title={item.title}
        description={item.description}
        content={item.content}
        icon={item.icon}
        showIcon={item.showIcon}
        closable={item.closable}
        closeLabel={item.closeLabel ?? closeLabel}
        className={item.className}
        style={item.style}
        role={item.role}
        actions={item.actions}
        onClose={() => onClose(item.key)}
      />
    </div>
  );
}

export interface MessageViewportProps {
  messages: MessageToastItem[];
  closeLabel: string;
  className?: string;
  onClose: (key: string) => void;
}

export function MessageViewport(props: MessageViewportProps): ReactElement | null {
  const { messages, closeLabel, className, onClose } = props;

  const grouped = useMemo(() => {
    const map: Record<MessagePlacement, MessageToastItem[]> = {
      "top-left": [],
      "top-center": [],
      "top-right": [],
      "bottom-left": [],
      "bottom-center": [],
      "bottom-right": [],
    };

    for (const item of messages) {
      map[item.placement].push(item);
    }

    for (const placement of placementOrder) {
      map[placement].sort((left, right) => {
        if (left.priority !== right.priority) {
          return right.priority - left.priority;
        }

        return right.createdAt - left.createdAt;
      });
    }

    return map;
  }, [messages]);

  if (typeof document === "undefined") {
    return null;
  }

  return createPortal(
    <>
      {placementOrder.map((placement) => {
        const items = grouped[placement];
        if (items.length === 0) {
          return null;
        }

        return (
          <div
            key={placement}
            className={cx(
              "pointer-events-none fixed z-[95] flex w-[min(420px,calc(100vw-1rem))] max-w-full flex-col gap-2",
              placementClassMap[placement],
              className,
            )}
          >
            {items.map((item) => (
              <ToastItem key={item.key} item={item} closeLabel={closeLabel} onClose={onClose} />
            ))}
          </div>
        );
      })}
    </>,
    document.body,
  );
}

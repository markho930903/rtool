import {
  createContext,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactElement,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";

import { bindMessageApi, message as messageSingleton, normalizeMessageOpenInput } from "@ui/message/api";
import { MessageViewport } from "@ui/message/MessageViewport";
import type {
  MessageApi,
  MessageOpenInput,
  MessageOpenOptions,
  MessagePlacement,
  MessageProviderConfig,
  MessageToastItem,
  MessageType,
} from "@ui/message/types";

interface MessageContextValue {
  api: MessageApi;
}

const DEFAULT_MAX_COUNT = 6;

const defaultConfig: Required<MessageProviderConfig> = {
  maxCount: DEFAULT_MAX_COUNT,
  defaultPlacement: "top-right",
  defaultDuration: 3000,
  defaultPauseOnHover: true,
  defaultClosable: true,
  defaultCloseLabel: "Close",
};

let messageIdSeed = 0;

function createMessageId(): string {
  messageIdSeed += 1;
  return `message-${Date.now()}-${messageIdSeed}`;
}

function ensureToastType(type: MessageType | undefined): MessageType {
  return type ?? "info";
}

function sanitizeDuration(value: number | null | undefined, fallback: number | null): number | null {
  if (value === null) {
    return null;
  }

  if (value === undefined) {
    return fallback;
  }

  if (!Number.isFinite(value) || value < 0) {
    return fallback;
  }

  return value;
}

function applyQueueLimit(nextItems: MessageToastItem[], maxCount: number): MessageToastItem[] {
  if (maxCount <= 0 || nextItems.length <= maxCount) {
    return nextItems;
  }

  const overflow = nextItems.length - maxCount;
  const removalCandidates = [...nextItems].sort((left, right) => {
    if (left.priority !== right.priority) {
      return left.priority - right.priority;
    }

    return left.createdAt - right.createdAt;
  });

  const toRemove = new Set(removalCandidates.slice(0, overflow).map((item) => item.key));
  return nextItems.filter((item) => !toRemove.has(item.key));
}

function mergeItem(
  base: MessageToastItem | null,
  input: MessageOpenOptions,
  config: Required<MessageProviderConfig>,
  forcedKey?: string,
): MessageToastItem {
  const createdAt = base?.createdAt ?? Date.now();
  const key = forcedKey ?? input.key ?? base?.key ?? createMessageId();
  const closeLabel = input.closeLabel ?? base?.closeLabel ?? config.defaultCloseLabel;
  const content = input.content ?? base?.content;
  const description = input.description ?? (input.content !== undefined ? input.content : base?.description);

  return {
    key,
    mode: "toast",
    type: ensureToastType(input.type ?? base?.type),
    title: input.title ?? base?.title,
    description,
    content,
    icon: input.icon ?? base?.icon,
    showIcon: input.showIcon ?? base?.showIcon ?? true,
    closable: input.closable ?? base?.closable ?? config.defaultClosable,
    closeLabel,
    className: input.className ?? base?.className,
    style: input.style ?? base?.style,
    role: input.role ?? base?.role,
    actions: input.actions ?? base?.actions,
    placement: (input.placement ?? base?.placement ?? config.defaultPlacement) as MessagePlacement,
    duration: sanitizeDuration(input.duration, base?.duration ?? config.defaultDuration),
    pauseOnHover: input.pauseOnHover ?? base?.pauseOnHover ?? config.defaultPauseOnHover,
    priority: input.priority ?? base?.priority ?? 0,
    dedupeKey: input.dedupeKey ?? base?.dedupeKey,
    onClose: input.onClose ?? base?.onClose,
    createdAt,
  };
}

function removeAndNotify(items: MessageToastItem[], key: string): MessageToastItem[] {
  const target = items.find((item) => item.key === key);
  if (target?.onClose) {
    queueMicrotask(() => target.onClose?.());
  }

  return items.filter((item) => item.key !== key);
}

export const MessageContext = createContext<MessageContextValue | null>(null);

export interface MessageProviderProps extends MessageProviderConfig {
  children: ReactNode;
  viewport?: boolean;
  viewportClassName?: string;
}

export function MessageProvider(props: MessageProviderProps): ReactElement {
  const { t } = useTranslation("common");
  const {
    children,
    viewport = true,
    viewportClassName,
    maxCount = defaultConfig.maxCount,
    defaultPlacement = defaultConfig.defaultPlacement,
    defaultDuration = defaultConfig.defaultDuration,
    defaultPauseOnHover = defaultConfig.defaultPauseOnHover,
    defaultClosable = defaultConfig.defaultClosable,
    defaultCloseLabel,
  } = props;
  const resolvedDefaultCloseLabel = defaultCloseLabel ?? t("message.close");

  const config = useMemo(
    () => ({
      maxCount,
      defaultPlacement,
      defaultDuration,
      defaultPauseOnHover,
      defaultClosable,
      defaultCloseLabel: resolvedDefaultCloseLabel,
    }),
    [
      defaultClosable,
      defaultDuration,
      defaultPauseOnHover,
      defaultPlacement,
      maxCount,
      resolvedDefaultCloseLabel,
    ],
  );

  const [messages, setMessages] = useState<MessageToastItem[]>([]);
  const messagesRef = useRef<MessageToastItem[]>([]);

  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  const open = useCallback(
    (input: MessageOpenInput): string => {
      const options = normalizeMessageOpenInput(input);

      let resolvedKey = options.key ?? createMessageId();

      setMessages((previous) => {
        const dedupeTarget = options.dedupeKey
          ? previous.find((item) => item.dedupeKey === options.dedupeKey)
          : undefined;

        const key = dedupeTarget?.key ?? options.key ?? resolvedKey;
        resolvedKey = key;

        const existing = previous.find((item) => item.key === key) ?? null;
        const nextItem = mergeItem(existing, options, config, key);

        const filtered = previous.filter((item) => item.key !== key);
        return applyQueueLimit([...filtered, nextItem], config.maxCount);
      });

      return resolvedKey;
    },
    [config],
  );

  const update = useCallback(
    (key: string, input: MessageOpenInput): boolean => {
      if (!messagesRef.current.some((item) => item.key === key)) {
        return false;
      }

      const options = normalizeMessageOpenInput(input);
      setMessages((previous) =>
        previous.map((item) => {
          if (item.key !== key) {
            return item;
          }

          return mergeItem(item, { ...options, key }, config, key);
        }),
      );

      return true;
    },
    [config],
  );

  const close = useCallback((key: string) => {
    setMessages((previous) => removeAndNotify(previous, key));
  }, []);

  const destroy = useCallback(() => {
    const current = messagesRef.current;
    for (const item of current) {
      if (item.onClose) {
        queueMicrotask(() => item.onClose?.());
      }
    }

    setMessages([]);
  }, []);

  const success = useCallback((input: MessageOpenInput) => open({ ...normalizeMessageOpenInput(input), type: "success" }), [open]);

  const info = useCallback((input: MessageOpenInput) => open({ ...normalizeMessageOpenInput(input), type: "info" }), [open]);

  const warning = useCallback((input: MessageOpenInput) => open({ ...normalizeMessageOpenInput(input), type: "warning" }), [open]);

  const error = useCallback((input: MessageOpenInput) => open({ ...normalizeMessageOpenInput(input), type: "error" }), [open]);

  const loading = useCallback(
    (input: MessageOpenInput) => {
      const options = normalizeMessageOpenInput(input);
      return open({
        ...options,
        type: "loading",
        duration: options.duration ?? null,
      });
    },
    [open],
  );

  const promise: MessageApi["promise"] = useCallback(
    async (promiseLike, options) => messageSingleton.promise(promiseLike, options),
    [],
  );

  const api = useMemo<MessageApi>(
    () => ({
      open,
      update,
      close,
      destroy,
      success,
      info,
      warning,
      error,
      loading,
      promise,
    }),
    [close, destroy, error, info, loading, open, promise, success, update, warning],
  );

  useEffect(() => bindMessageApi(api), [api]);

  return (
    <MessageContext.Provider value={{ api }}>
      {children}
      {viewport ? (
        <MessageViewport
          messages={messages}
          closeLabel={resolvedDefaultCloseLabel}
          className={viewportClassName}
          onClose={close}
        />
      ) : null}
    </MessageContext.Provider>
  );
}

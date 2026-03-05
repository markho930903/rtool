import { isValidElement, type ReactNode } from "react";

import type {
  MessageApi,
  MessageOpenInput,
  MessageOpenOptions,
  MessagePromiseOptions,
  MessagePromiseSource,
  MessagePromiseStage,
  MessageType,
} from "@ui/message/types";

let activeApi: MessageApi | null = null;
let keySeed = 0;
let warnedNoProvider = false;

function createFallbackKey(): string {
  keySeed += 1;
  return `message-${Date.now()}-${keySeed}`;
}

function getActiveApi(): MessageApi | null {
  if (activeApi) {
    return activeApi;
  }

  if (!warnedNoProvider && typeof window !== "undefined" && import.meta.env.DEV) {
    warnedNoProvider = true;
    console.warn("[message] MessageProvider is not mounted. Call is ignored.");
  }

  return null;
}

function isMessageOptions(value: unknown): value is MessageOpenOptions {
  if (!value || typeof value !== "object") {
    return false;
  }

  if (isValidElement(value)) {
    return false;
  }

  return true;
}

export function normalizeMessageOpenInput(input: MessageOpenInput): MessageOpenOptions {
  if (isMessageOptions(input)) {
    return input;
  }

  return {
    description: input as ReactNode,
  };
}

function resolveStage<TValue>(
  stage: MessagePromiseStage<TValue>,
  value: TValue,
  fallbackType: MessageType,
): MessageOpenOptions | null {
  if (stage === undefined) {
    return null;
  }

  const rawResult = typeof stage === "function" ? stage(value) : stage;
  const options = normalizeMessageOpenInput(rawResult);

  return {
    ...options,
    type: options.type ?? fallbackType,
  };
}

function withType(input: MessageOpenInput, type: MessageType): MessageOpenOptions {
  const options = normalizeMessageOpenInput(input);
  return {
    ...options,
    type,
  };
}

function openWithType(input: MessageOpenInput, type: MessageType): string {
  return message.open(withType(input, type));
}

async function resolveSource<T>(source: MessagePromiseSource<T>): Promise<T> {
  if (typeof source === "function") {
    return source();
  }

  return source;
}

export function bindMessageApi(api: MessageApi | null): () => void {
  activeApi = api;

  return () => {
    if (activeApi === api) {
      activeApi = null;
    }
  };
}

export const message: MessageApi = {
  open(input) {
    const api = getActiveApi();
    if (!api) {
      return createFallbackKey();
    }

    return api.open(input);
  },

  update(key, input) {
    const api = getActiveApi();
    if (!api) {
      return false;
    }

    return api.update(key, input);
  },

  close(key) {
    const api = getActiveApi();
    if (!api) {
      return;
    }

    api.close(key);
  },

  destroy() {
    const api = getActiveApi();
    if (!api) {
      return;
    }

    api.destroy();
  },

  success(input) {
    return openWithType(input, "success");
  },

  info(input) {
    return openWithType(input, "info");
  },

  warning(input) {
    return openWithType(input, "warning");
  },

  error(input) {
    return openWithType(input, "error");
  },

  loading(input) {
    const options = withType(input, "loading");
    if (options.duration === undefined) {
      options.duration = null;
    }
    return message.open(options);
  },

  async promise<T>(promiseLike: MessagePromiseSource<T>, options?: MessagePromiseOptions<T>): Promise<T> {
    const loadingStage = resolveStage(options?.loading ?? { description: "Loading..." }, undefined, "loading");
    const messageKey =
      options?.key ?? (loadingStage ? message.open({ ...loadingStage, duration: loadingStage.duration ?? null }) : createFallbackKey());

    try {
      const result = await resolveSource(promiseLike);
      const successStage = resolveStage(options?.success, result, "success");

      if (successStage) {
        const applied = message.update(messageKey, {
          ...successStage,
          duration: successStage.duration ?? 3000,
        });

        if (!applied) {
          message.open({
            ...successStage,
            key: messageKey,
            duration: successStage.duration ?? 3000,
          });
        }
      } else {
        message.close(messageKey);
      }

      return result;
    } catch (error) {
      const fallbackDescription = error instanceof Error ? error.message : String(error);
      const errorStage = resolveStage(options?.error ?? fallbackDescription, error, "error");

      if (errorStage) {
        const applied = message.update(messageKey, {
          ...errorStage,
          duration: errorStage.duration ?? 5000,
        });

        if (!applied) {
          message.open({
            ...errorStage,
            key: messageKey,
            duration: errorStage.duration ?? 5000,
          });
        }
      } else {
        message.close(messageKey);
      }

      throw error;
    }
  },
};

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { logDebug, logError, logInfo } from "@/services/logger";

interface InvokeWithLogOptions {
  silent?: boolean;
}

function createRequestId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }

  return `req-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function resolveWindowLabel(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    return "unknown";
  }
}

export async function invokeWithLog<T>(
  command: string,
  payload?: Record<string, unknown>,
  options?: InvokeWithLogOptions,
): Promise<T> {
  const requestId = createRequestId();
  const windowLabel = resolveWindowLabel();
  const startedAt = typeof performance !== "undefined" ? performance.now() : Date.now();

  const requestPayload: Record<string, unknown> = {
    ...(payload ?? {}),
    requestId,
    windowLabel,
  };

  if (!options?.silent) {
    logDebug(
      "invoke",
      "command_start",
      {
        command,
        requestId,
        windowLabel,
      },
      requestId,
    );
  }

  try {
    const result = await invoke<T>(command, requestPayload);

    if (!options?.silent) {
      const endedAt = typeof performance !== "undefined" ? performance.now() : Date.now();
      logInfo(
        "invoke",
        "command_end",
        {
          command,
          requestId,
          windowLabel,
          ok: true,
          durationMs: Math.round(endedAt - startedAt),
        },
        requestId,
      );
    }

    return result;
  } catch (error) {
    const endedAt = typeof performance !== "undefined" ? performance.now() : Date.now();
    const message = error instanceof Error ? error.message : String(error);
    logError(
      "invoke",
      "command_failed",
      {
        command,
        requestId,
        windowLabel,
        durationMs: Math.round(endedAt - startedAt),
        error: message,
      },
      requestId,
    );
    throw error;
  }
}

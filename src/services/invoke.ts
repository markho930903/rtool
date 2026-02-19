import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { logDebug, logError, logInfo } from "@/services/logger";

interface InvokeWithLogOptions {
  silent?: boolean;
}

interface InvokeErrorContextItem {
  key: string;
  value: string;
}

interface InvokeErrorPayload {
  code?: string;
  message?: string;
  causes?: string[];
  context?: InvokeErrorContextItem[];
  requestId?: string;
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

function isObjectRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function tryParseJsonRecord(value: string): Record<string, unknown> | null {
  const normalized = value.trim();
  if (!normalized.startsWith("{") || !normalized.endsWith("}")) {
    return null;
  }

  try {
    const parsed = JSON.parse(normalized);
    return isObjectRecord(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function toInvokeErrorPayload(value: unknown): InvokeErrorPayload | null {
  if (isObjectRecord(value)) {
    return value as InvokeErrorPayload;
  }

  if (value instanceof Error) {
    const fromMessage = tryParseJsonRecord(value.message);
    if (fromMessage) {
      return fromMessage as InvokeErrorPayload;
    }

    if ("cause" in value && isObjectRecord((value as Error & { cause?: unknown }).cause)) {
      return (value as Error & { cause?: InvokeErrorPayload }).cause ?? null;
    }
  }

  if (typeof value === "string") {
    const parsed = tryParseJsonRecord(value);
    if (parsed) {
      return parsed as InvokeErrorPayload;
    }
  }

  return null;
}

function resolveErrorMessage(error: unknown, payload: InvokeErrorPayload | null): string {
  if (payload?.message && payload.message.trim().length > 0) {
    return payload.message;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
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
    const payload = toInvokeErrorPayload(error);
    const message = resolveErrorMessage(error, payload);
    logError(
      "invoke",
      "command_failed",
      {
        command,
        requestId,
        windowLabel,
        durationMs: Math.round(endedAt - startedAt),
        error: message,
        errorCode: payload?.code ?? "unknown_error",
        errorCauses: payload?.causes ?? [],
        errorContext: payload?.context ?? [],
      },
      requestId,
    );
    throw error;
  }
}

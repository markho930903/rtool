import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import type {
  AppFeatureKey,
  AppFeatureRequestMap,
} from "@/contracts";
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

const FEATURE_COMMAND_MAP: Record<AppFeatureKey, string> = {
  app_manager: "rt_app_manager",
  clipboard: "rt_clipboard",
  launcher: "rt_launcher",
  locale: "rt_locale",
  logging: "rt_logging",
  screenshot: "rt_screenshot",
  settings: "rt_settings",
};

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

export async function invokeWithLog<T, F extends AppFeatureKey = AppFeatureKey>(
  feature: F,
  request: AppFeatureRequestMap[F],
  options?: InvokeWithLogOptions,
): Promise<T> {
  const command = FEATURE_COMMAND_MAP[feature];
  const action = (request as { kind: string }).kind;
  const requestId = createRequestId();
  const windowLabel = resolveWindowLabel();
  const startedAt = typeof performance !== "undefined" ? performance.now() : Date.now();

  const requestPayload = {
    request,
    meta: {
      requestId,
      windowLabel,
    },
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
          feature,
          action,
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
    const invokeErrorPayload = toInvokeErrorPayload(error);
    const message = resolveErrorMessage(error, invokeErrorPayload);
    logError(
      "invoke",
      "command_failed",
      {
        command,
        feature,
        action,
        requestId,
        windowLabel,
        durationMs: Math.round(endedAt - startedAt),
        error: message,
        errorCode: invokeErrorPayload?.code ?? "unknown_error",
        errorCauses: invokeErrorPayload?.causes ?? [],
        errorContext: invokeErrorPayload?.context ?? [],
      },
      requestId,
    );
    throw error;
  }
}

export async function invokeFeature<T, F extends AppFeatureKey = AppFeatureKey>(
  feature: F,
  request: AppFeatureRequestMap[F],
  options?: InvokeWithLogOptions,
): Promise<T> {
  return invokeWithLog<T, F>(feature, request, options);
}

import { logWarn } from "@/services/logger";

export type RecoverableResult<T> = { ok: true; data: T } | { ok: false; message: string };

export interface RecoverableContext {
  scope: string;
  action: string;
  message?: string;
  metadata?: Record<string, unknown>;
  silent?: boolean;
}

function normalizeErrorMessage(error: unknown, fallback?: string): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string" && error.trim().length > 0) {
    return error;
  }

  return fallback ?? "unknown error";
}

function logRecoverableError(context: RecoverableContext, message: string, error: unknown) {
  if (context.silent) {
    return;
  }

  logWarn(
    context.scope,
    `${context.action}_recoverable_failed`,
    {
      ...(context.metadata ?? {}),
      error: message,
      rawError: error instanceof Error ? error.name : String(error),
    },
  );
}

export async function runRecoverable<T>(
  task: () => Promise<T>,
  context: RecoverableContext,
): Promise<RecoverableResult<T>> {
  try {
    const data = await task();
    return { ok: true, data };
  } catch (error) {
    const message = normalizeErrorMessage(error, context.message);
    logRecoverableError(context, message, error);
    return {
      ok: false,
      message,
    };
  }
}

export function runRecoverableSync<T>(
  task: () => T,
  context: RecoverableContext,
): RecoverableResult<T> {
  try {
    const data = task();
    return { ok: true, data };
  } catch (error) {
    const message = normalizeErrorMessage(error, context.message);
    logRecoverableError(context, message, error);
    return {
      ok: false,
      message,
    };
  }
}

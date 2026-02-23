import { invoke } from "@tauri-apps/api/core";

type LogLevel = "debug" | "info" | "warn" | "error";

const MAX_STRING_LEN = 256;
const MAX_MESSAGE_LEN = 2048;
const MAX_COLLECTION_ITEMS = 64;
const CLIENT_LOG_HIGH_FREQ_WINDOW_MS = 1000;
const CLIENT_LOG_HIGH_FREQ_MAX_PER_KEY = 20;
const CLIENT_LOG_HIGH_FREQ_GC_WINDOW_MS = 10_000;
const SENSITIVE_TEXT_KEYS = ["text", "content", "clipboard", "prompt", "input"];
const SENSITIVE_PATH_KEYS = ["path", "file", "filepath", "filename"];
const SENSITIVE_HOST_KEYS = ["host", "hostname"];

interface HighFrequencyWindow {
  startedAt: number;
  count: number;
}

const highFrequencyWindows = new Map<string, HighFrequencyWindow>();
let lastHighFrequencyGcAt = 0;

function hashString(value: string): string {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash += (hash << 1) + (hash << 4) + (hash << 7) + (hash << 8) + (hash << 24);
  }
  return (hash >>> 0).toString(16);
}

function truncateString(value: string, maxLen: number): string {
  if (value.length <= maxLen) {
    return value;
  }
  return `${value.slice(0, maxLen)}...(truncated,len=${value.length})`;
}

function looksLikePath(value: string): boolean {
  return (
    value.startsWith("file://") ||
    value.startsWith("~/") ||
    value.startsWith("/") ||
    value.includes(":\\") ||
    value.includes("\\")
  );
}

function sanitizePath(value: string): string {
  const normalized = value.trim().replace(/^["']|["']$/g, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  const baseName = parts.length > 0 ? parts[parts.length - 1] : "unknown";
  return `[path:${baseName} dir_hash=${hashString(normalized)}]`;
}

function sanitizeString(value: string): string {
  const normalized = value.trim();
  if (!normalized) {
    return "";
  }

  if (normalized.startsWith("data:")) {
    return `[data-url redacted len=${normalized.length} hash=${hashString(normalized)}]`;
  }

  if (looksLikePath(normalized)) {
    return sanitizePath(normalized);
  }

  return truncateString(normalized, MAX_STRING_LEN);
}

function containsKeyword(key: string | undefined, candidates: string[]): boolean {
  if (!key) {
    return false;
  }
  const normalized = key.toLowerCase();
  return candidates.some((candidate) => normalized.includes(candidate));
}

function sanitizeValue(value: unknown, key?: string, depth = 0): unknown {
  if (depth > 6) {
    return "[max-depth-reached]";
  }

  if (value === null || value === undefined) {
    return value;
  }

  if (typeof value === "string") {
    if (containsKeyword(key, SENSITIVE_TEXT_KEYS)) {
      return `[redacted-text len=${value.length} hash=${hashString(value)}]`;
    }
    if (containsKeyword(key, SENSITIVE_PATH_KEYS)) {
      return sanitizePath(value);
    }
    if (containsKeyword(key, SENSITIVE_HOST_KEYS)) {
      return `[host hash=${hashString(value)}]`;
    }
    return sanitizeString(value);
  }

  if (typeof value === "number" || typeof value === "boolean") {
    return value;
  }

  if (Array.isArray(value)) {
    return value.slice(0, MAX_COLLECTION_ITEMS).map((item) => sanitizeValue(item, key, depth + 1));
  }

  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    const result: Record<string, unknown> = {};

    entries.slice(0, MAX_COLLECTION_ITEMS).forEach(([field, item]) => {
      result[field] = sanitizeValue(item, field, depth + 1);
    });

    if (entries.length > MAX_COLLECTION_ITEMS) {
      result._truncated = `${entries.length - MAX_COLLECTION_ITEMS} keys truncated`;
    }

    return result;
  }

  return String(value);
}

function canUseConsole(level: LogLevel): boolean {
  if (import.meta.env.DEV) {
    return true;
  }
  return level === "warn" || level === "error";
}

function shouldEmitClientLog(level: LogLevel, scope: string, message: string): boolean {
  if (level === "debug") {
    return false;
  }

  const now = Date.now();
  if (now - lastHighFrequencyGcAt >= CLIENT_LOG_HIGH_FREQ_GC_WINDOW_MS) {
    highFrequencyWindows.forEach((window, key) => {
      if (now - window.startedAt >= CLIENT_LOG_HIGH_FREQ_GC_WINDOW_MS) {
        highFrequencyWindows.delete(key);
      }
    });
    lastHighFrequencyGcAt = now;
  }

  const eventKey = `${level}|${scope}|${message}`;
  const current = highFrequencyWindows.get(eventKey);
  if (!current || now - current.startedAt >= CLIENT_LOG_HIGH_FREQ_WINDOW_MS) {
    highFrequencyWindows.set(eventKey, {
      startedAt: now,
      count: 1,
    });
    return true;
  }

  const nextCount = current.count + 1;
  highFrequencyWindows.set(eventKey, {
    startedAt: current.startedAt,
    count: nextCount,
  });

  return nextCount <= CLIENT_LOG_HIGH_FREQ_MAX_PER_KEY;
}

function writeConsole(level: LogLevel, scope: string, message: string, metadata?: unknown, requestId?: string) {
  if (!canUseConsole(level)) {
    return;
  }

  const payload = metadata === undefined ? undefined : sanitizeValue(metadata);
  const prefix = `[${level}] [${scope}]${requestId ? ` [${requestId}]` : ""} ${message}`;

  if (level === "error") {
    console.error(prefix, payload);
    return;
  }
  if (level === "warn") {
    console.warn(prefix, payload);
    return;
  }
  if (level === "info") {
    console.info(prefix, payload);
    return;
  }
  console.debug(prefix, payload);
}

async function emitClientLog(level: LogLevel, scope: string, message: string, metadata?: unknown, requestId?: string) {
  if (!shouldEmitClientLog(level, scope, message)) {
    return;
  }

  const sanitizedMessage = truncateString(sanitizeString(message), MAX_MESSAGE_LEN);
  const sanitizedMetadata = metadata === undefined ? undefined : sanitizeValue(metadata);

  try {
    await invoke("client_log", {
      level,
      scope: sanitizeString(scope),
      message: sanitizedMessage,
      metadata: sanitizedMetadata,
      requestId,
    });
  } catch {
    // 防止日志上报失败导致业务逻辑受影响
  }
}

function log(level: LogLevel, scope: string, message: string, metadata?: unknown, requestId?: string) {
  writeConsole(level, scope, message, metadata, requestId);
  void emitClientLog(level, scope, message, metadata, requestId);
}

export function logDebug(scope: string, message: string, metadata?: unknown, requestId?: string) {
  log("debug", scope, message, metadata, requestId);
}

export function logInfo(scope: string, message: string, metadata?: unknown, requestId?: string) {
  log("info", scope, message, metadata, requestId);
}

export function logWarn(scope: string, message: string, metadata?: unknown, requestId?: string) {
  log("warn", scope, message, metadata, requestId);
}

export function logError(scope: string, message: string, metadata?: unknown, requestId?: string) {
  log("error", scope, message, metadata, requestId);
}

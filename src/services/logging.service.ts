import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { LogConfigDto, LogEntryDto, LogPageDto, LogQueryDto, LoggingRequestDto } from "@/contracts";
import { invokeFeature } from "@/services/invoke";
import { safeUnlisten } from "@/services/tauri-event";

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

export interface LogEntry {
  id: number;
  timestamp: number;
  level: LogLevel;
  scope: string;
  event: string;
  requestId: string;
  windowLabel: string | null;
  message: string;
  metadata: Record<string, unknown> | null;
  rawRef: string | null;
  aggregatedCount: number | null;
}

export interface LogQuery {
  cursor?: string;
  limit?: number;
  levels?: LogLevel[];
  scope?: string;
  requestId?: string;
  windowLabel?: string;
  keyword?: string;
  startAt?: number;
  endAt?: number;
}

export interface LogPage {
  items: LogEntry[];
  nextCursor: string | null;
}

export interface LoggingConfig {
  minLevel: LogLevel;
  keepDays: number;
  realtimeEnabled: boolean;
  highFreqWindowMs: number;
  highFreqMaxPerKey: number;
  allowRawView: boolean;
}

function invokeLogging<T>(
  request: LoggingRequestDto,
  silent = true,
): Promise<T> {
  return invokeFeature<T>("logging", request, { silent });
}

export async function fetchLogPage(query?: LogQuery): Promise<LogPage> {
  const dto = await invokeLogging<LogPageDto>({
    kind: "query",
    payload: { query: query as LogQueryDto | undefined },
  });
  return dto as LogPage;
}

export async function fetchLoggingConfig(): Promise<LoggingConfig> {
  const dto = await invokeLogging<LogConfigDto>({ kind: "get_config" });
  return dto as LoggingConfig;
}

export async function saveLoggingConfig(config: LoggingConfig): Promise<LoggingConfig> {
  const dto = await invokeLogging<LogConfigDto>({
    kind: "update_config",
    payload: { config: config as LogConfigDto },
  });
  return dto as LoggingConfig;
}

export async function exportLogs(query?: LogQuery, outputPath?: string): Promise<string> {
  return invokeLogging<string>({
    kind: "export_jsonl",
    payload: {
      query: query as LogQueryDto | undefined,
      outputPath,
    },
  });
}

export async function subscribeLogStream(onEntry: (entry: LogEntry) => void): Promise<UnlistenFn> {
  const unlisten = await listen<LogEntryDto>("rtool://logging/stream", (event) => {
    onEntry(event.payload as LogEntry);
  });

  return () => {
    safeUnlisten(unlisten, "logging-stream");
  };
}

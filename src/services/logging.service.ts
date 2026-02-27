import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { CommandRequestDto, LogConfigDto, LogEntryDto, LogPageDto, LogQueryDto } from "@/contracts";
import { invokeWithLog } from "@/services/invoke";
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
  kind: string,
  payload?: Record<string, unknown>,
  silent = true,
): Promise<T> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload;
  }
  return invokeWithLog<T>(
    "logging_handle",
    {
      request,
    },
    {
      silent,
    },
  );
}

export async function fetchLogPage(query?: LogQuery): Promise<LogPage> {
  const dto = await invokeLogging<LogPageDto>("query", {
    query: query as LogQueryDto | undefined,
  });
  return dto as LogPage;
}

export async function fetchLoggingConfig(): Promise<LoggingConfig> {
  const dto = await invokeLogging<LogConfigDto>("get_config");
  return dto as LoggingConfig;
}

export async function saveLoggingConfig(config: LoggingConfig): Promise<LoggingConfig> {
  const dto = await invokeLogging<LogConfigDto>("update_config", {
    config: config as LogConfigDto,
  });
  return dto as LoggingConfig;
}

export async function exportLogs(query?: LogQuery, outputPath?: string): Promise<string> {
  return invokeLogging<string>("export_jsonl", {
    query: query as LogQueryDto | undefined,
    outputPath,
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

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { LogConfigDto, LogEntryDto, LogPageDto, LogQueryDto } from "@/contracts";
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

export async function fetchLogPage(query?: LogQuery): Promise<LogPage> {
  const dto = await invokeWithLog<LogPageDto>(
    "logging_query",
    {
      query: query as LogQueryDto | undefined,
    },
    {
      silent: true,
    },
  );
  return dto as LogPage;
}

export async function fetchLoggingConfig(): Promise<LoggingConfig> {
  const dto = await invokeWithLog<LogConfigDto>("logging_get_config", undefined, {
    silent: true,
  });
  return dto as LoggingConfig;
}

export async function saveLoggingConfig(config: LoggingConfig): Promise<LoggingConfig> {
  const dto = await invokeWithLog<LogConfigDto>(
    "logging_update_config",
    {
      config: config as LogConfigDto,
    },
    {
      silent: true,
    },
  );
  return dto as LoggingConfig;
}

export async function exportLogs(query?: LogQuery, outputPath?: string): Promise<string> {
  return invokeWithLog<string>(
    "logging_export_jsonl",
    {
      query: query as LogQueryDto | undefined,
      outputPath,
    },
    {
      silent: true,
    },
  );
}

export async function subscribeLogStream(onEntry: (entry: LogEntry) => void): Promise<UnlistenFn> {
  const unlisten = await listen<LogEntryDto>("rtool://logging/stream", (event) => {
    onEntry(event.payload as LogEntry);
  });

  return () => {
    safeUnlisten(unlisten, "logging-stream");
  };
}

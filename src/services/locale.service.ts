import type {
  CommandRequestDto,
  LocaleStateDto as BackendLocaleState,
} from "@/contracts";
import type { AppLocale, LocalePreference } from "@/i18n/types";
import { invokeWithLog } from "@/services/invoke";

export type { BackendLocaleState };

function invokeLocale<T>(kind: string, payload?: Record<string, unknown>, silent = true): Promise<T> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload;
  }
  return invokeWithLog<T>(
    "locale_handle",
    { request },
    {
      silent,
    },
  );
}

export function toAppLocale(locale: string): AppLocale {
  return locale as AppLocale;
}

export function toLocalePreference(preference: string): LocalePreference {
  return preference as LocalePreference;
}

export async function fetchBackendLocaleState(): Promise<BackendLocaleState> {
  return invokeLocale<BackendLocaleState>("get");
}

export async function saveBackendLocalePreference(preference: LocalePreference): Promise<BackendLocaleState> {
  return invokeLocale<BackendLocaleState>("set", { preference });
}

import type {
  LocaleRequestDto,
  LocaleStateDto as BackendLocaleState,
} from "@/contracts";
import type { AppLocale, LocalePreference } from "@/i18n/types";
import { invokeFeature } from "@/services/invoke";

export type { BackendLocaleState };

function invokeLocale<T>(request: LocaleRequestDto, silent = true): Promise<T> {
  return invokeFeature<T>("locale", request, { silent });
}

export function toAppLocale(locale: string): AppLocale {
  return locale as AppLocale;
}

export function toLocalePreference(preference: string): LocalePreference {
  return preference as LocalePreference;
}

export async function fetchBackendLocaleState(): Promise<BackendLocaleState> {
  return invokeLocale<BackendLocaleState>({ kind: "get" });
}

export async function saveBackendLocalePreference(preference: LocalePreference): Promise<BackendLocaleState> {
  return invokeLocale<BackendLocaleState>({ kind: "set", payload: { preference } });
}

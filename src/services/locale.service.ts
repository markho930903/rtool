import type {
  CommandRequestDto,
  ImportLocaleResult as ImportLocaleFileResult,
  LocaleCatalogList as BackendLocaleCatalogList,
  LocaleNamespaces,
  LocaleStateDto as BackendLocaleState,
  ReloadLocalesResult,
} from "@/contracts";
import type { AppLocale, LocalePreference } from "@/i18n/types";
import { invokeWithLog } from "@/services/invoke";

export type { BackendLocaleCatalogList, BackendLocaleState, ImportLocaleFileResult, ReloadLocalesResult };

export interface ImportLocaleFilePayload {
  locale: string;
  namespace: string;
  content: string;
  replace?: boolean;
}

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

function invokeI18nImport<T>(
  kind: string,
  payload?: Record<string, unknown>,
  silent = true,
): Promise<T> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload;
  }
  return invokeWithLog<T>(
    "i18n_import_handle",
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

export function mapLocaleNamespaces(input: LocaleNamespaces[]): LocaleNamespaces[] {
  return input;
}

export async function fetchBackendLocaleState(): Promise<BackendLocaleState> {
  return invokeLocale<BackendLocaleState>("get");
}

export async function saveBackendLocalePreference(preference: LocalePreference): Promise<BackendLocaleState> {
  return invokeLocale<BackendLocaleState>("set", { preference });
}

export async function listBackendLocales(): Promise<BackendLocaleCatalogList> {
  return invokeI18nImport<BackendLocaleCatalogList>("list_locales");
}

export async function reloadBackendLocales(): Promise<ReloadLocalesResult> {
  return invokeI18nImport<ReloadLocalesResult>("reload_locales");
}

export async function importBackendLocaleFile(payload: ImportLocaleFilePayload): Promise<ImportLocaleFileResult> {
  return invokeI18nImport<ImportLocaleFileResult>("import_locale_file", {
    locale: payload.locale,
    namespace: payload.namespace,
    content: payload.content,
    replace: payload.replace ?? true,
  });
}

import type { AppLocale, LocalePreference } from "@/i18n/types";
import type {
  ImportLocaleResult as ImportLocaleFileResult,
  LocaleCatalogList as BackendLocaleCatalogList,
  LocaleNamespaces,
  LocaleStateDto as BackendLocaleState,
  ReloadLocalesResult,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export type { BackendLocaleCatalogList, BackendLocaleState, ImportLocaleFileResult, ReloadLocalesResult };

export interface ImportLocaleFilePayload {
  locale: string;
  namespace: string;
  content: string;
  replace?: boolean;
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
  return invokeWithLog<BackendLocaleState>("app_get_locale", undefined, {
    silent: true,
  });
}

export async function saveBackendLocalePreference(preference: LocalePreference): Promise<BackendLocaleState> {
  return invokeWithLog<BackendLocaleState>(
    "app_set_locale",
    {
      preference,
    },
    {
      silent: true,
    },
  );
}

export async function listBackendLocales(): Promise<BackendLocaleCatalogList> {
  return invokeWithLog<BackendLocaleCatalogList>("app_list_locales", undefined, {
    silent: true,
  });
}

export async function reloadBackendLocales(): Promise<ReloadLocalesResult> {
  return invokeWithLog<ReloadLocalesResult>("app_reload_locales", undefined, {
    silent: true,
  });
}

export async function importBackendLocaleFile(payload: ImportLocaleFilePayload): Promise<ImportLocaleFileResult> {
  return invokeWithLog<ImportLocaleFileResult>(
    "app_import_locale_file",
    {
      locale: payload.locale,
      namespace: payload.namespace,
      content: payload.content,
      replace: payload.replace ?? true,
    },
    {
      silent: true,
    },
  );
}

import type { AppLocale, LocalePreference } from "@/i18n/types";
import { invokeWithLog } from "@/services/invoke";

export interface BackendLocaleState {
  preference: LocalePreference;
  resolved: AppLocale;
}

export interface LocaleNamespaces {
  locale: string;
  namespaces: string[];
}

export interface BackendLocaleCatalogList {
  builtinLocales: LocaleNamespaces[];
  overlayLocales: LocaleNamespaces[];
  effectiveLocales: LocaleNamespaces[];
}

export interface ImportLocaleFilePayload {
  locale: string;
  namespace: string;
  content: string;
  replace?: boolean;
}

export interface ImportLocaleFileResult {
  success: boolean;
  locale: string;
  namespace: string;
  importedKeys: number;
  warnings: string[];
  effectiveLocaleNamespaces: string[];
}

export interface ReloadLocalesResult {
  success: boolean;
  overlayLocales: LocaleNamespaces[];
  reloadedFiles: number;
  warnings: string[];
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

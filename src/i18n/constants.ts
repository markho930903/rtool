import type { AppLocale, LocalePreference } from "@/i18n/types";

export const SUPPORTED_LOCALES: AppLocale[] = ["zh-CN", "en-US"];

export const LOCALE_STORAGE_KEY = "rtool.locale.preference";

export const DEFAULT_LOCALE_PREFERENCE: LocalePreference = "system";

export const FALLBACK_LOCALE: AppLocale = "zh-CN";

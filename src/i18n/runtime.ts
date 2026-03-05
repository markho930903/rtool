import { DEFAULT_LOCALE_PREFERENCE, FALLBACK_LOCALE, LOCALE_STORAGE_KEY } from "@/i18n/constants";
import type { AppLocale, LocalePreference } from "@/i18n/types";

function normalizeLocale(value: string | null | undefined): AppLocale | null {
  if (!value) {
    return null;
  }

  const normalized = value.trim().replace(/_/g, "-");
  if (!normalized) {
    return null;
  }

  const segments = normalized
    .split("-")
    .map((segment: string) => segment.trim())
    .filter(Boolean);
  if (segments.length === 0) {
    return null;
  }

  const language = segments[0].toLowerCase();
  if (!/^[a-z]{2}$/.test(language)) {
    return null;
  }

  const region = segments
    .slice(1)
    .find((segment: string) => /^[a-z]{2}$/i.test(segment))
    ?.toUpperCase();
  if (region) {
    return `${language}-${region}`;
  }

  if (language === "zh") {
    return "zh-CN";
  }

  if (language === "en") {
    return "en-US";
  }

  return null;
}

function isLocalePreference(value: string | null): value is LocalePreference {
  return value === "system" || normalizeLocale(value) !== null;
}

export function detectSystemLocale(): AppLocale {
  if (typeof navigator === "undefined") {
    return FALLBACK_LOCALE;
  }

  const languageCandidates = [...(Array.isArray(navigator.languages) ? navigator.languages : []), navigator.language];

  for (const candidate of languageCandidates) {
    const normalized = normalizeLocale(candidate);
    if (normalized) {
      return normalized;
    }
  }

  return FALLBACK_LOCALE;
}

export function resolveLocale(preference: LocalePreference): AppLocale {
  if (preference === "system") {
    return detectSystemLocale();
  }

  return normalizeLocale(preference) ?? FALLBACK_LOCALE;
}

export function getStoredLocalePreference(): LocalePreference {
  if (typeof window === "undefined") {
    return DEFAULT_LOCALE_PREFERENCE;
  }

  let stored: string | null = null;
  try {
    stored = window.localStorage.getItem(LOCALE_STORAGE_KEY);
  } catch {
    stored = null;
  }

  return isLocalePreference(stored) ? stored : DEFAULT_LOCALE_PREFERENCE;
}

export function setStoredLocalePreference(preference: LocalePreference) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(LOCALE_STORAGE_KEY, preference);
  } catch {}
}

export function applyLocaleToDocument(locale: AppLocale) {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.lang = locale;
  document.documentElement.setAttribute("data-locale", locale);
}

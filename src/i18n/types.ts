export type AppLocale = string;

export type LocalePreference = "system" | AppLocale;

export interface LocaleState {
  preference: LocalePreference;
  resolved: AppLocale;
  initialized: boolean;
}

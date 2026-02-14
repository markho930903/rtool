export type ThemePreference = "light" | "dark" | "system";

export type ResolvedTheme = "light" | "dark";

export interface ThemeState {
  preference: ThemePreference;
  resolved: ResolvedTheme;
  initialized: boolean;
}

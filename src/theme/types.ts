export type ThemePreference = "light" | "dark" | "system";

export type ResolvedTheme = "light" | "dark";

export interface LiquidGlassProfile {
  opacity: number;
  blur: number;
  saturate: number;
  brightness: number;
}

export interface LiquidGlassSettings {
  light: LiquidGlassProfile;
  dark: LiquidGlassProfile;
}

export interface ThemeState {
  preference: ThemePreference;
  resolved: ResolvedTheme;
  glassSettings: LiquidGlassSettings;
  initialized: boolean;
}

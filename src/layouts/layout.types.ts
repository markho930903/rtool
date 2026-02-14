export type LayoutPreference = "topbar" | "sidebar";

export interface LayoutState {
  preference: LayoutPreference;
  initialized: boolean;
}

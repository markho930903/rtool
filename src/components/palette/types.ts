export type PaletteCategory =
  | "builtin"
  | "application"
  | "directory"
  | "file"
  | "action"
  | "clipboard"
  | "tool"
  | "system";

export type LauncherAction =
  | { kind: "open_builtin_route"; route: string }
  | { kind: "open_builtin_tool"; toolId: string }
  | { kind: "open_builtin_window"; windowLabel: string }
  | { kind: "open_directory"; path: string }
  | { kind: "open_file"; path: string }
  | { kind: "open_application"; path: string };

export interface PaletteItem {
  id: string;
  title: string;
  subtitle: string;
  category: PaletteCategory | string;
  source?: string;
  shortcut?: string;
  score?: number;
  iconKind?: "raster" | "iconify";
  iconValue?: string;
  action?: LauncherAction;
}

export interface PaletteActionResult {
  ok: boolean;
  message: string;
}

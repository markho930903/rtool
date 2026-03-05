import type {
  SettingsRequestDto,
  SettingsDto,
  SettingsUpdateInputDto,
} from "@/contracts";
import type { LayoutPreference } from "@/layouts/layout.types";
import { invokeFeature } from "@/services/invoke";
import type { ThemePreference } from "@/theme/types";

function invokeSettings<T>(
  request: SettingsRequestDto,
  silent = true,
): Promise<T> {
  return invokeFeature<T>("settings", request, { silent });
}

export interface SettingsPatchInput {
  theme?: {
    preference?: ThemePreference;
    transparentWindowBackground?: boolean;
  };
  layout?: {
    preference?: LayoutPreference;
  };
  locale?: {
    preference?: string;
  };
  clipboard?: {
    maxItems?: number;
    sizeCleanupEnabled?: boolean;
    maxTotalSizeMb?: number;
  };
  screenshot?: {
    shortcut?: string;
    autoSaveEnabled?: boolean;
    maxItems?: number;
    maxTotalSizeMb?: number;
    pinMaxInstances?: number;
  };
}

export async function getSettings(): Promise<SettingsDto> {
  return invokeSettings<SettingsDto>({ kind: "get" });
}

export async function patchSettings(input: SettingsPatchInput): Promise<SettingsDto> {
  return invokeSettings<SettingsDto>({
    kind: "update",
    payload: { input: input as SettingsUpdateInputDto },
  });
}

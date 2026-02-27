import type { CommandRequestDto, UserGlassProfileDto, UserSettingsDto } from "@/contracts";
import type { LayoutPreference } from "@/layouts/layout.types";
import { invokeWithLog } from "@/services/invoke";
import type { ThemePreference } from "@/theme/types";

export interface UserSettingsPatchInput {
  theme?: {
    preference?: ThemePreference;
    glass?: {
      light?: Partial<UserGlassProfileDto>;
      dark?: Partial<UserGlassProfileDto>;
    };
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
}

function invokeSettings<T>(
  kind: string,
  payload?: Record<string, unknown>,
  silent = true,
): Promise<T> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload;
  }
  return invokeWithLog<T>(
    "settings_handle",
    { request },
    {
      silent,
    },
  );
}

export async function getUserSettings(): Promise<UserSettingsDto> {
  return invokeSettings<UserSettingsDto>("get");
}

export async function patchUserSettings(input: UserSettingsPatchInput): Promise<UserSettingsDto> {
  return invokeSettings<UserSettingsDto>("update", { input });
}

import type { UserGlassProfileDto, UserSettingsDto } from "@/contracts";
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

export async function getUserSettings(): Promise<UserSettingsDto> {
  return invokeWithLog<UserSettingsDto>("app_get_user_settings", undefined, {
    silent: true,
  });
}

export async function patchUserSettings(input: UserSettingsPatchInput): Promise<UserSettingsDto> {
  return invokeWithLog<UserSettingsDto>(
    "app_update_user_settings",
    {
      input,
    },
    {
      silent: true,
    },
  );
}

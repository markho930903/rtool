import type { SettingsDto } from "@/contracts";
import { getSettings } from "@/services/settings.service";

let startupSettingsRequest: Promise<SettingsDto> | null = null;
let startupSettingsSnapshot: SettingsDto | null = null;
let startupSettingsSnapshotAt = 0;
let startupSettingsPending = false;

export function getStartupSettings(): Promise<SettingsDto> {
  if (!startupSettingsRequest) {
    startupSettingsPending = true;
    startupSettingsRequest = getSettings()
      .then((settings) => {
        startupSettingsPending = false;
        startupSettingsSnapshot = settings;
        startupSettingsSnapshotAt = Date.now();
        return settings;
      })
      .catch((error) => {
        startupSettingsPending = false;
        startupSettingsRequest = null;
        throw error;
      });
  }

  return startupSettingsRequest;
}

export function getFreshStartupSettings(maxAgeMs: number): SettingsDto | null {
  if (!startupSettingsSnapshot) {
    return null;
  }
  if (maxAgeMs < 0) {
    return startupSettingsSnapshot;
  }
  const ageMs = Date.now() - startupSettingsSnapshotAt;
  if (ageMs > maxAgeMs) {
    return null;
  }
  return startupSettingsSnapshot;
}

export function getPendingStartupSettingsRequest(): Promise<SettingsDto> | null {
  if (!startupSettingsPending) {
    return null;
  }
  return startupSettingsRequest;
}

import type {
  ScreenshotRequestDto,
  ScreenshotCommitInputDto,
  ScreenshotCommitResultDto,
  ScreenshotPinResultDto,
  ScreenshotSessionDto,
  SettingsScreenshotDto,
  SettingsScreenshotUpdateInputDto,
} from "@/contracts";
import { invokeFeature } from "@/services/invoke";

function invokeScreenshot<T>(request: ScreenshotRequestDto): Promise<T> {
  return invokeFeature<T>("screenshot", request);
}

export async function screenshotStartSession(displayId?: string): Promise<ScreenshotSessionDto> {
  return invokeScreenshot<ScreenshotSessionDto>({
    kind: "start_session",
    payload: {
      input: {
        displayId: displayId ?? null,
      },
    },
  });
}

export async function screenshotCommitSelection(
  input: ScreenshotCommitInputDto,
): Promise<ScreenshotCommitResultDto> {
  return invokeScreenshot<ScreenshotCommitResultDto>({
    kind: "commit_selection",
    payload: { input },
  });
}

export async function screenshotPinSelection(input: ScreenshotCommitInputDto): Promise<ScreenshotPinResultDto> {
  return invokeScreenshot<ScreenshotPinResultDto>({
    kind: "pin_selection",
    payload: { input },
  });
}

export async function screenshotCancelSession(sessionId: string): Promise<boolean> {
  return invokeScreenshot<boolean>({
    kind: "cancel_session",
    payload: {
      input: { sessionId },
    },
  });
}

export async function screenshotGetSettings(): Promise<SettingsScreenshotDto> {
  return invokeScreenshot<SettingsScreenshotDto>({ kind: "get_settings" });
}

export async function screenshotUpdateSettings(
  input: Partial<SettingsScreenshotUpdateInputDto>,
): Promise<SettingsScreenshotDto> {
  return invokeScreenshot<SettingsScreenshotDto>({
    kind: "update_settings",
    payload: { input: input as SettingsScreenshotUpdateInputDto },
  });
}

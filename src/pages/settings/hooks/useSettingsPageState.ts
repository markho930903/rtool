import { message as globalMessage } from "@ui/message/api";
import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";

import type { SelectOptionInput } from "@/components/ui";
import { SUPPORTED_LOCALES } from "@/i18n/constants";
import { useLocaleStore } from "@/i18n/store";
import type { LocalePreference } from "@/i18n/types";
import { useLayoutStore } from "@/layouts/layout.store";
import type { LayoutPreference } from "@/layouts/layout.types";
import { screenshotGetSettings, screenshotUpdateSettings } from "@/services/screenshot.service";
import { useLoggingStore } from "@/stores/logging.store";
import { useSettingsStore } from "@/stores/settings.store";
import { useThemeStore } from "@/theme/store";
import type { ThemePreference } from "@/theme/types";

import { parsePositiveInt, type MessageState } from "./settingsShared";
import {
  useLauncherSettingsSectionState,
  type LauncherSettingsSectionState,
} from "./useLauncherSettingsSectionState";

const MIN_MAX_ITEMS = 100;
const MAX_MAX_ITEMS = 10_000;
const MIN_MAX_TOTAL_SIZE_MB = 100;
const MAX_MAX_TOTAL_SIZE_MB = 10_240;
const CLIPBOARD_SIZE_MB_PRESETS = ["500", "1024", "5120"];
const DEFAULT_CLIPBOARD_SIZE_PRESET_MB = "500";
const MIN_KEEP_DAYS = 1;
const MAX_KEEP_DAYS = 90;
const MIN_HIGH_FREQ_WINDOW_MS = 100;
const MAX_HIGH_FREQ_WINDOW_MS = 60_000;
const MIN_HIGH_FREQ_MAX_PER_KEY = 1;
const MAX_HIGH_FREQ_MAX_PER_KEY = 200;
const LOG_KEEP_DAYS_PRESETS = ["1", "3", "7", "14", "30", "60", "90"];
const LOG_WINDOW_MS_PRESETS = ["100", "250", "500", "1000", "2000", "5000", "10000", "30000", "60000"];
const LOG_MAX_PER_KEY_PRESETS = ["1", "5", "10", "20", "50", "100", "200"];
const MIN_SCREENSHOT_MAX_ITEMS = 50;
const MAX_SCREENSHOT_MAX_ITEMS = 10_000;
const MIN_SCREENSHOT_MAX_TOTAL_SIZE_MB = 100;
const MAX_SCREENSHOT_MAX_TOTAL_SIZE_MB = 20_480;
const MIN_SCREENSHOT_PIN_MAX_INSTANCES = 1;
const MAX_SCREENSHOT_PIN_MAX_INSTANCES = 6;
const CLIPBOARD_SAVE_TOAST_DEDUPE_KEY = "settings-clipboard-save";

type SizeThresholdMode = "preset" | "custom";

export type SettingsSection = "general" | "clipboard" | "screenshot" | "launcher" | "logging";

export interface SettingsNavItem {
  key: SettingsSection;
  label: string;
  description: string;
  icon: string;
}

export interface SettingsNavState {
  activeSection: SettingsSection;
  setActiveSection: (section: SettingsSection) => void;
  settingsNavItems: SettingsNavItem[];
}

export interface GeneralSettingsSectionState {
  localePreference: LocalePreference;
  resolvedLocaleLabel: string;
  localePreferenceOptions: SelectOptionInput[];
  onLocalePreferenceChange: (value: string) => void;

  layoutPreference: LayoutPreference;
  layoutPreferenceOptions: SelectOptionInput[];
  onLayoutPreferenceChange: (value: string) => void;
  themePreference: ThemePreference;
  themePreferenceOptions: SelectOptionInput[];
  onThemePreferenceChange: (value: string) => void;
  transparentWindowBackground: boolean;
  onTransparentWindowBackgroundChange: (checked: boolean) => void;
}

export interface ClipboardSettingsSectionState {
  maxItemsInput: string;
  maxItemsInvalid: boolean;
  clipboardMaxItemsHelperText: string;

  sizeCleanupEnabled: boolean;
  sizeThresholdMode: SizeThresholdMode;
  selectedPresetMb: string;
  presets: string[];
  customSizeMbInput: string;
  customSizeInputRef: RefObject<HTMLInputElement | null>;
  maxTotalSizeInvalid: boolean;
  clipboardSizeHelperText: string;

  limits: {
    maxItemsMin: number;
    maxItemsMax: number;
    maxTotalSizeMin: number;
    maxTotalSizeMax: number;
  };

  loading: boolean;
  saving: boolean;
  invalid: boolean;
  unchanged: boolean;
  error: string | null;

  onMaxItemsChange: (value: string) => void;
  onSizeCleanupEnabledChange: (checked: boolean) => void;
  onPresetSelect: (presetValue: string) => void;
  onCustomModeSelect: () => void;
  onCustomSizeChange: (value: string) => void;
  onSave: () => Promise<void>;
}

export interface ScreenshotSettingsSectionState {
  loading: boolean;
  saving: boolean;
  shortcutInput: string;
  autoSaveEnabled: boolean;
  maxItemsInput: string;
  maxTotalSizeInput: string;
  pinMaxInstancesInput: string;
  shortcutInvalid: boolean;
  maxItemsInvalid: boolean;
  maxTotalSizeInvalid: boolean;
  pinMaxInstancesInvalid: boolean;
  unchanged: boolean;
  saveMessage: MessageState | null;
  limits: {
    maxItemsMin: number;
    maxItemsMax: number;
    maxTotalSizeMin: number;
    maxTotalSizeMax: number;
    pinMaxInstancesMin: number;
    pinMaxInstancesMax: number;
  };
  onShortcutChange: (value: string) => void;
  onAutoSaveEnabledChange: (checked: boolean) => void;
  onMaxItemsChange: (value: string) => void;
  onMaxTotalSizeChange: (value: string) => void;
  onPinMaxInstancesChange: (value: string) => void;
  onSave: () => Promise<void>;
}

export type { LauncherSettingsSectionState } from "./useLauncherSettingsSectionState";

export interface LoggingSettingsSectionState {
  minLevel: string;
  keepDaysInput: string;
  realtimeEnabled: boolean;
  highFreqWindowMsInput: string;
  highFreqMaxPerKeyInput: string;
  allowRawView: boolean;

  keepDaysOptions: SelectOptionInput[];
  windowMsOptions: SelectOptionInput[];
  maxPerKeyOptions: SelectOptionInput[];

  keepDaysInvalid: boolean;
  highFreqWindowInvalid: boolean;
  highFreqMaxPerKeyInvalid: boolean;

  limits: {
    keepDaysMin: number;
    keepDaysMax: number;
    windowMsMin: number;
    windowMsMax: number;
    maxPerKeyMin: number;
    maxPerKeyMax: number;
  };

  invalid: boolean;
  unchanged: boolean;
  error: string | null;
  saveMessage: MessageState | null;

  onMinLevelChange: (value: string) => void;
  onKeepDaysChange: (value: string) => void;
  onRealtimeEnabledChange: (checked: boolean) => void;
  onHighFreqWindowMsChange: (value: string) => void;
  onHighFreqMaxPerKeyChange: (value: string) => void;
  onAllowRawViewChange: (checked: boolean) => void;
  onSave: () => Promise<void>;
}

export interface UseSettingsPageStateResult {
  nav: SettingsNavState;
  general: GeneralSettingsSectionState;
  clipboard: ClipboardSettingsSectionState;
  screenshot: ScreenshotSettingsSectionState;
  launcher: LauncherSettingsSectionState;
  logging: LoggingSettingsSectionState;
}

function localeDisplayLabel(locale: string, t: (key: string) => string): string {
  if (locale === "zh-CN") {
    return t("general.option.zh");
  }
  if (locale === "en-US") {
    return t("general.option.en");
  }
  return locale;
}

function buildNumericSelectOptions(presets: string[], currentValue: string): SelectOptionInput[] {
  const values = new Set(presets);
  const normalizedCurrentValue = currentValue.trim();
  if (normalizedCurrentValue && !values.has(normalizedCurrentValue)) {
    values.add(normalizedCurrentValue);
  }

  return [...values]
    .sort((left, right) => {
      const leftNumber = Number(left);
      const rightNumber = Number(right);
      const leftIsNumber = Number.isFinite(leftNumber);
      const rightIsNumber = Number.isFinite(rightNumber);
      if (leftIsNumber && rightIsNumber) {
        return leftNumber - rightNumber;
      }
      if (leftIsNumber) {
        return -1;
      }
      if (rightIsNumber) {
        return 1;
      }
      return left.localeCompare(right);
    })
    .map((value) => ({ value, label: value }));
}

export function useSettingsPageState(): UseSettingsPageStateResult {
  const { t } = useTranslation("settings");

  const localePreference = useLocaleStore((state) => state.preference);
  const resolvedLocale = useLocaleStore((state) => state.resolved);
  const setLocalePreference = useLocaleStore((state) => state.setPreference);
  const layoutPreference = useLayoutStore((state) => state.preference);
  const setLayoutPreference = useLayoutStore((state) => state.setPreference);
  const themePreference = useThemeStore((state) => state.preference);
  const setThemePreference = useThemeStore((state) => state.setPreference);
  const transparentWindowBackground = useThemeStore((state) => state.transparentWindowBackground);
  const setTransparentWindowBackground = useThemeStore((state) => state.setTransparentWindowBackground);

  const clipboardSettings = useSettingsStore((state) => state.clipboardSettings);
  const clipboardLoading = useSettingsStore((state) => state.loading);
  const clipboardSaving = useSettingsStore((state) => state.saving);
  const clipboardError = useSettingsStore((state) => state.error);
  const fetchClipboardSettings = useSettingsStore((state) => state.fetchClipboardSettings);
  const updateClipboardSettings = useSettingsStore((state) => state.updateClipboardSettings);

  const loggingConfig = useLoggingStore((state) => state.config);
  const loggingError = useLoggingStore((state) => state.error);
  const fetchLoggingConfig = useLoggingStore((state) => state.fetchConfig);
  const updateLoggingConfig = useLoggingStore((state) => state.saveConfig);

  const [maxItemsInput, setMaxItemsInput] = useState(String(clipboardSettings?.maxItems ?? 1000));
  const [sizeCleanupEnabled, setSizeCleanupEnabled] = useState(clipboardSettings?.sizeCleanupEnabled ?? true);
  const [selectedPresetMb, setSelectedPresetMb] = useState(() => {
    const initialValue = String(clipboardSettings?.maxTotalSizeMb ?? DEFAULT_CLIPBOARD_SIZE_PRESET_MB);
    return CLIPBOARD_SIZE_MB_PRESETS.includes(initialValue) ? initialValue : DEFAULT_CLIPBOARD_SIZE_PRESET_MB;
  });
  const [sizeThresholdMode, setSizeThresholdMode] = useState<SizeThresholdMode>(() => {
    const initialValue = String(clipboardSettings?.maxTotalSizeMb ?? DEFAULT_CLIPBOARD_SIZE_PRESET_MB);
    return CLIPBOARD_SIZE_MB_PRESETS.includes(initialValue) ? "preset" : "custom";
  });
  const [customSizeMbInput, setCustomSizeMbInput] = useState(
    String(clipboardSettings?.maxTotalSizeMb ?? DEFAULT_CLIPBOARD_SIZE_PRESET_MB),
  );
  const customSizeInputRef = useRef<HTMLInputElement>(null);
  const [activeSection, setActiveSection] = useState<SettingsSection>("general");
  const [screenshotLoading, setScreenshotLoading] = useState(false);
  const [screenshotSaving, setScreenshotSaving] = useState(false);
  const [screenshotShortcutInput, setScreenshotShortcutInput] = useState("");
  const [screenshotAutoSaveEnabled, setScreenshotAutoSaveEnabled] = useState(true);
  const [screenshotMaxItemsInput, setScreenshotMaxItemsInput] = useState(String(MIN_SCREENSHOT_MAX_ITEMS));
  const [screenshotMaxTotalSizeInput, setScreenshotMaxTotalSizeInput] = useState(
    String(MIN_SCREENSHOT_MAX_TOTAL_SIZE_MB),
  );
  const [screenshotPinMaxInstancesInput, setScreenshotPinMaxInstancesInput] = useState(
    String(MAX_SCREENSHOT_PIN_MAX_INSTANCES),
  );
  const [screenshotSaveMessage, setScreenshotSaveMessage] = useState<MessageState | null>(null);
  const [screenshotBaseline, setScreenshotBaseline] = useState<{
    shortcut: string;
    autoSaveEnabled: boolean;
    maxItems: number;
    maxTotalSizeMb: number;
    pinMaxInstances: number;
  } | null>(null);
  const launcher = useLauncherSettingsSectionState({ active: activeSection === "launcher", t });

  const [logMinLevel, setLogMinLevel] = useState("info");
  const [logKeepDaysInput, setLogKeepDaysInput] = useState(String(MIN_KEEP_DAYS));
  const [logRealtimeEnabled, setLogRealtimeEnabled] = useState(true);
  const [logHighFreqWindowMsInput, setLogHighFreqWindowMsInput] = useState(String(1000));
  const [logHighFreqMaxPerKeyInput, setLogHighFreqMaxPerKeyInput] = useState(String(20));
  const [logAllowRawView, setLogAllowRawView] = useState(false);
  const [loggingSaveMessage, setLoggingSaveMessage] = useState<MessageState | null>(null);

  const settingsNavItems: SettingsNavItem[] = useMemo(
    () => [
      {
        key: "general",
        label: t("section.general.label"),
        description: t("section.general.description"),
        icon: "i-noto:gear",
      },
      {
        key: "clipboard",
        label: t("section.clipboard.label"),
        description: t("section.clipboard.description"),
        icon: "i-noto:clipboard",
      },
      {
        key: "screenshot",
        label: t("section.screenshot.label"),
        description: t("section.screenshot.description"),
        icon: "i-noto:camera-with-flash",
      },
      {
        key: "launcher",
        label: t("section.launcher.label"),
        description: t("section.launcher.description"),
        icon: "i-noto:card-index-dividers",
      },
      {
        key: "logging",
        label: t("section.logging.label"),
        description: t("section.logging.description"),
        icon: "i-noto:scroll",
      },
    ],
    [t],
  );

  useEffect(() => {
    void fetchClipboardSettings();
    void fetchLoggingConfig();

    const loadScreenshot = async () => {
      setScreenshotLoading(true);
      try {
        const settings = await screenshotGetSettings();
        setScreenshotShortcutInput(settings.shortcut);
        setScreenshotAutoSaveEnabled(settings.autoSaveEnabled);
        setScreenshotMaxItemsInput(String(settings.maxItems));
        setScreenshotMaxTotalSizeInput(String(settings.maxTotalSizeMb));
        setScreenshotPinMaxInstancesInput(String(settings.pinMaxInstances));
        setScreenshotBaseline({
          shortcut: settings.shortcut,
          autoSaveEnabled: settings.autoSaveEnabled,
          maxItems: settings.maxItems,
          maxTotalSizeMb: settings.maxTotalSizeMb,
          pinMaxInstances: settings.pinMaxInstances,
        });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setScreenshotSaveMessage({ text: message, isError: true });
      } finally {
        setScreenshotLoading(false);
      }
    };

    void loadScreenshot();

  }, [fetchClipboardSettings, fetchLoggingConfig]);

  useEffect(() => {
    if (clipboardSettings) {
      setMaxItemsInput(String(clipboardSettings.maxItems));
      setSizeCleanupEnabled(clipboardSettings.sizeCleanupEnabled);
      const thresholdValue = String(clipboardSettings.maxTotalSizeMb);
      setCustomSizeMbInput(thresholdValue);
      if (CLIPBOARD_SIZE_MB_PRESETS.includes(thresholdValue)) {
        setSizeThresholdMode("preset");
        setSelectedPresetMb(thresholdValue);
      } else {
        setSizeThresholdMode("custom");
        setSelectedPresetMb(DEFAULT_CLIPBOARD_SIZE_PRESET_MB);
      }
    }
  }, [clipboardSettings]);

  useEffect(() => {
    if (!sizeCleanupEnabled || sizeThresholdMode !== "custom") {
      return;
    }

    const frame = requestAnimationFrame(() => {
      customSizeInputRef.current?.focus();
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [sizeCleanupEnabled, sizeThresholdMode]);

  useEffect(() => {
    if (!loggingConfig) {
      return;
    }

    setLogMinLevel(loggingConfig.minLevel);
    setLogKeepDaysInput(String(loggingConfig.keepDays));
    setLogRealtimeEnabled(loggingConfig.realtimeEnabled);
    setLogHighFreqWindowMsInput(String(loggingConfig.highFreqWindowMs));
    setLogHighFreqMaxPerKeyInput(String(loggingConfig.highFreqMaxPerKey));
    setLogAllowRawView(loggingConfig.allowRawView);
  }, [loggingConfig]);

  const parsedMaxItems = useMemo(() => parsePositiveInt(maxItemsInput), [maxItemsInput]);
  const parsedCustomSizeMb = useMemo(() => parsePositiveInt(customSizeMbInput), [customSizeMbInput]);
  const effectiveMaxTotalSizeMb = useMemo(() => {
    if (sizeThresholdMode === "preset") {
      return Number.parseInt(selectedPresetMb, 10);
    }
    return parsedCustomSizeMb;
  }, [parsedCustomSizeMb, selectedPresetMb, sizeThresholdMode]);

  const maxItemsInvalid = parsedMaxItems === null || parsedMaxItems < MIN_MAX_ITEMS || parsedMaxItems > MAX_MAX_ITEMS;
  const maxTotalSizeInvalid =
    sizeThresholdMode === "custom" &&
    (parsedCustomSizeMb === null ||
      parsedCustomSizeMb < MIN_MAX_TOTAL_SIZE_MB ||
      parsedCustomSizeMb > MAX_MAX_TOTAL_SIZE_MB);

  const clipboardInvalid = maxItemsInvalid || maxTotalSizeInvalid;
  const clipboardUnchanged =
    parsedMaxItems !== null &&
    effectiveMaxTotalSizeMb !== null &&
    clipboardSettings !== null &&
    parsedMaxItems === clipboardSettings.maxItems &&
    effectiveMaxTotalSizeMb === clipboardSettings.maxTotalSizeMb &&
    sizeCleanupEnabled === clipboardSettings.sizeCleanupEnabled;

  const parsedKeepDays = useMemo(() => parsePositiveInt(logKeepDaysInput), [logKeepDaysInput]);
  const parsedHighFreqWindowMs = useMemo(() => parsePositiveInt(logHighFreqWindowMsInput), [logHighFreqWindowMsInput]);
  const parsedHighFreqMaxPerKey = useMemo(
    () => parsePositiveInt(logHighFreqMaxPerKeyInput),
    [logHighFreqMaxPerKeyInput],
  );

  const logKeepDaysOptions = useMemo(
    () => buildNumericSelectOptions(LOG_KEEP_DAYS_PRESETS, logKeepDaysInput),
    [logKeepDaysInput],
  );
  const logWindowMsOptions = useMemo(
    () => buildNumericSelectOptions(LOG_WINDOW_MS_PRESETS, logHighFreqWindowMsInput),
    [logHighFreqWindowMsInput],
  );
  const logMaxPerKeyOptions = useMemo(
    () => buildNumericSelectOptions(LOG_MAX_PER_KEY_PRESETS, logHighFreqMaxPerKeyInput),
    [logHighFreqMaxPerKeyInput],
  );

  const logKeepDaysInvalid =
    parsedKeepDays === null || parsedKeepDays < MIN_KEEP_DAYS || parsedKeepDays > MAX_KEEP_DAYS;
  const logHighFreqWindowInvalid =
    parsedHighFreqWindowMs === null ||
    parsedHighFreqWindowMs < MIN_HIGH_FREQ_WINDOW_MS ||
    parsedHighFreqWindowMs > MAX_HIGH_FREQ_WINDOW_MS;
  const logHighFreqMaxPerKeyInvalid =
    parsedHighFreqMaxPerKey === null ||
    parsedHighFreqMaxPerKey < MIN_HIGH_FREQ_MAX_PER_KEY ||
    parsedHighFreqMaxPerKey > MAX_HIGH_FREQ_MAX_PER_KEY;

  const validMinLevel =
    logMinLevel === "trace" ||
    logMinLevel === "debug" ||
    logMinLevel === "info" ||
    logMinLevel === "warn" ||
    logMinLevel === "error";

  const loggingInvalid =
    !validMinLevel || logKeepDaysInvalid || logHighFreqWindowInvalid || logHighFreqMaxPerKeyInvalid;

  const loggingUnchanged =
    loggingConfig !== null &&
    loggingConfig.minLevel === logMinLevel &&
    loggingConfig.keepDays === parsedKeepDays &&
    loggingConfig.realtimeEnabled === logRealtimeEnabled &&
    loggingConfig.highFreqWindowMs === parsedHighFreqWindowMs &&
    loggingConfig.highFreqMaxPerKey === parsedHighFreqMaxPerKey &&
    loggingConfig.allowRawView === logAllowRawView;

  const normalizedScreenshotShortcut = screenshotShortcutInput.trim();
  const screenshotShortcutInvalid = normalizedScreenshotShortcut.length === 0;
  const parsedScreenshotMaxItems = useMemo(() => parsePositiveInt(screenshotMaxItemsInput), [screenshotMaxItemsInput]);
  const parsedScreenshotMaxTotalSize = useMemo(
    () => parsePositiveInt(screenshotMaxTotalSizeInput),
    [screenshotMaxTotalSizeInput],
  );
  const parsedScreenshotPinMaxInstances = useMemo(
    () => parsePositiveInt(screenshotPinMaxInstancesInput),
    [screenshotPinMaxInstancesInput],
  );
  const screenshotMaxItemsInvalid =
    parsedScreenshotMaxItems === null ||
    parsedScreenshotMaxItems < MIN_SCREENSHOT_MAX_ITEMS ||
    parsedScreenshotMaxItems > MAX_SCREENSHOT_MAX_ITEMS;
  const screenshotMaxTotalSizeInvalid =
    parsedScreenshotMaxTotalSize === null ||
    parsedScreenshotMaxTotalSize < MIN_SCREENSHOT_MAX_TOTAL_SIZE_MB ||
    parsedScreenshotMaxTotalSize > MAX_SCREENSHOT_MAX_TOTAL_SIZE_MB;
  const screenshotPinMaxInstancesInvalid =
    parsedScreenshotPinMaxInstances === null ||
    parsedScreenshotPinMaxInstances < MIN_SCREENSHOT_PIN_MAX_INSTANCES ||
    parsedScreenshotPinMaxInstances > MAX_SCREENSHOT_PIN_MAX_INSTANCES;
  const screenshotUnchanged =
    screenshotBaseline !== null &&
    !screenshotShortcutInvalid &&
    parsedScreenshotMaxItems !== null &&
    parsedScreenshotMaxTotalSize !== null &&
    parsedScreenshotPinMaxInstances !== null &&
    screenshotBaseline.shortcut === normalizedScreenshotShortcut &&
    screenshotBaseline.autoSaveEnabled === screenshotAutoSaveEnabled &&
    screenshotBaseline.maxItems === parsedScreenshotMaxItems &&
    screenshotBaseline.maxTotalSizeMb === parsedScreenshotMaxTotalSize &&
    screenshotBaseline.pinMaxInstances === parsedScreenshotPinMaxInstances;

  const localePreferenceOptions = useMemo(() => {
    const sortedValues = [...SUPPORTED_LOCALES].sort((left, right) => left.localeCompare(right));
    return [
      { value: "system", label: t("general.option.system") },
      ...sortedValues.map((value) => ({
        value,
        label: localeDisplayLabel(value, t),
      })),
    ];
  }, [t]);

  const layoutPreferenceOptions = useMemo(
    () => [
      { value: "topbar", label: t("general.layout.option.topbar") },
      { value: "sidebar", label: t("general.layout.option.sidebar") },
    ],
    [t],
  );

  const themePreferenceOptions = useMemo(
    () => [
      { value: "light", label: t("general.theme.option.light") },
      { value: "dark", label: t("general.theme.option.dark") },
      { value: "system", label: t("general.theme.option.system") },
    ],
    [t],
  );

  const clipboardMaxItemsHelperText = maxItemsInvalid
    ? t("clipboard.invalid", { min: MIN_MAX_ITEMS, max: MAX_MAX_ITEMS })
    : t("clipboard.helper");

  const clipboardSizeHelperText = !sizeCleanupEnabled
    ? t("clipboard.sizeHelperDisabled")
    : sizeThresholdMode === "custom"
      ? maxTotalSizeInvalid
        ? t("clipboard.sizeInvalid", { min: MIN_MAX_TOTAL_SIZE_MB, max: MAX_MAX_TOTAL_SIZE_MB })
        : t("clipboard.sizeCustomInputHint")
      : t("clipboard.sizePresetHint", { value: selectedPresetMb });

  const handleThemePreferenceChange = (value: string) => {
    const nextPreference: ThemePreference =
      value === "dark" || value === "light" || value === "system" ? value : "system";
    if (themePreference !== nextPreference) {
      void setThemePreference(nextPreference);
    }
  };

  const handleTransparentWindowBackgroundChange = (checked: boolean) => {
    if (checked === transparentWindowBackground) {
      return;
    }
    void setTransparentWindowBackground(checked);
  };

  const handleSaveScreenshot = async () => {
    if (
      screenshotShortcutInvalid ||
      screenshotMaxItemsInvalid ||
      screenshotMaxTotalSizeInvalid ||
      screenshotPinMaxInstancesInvalid
    ) {
      setScreenshotSaveMessage({
        text: t("screenshot.saveFailedInvalid"),
        isError: true,
      });
      return;
    }

    setScreenshotSaving(true);
    try {
      const settings = await screenshotUpdateSettings({
        shortcut: normalizedScreenshotShortcut,
        autoSaveEnabled: screenshotAutoSaveEnabled,
        maxItems: parsedScreenshotMaxItems,
        maxTotalSizeMb: parsedScreenshotMaxTotalSize,
        pinMaxInstances: parsedScreenshotPinMaxInstances,
      });
      setScreenshotShortcutInput(settings.shortcut);
      setScreenshotAutoSaveEnabled(settings.autoSaveEnabled);
      setScreenshotMaxItemsInput(String(settings.maxItems));
      setScreenshotMaxTotalSizeInput(String(settings.maxTotalSizeMb));
      setScreenshotPinMaxInstancesInput(String(settings.pinMaxInstances));
      setScreenshotBaseline({
        shortcut: settings.shortcut,
        autoSaveEnabled: settings.autoSaveEnabled,
        maxItems: settings.maxItems,
        maxTotalSizeMb: settings.maxTotalSizeMb,
        pinMaxInstances: settings.pinMaxInstances,
      });
      setScreenshotSaveMessage({ text: t("screenshot.saved"), isError: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setScreenshotSaveMessage({ text: t("screenshot.saveFailed", { message }), isError: true });
    } finally {
      setScreenshotSaving(false);
    }
  };

  const handleSaveClipboard = async () => {
    if (parsedMaxItems === null || maxItemsInvalid) {
      globalMessage.error({
        description: t("clipboard.saveFailedInvalid", { min: MIN_MAX_ITEMS, max: MAX_MAX_ITEMS }),
        dedupeKey: CLIPBOARD_SAVE_TOAST_DEDUPE_KEY,
        duration: 5000,
      });
      return;
    }

    if (effectiveMaxTotalSizeMb === null || maxTotalSizeInvalid) {
      globalMessage.error({
        description: t("clipboard.saveFailedInvalidSize", {
          min: MIN_MAX_TOTAL_SIZE_MB,
          max: MAX_MAX_TOTAL_SIZE_MB,
        }),
        dedupeKey: CLIPBOARD_SAVE_TOAST_DEDUPE_KEY,
        duration: 5000,
      });
      return;
    }

    try {
      await updateClipboardSettings({
        maxItems: parsedMaxItems,
        sizeCleanupEnabled,
        maxTotalSizeMb: effectiveMaxTotalSizeMb,
      });
      globalMessage.success({
        description: t("clipboard.saved"),
        dedupeKey: CLIPBOARD_SAVE_TOAST_DEDUPE_KEY,
      });
    } catch (saveError) {
      const errorMessage = saveError instanceof Error ? saveError.message : String(saveError);
      const isDiskLowError = errorMessage.includes("clipboard_disk_space_low");
      globalMessage.error({
        description: isDiskLowError
          ? t("clipboard.saveFailedDiskLow", { minMb: 512 })
          : t("clipboard.saveFailed", { message: errorMessage }),
        dedupeKey: CLIPBOARD_SAVE_TOAST_DEDUPE_KEY,
        duration: 5000,
      });
    }
  };

  const handleSaveLogging = async () => {
    if (
      !validMinLevel ||
      parsedKeepDays === null ||
      parsedHighFreqWindowMs === null ||
      parsedHighFreqMaxPerKey === null
    ) {
      setLoggingSaveMessage({ text: t("logging.saveFailedInput"), isError: true });
      return;
    }

    if (loggingInvalid) {
      setLoggingSaveMessage({ text: t("logging.saveFailedRange"), isError: true });
      return;
    }

    try {
      await updateLoggingConfig({
        minLevel: logMinLevel,
        keepDays: parsedKeepDays,
        realtimeEnabled: logRealtimeEnabled,
        highFreqWindowMs: parsedHighFreqWindowMs,
        highFreqMaxPerKey: parsedHighFreqMaxPerKey,
        allowRawView: logAllowRawView,
      });
      setLoggingSaveMessage({ text: t("logging.saved"), isError: false });
    } catch (saveError) {
      const message = saveError instanceof Error ? saveError.message : String(saveError);
      setLoggingSaveMessage({ text: t("logging.saveFailed", { message }), isError: true });
    }
  };

  const onLocalePreferenceChange = useCallback(
    (value: string) => {
      void setLocalePreference(value as LocalePreference);
    },
    [setLocalePreference],
  );

  const onLayoutPreferenceChange = useCallback(
    (value: string) => {
      void setLayoutPreference(value as LayoutPreference);
    },
    [setLayoutPreference],
  );

  const onMaxItemsChange = useCallback((value: string) => {
    setMaxItemsInput(value);
  }, []);

  const onSizeCleanupEnabledChange = useCallback((checked: boolean) => {
    setSizeCleanupEnabled(checked);
  }, []);

  const onPresetSelect = useCallback((presetValue: string) => {
    setSizeThresholdMode("preset");
    setSelectedPresetMb(presetValue);
  }, []);

  const onCustomModeSelect = useCallback(() => {
    setSizeThresholdMode("custom");
  }, []);

  const onCustomSizeChange = useCallback((value: string) => {
    setCustomSizeMbInput(value);
  }, []);

  const onScreenshotShortcutChange = useCallback((value: string) => {
    setScreenshotShortcutInput(value);
    setScreenshotSaveMessage(null);
  }, []);

  const onScreenshotAutoSaveEnabledChange = useCallback((checked: boolean) => {
    setScreenshotAutoSaveEnabled(checked);
    setScreenshotSaveMessage(null);
  }, []);

  const onScreenshotMaxItemsChange = useCallback((value: string) => {
    setScreenshotMaxItemsInput(value);
    setScreenshotSaveMessage(null);
  }, []);

  const onScreenshotMaxTotalSizeChange = useCallback((value: string) => {
    setScreenshotMaxTotalSizeInput(value);
    setScreenshotSaveMessage(null);
  }, []);

  const onScreenshotPinMaxInstancesChange = useCallback((value: string) => {
    setScreenshotPinMaxInstancesInput(value);
    setScreenshotSaveMessage(null);
  }, []);

  const onMinLevelChange = useCallback((value: string) => {
    setLogMinLevel(value);
    setLoggingSaveMessage(null);
  }, []);

  const onKeepDaysChange = useCallback((value: string) => {
    setLogKeepDaysInput(value);
    setLoggingSaveMessage(null);
  }, []);

  const onRealtimeEnabledChange = useCallback((checked: boolean) => {
    setLogRealtimeEnabled(checked);
    setLoggingSaveMessage(null);
  }, []);

  const onHighFreqWindowMsChange = useCallback((value: string) => {
    setLogHighFreqWindowMsInput(value);
    setLoggingSaveMessage(null);
  }, []);

  const onHighFreqMaxPerKeyChange = useCallback((value: string) => {
    setLogHighFreqMaxPerKeyInput(value);
    setLoggingSaveMessage(null);
  }, []);

  const onAllowRawViewChange = useCallback((checked: boolean) => {
    setLogAllowRawView(checked);
    setLoggingSaveMessage(null);
  }, []);

  return {
    nav: {
      activeSection,
      setActiveSection,
      settingsNavItems,
    },
    general: {
      localePreference,
      resolvedLocaleLabel: localeDisplayLabel(resolvedLocale, t),
      localePreferenceOptions,
      onLocalePreferenceChange,
      layoutPreference,
      layoutPreferenceOptions,
      onLayoutPreferenceChange,
      themePreference,
      themePreferenceOptions,
      onThemePreferenceChange: handleThemePreferenceChange,
      transparentWindowBackground,
      onTransparentWindowBackgroundChange: handleTransparentWindowBackgroundChange,
    },
    clipboard: {
      maxItemsInput,
      maxItemsInvalid,
      clipboardMaxItemsHelperText,
      sizeCleanupEnabled,
      sizeThresholdMode,
      selectedPresetMb,
      presets: CLIPBOARD_SIZE_MB_PRESETS,
      customSizeMbInput,
      customSizeInputRef,
      maxTotalSizeInvalid,
      clipboardSizeHelperText,
      limits: {
        maxItemsMin: MIN_MAX_ITEMS,
        maxItemsMax: MAX_MAX_ITEMS,
        maxTotalSizeMin: MIN_MAX_TOTAL_SIZE_MB,
        maxTotalSizeMax: MAX_MAX_TOTAL_SIZE_MB,
      },
      loading: clipboardLoading,
      saving: clipboardSaving,
      invalid: clipboardInvalid,
      unchanged: clipboardUnchanged,
      error: clipboardError,
      onMaxItemsChange,
      onSizeCleanupEnabledChange,
      onPresetSelect,
      onCustomModeSelect,
      onCustomSizeChange,
      onSave: handleSaveClipboard,
    },
    screenshot: {
      loading: screenshotLoading,
      saving: screenshotSaving,
      shortcutInput: screenshotShortcutInput,
      autoSaveEnabled: screenshotAutoSaveEnabled,
      maxItemsInput: screenshotMaxItemsInput,
      maxTotalSizeInput: screenshotMaxTotalSizeInput,
      pinMaxInstancesInput: screenshotPinMaxInstancesInput,
      shortcutInvalid: screenshotShortcutInvalid,
      maxItemsInvalid: screenshotMaxItemsInvalid,
      maxTotalSizeInvalid: screenshotMaxTotalSizeInvalid,
      pinMaxInstancesInvalid: screenshotPinMaxInstancesInvalid,
      unchanged: screenshotUnchanged,
      saveMessage: screenshotSaveMessage,
      limits: {
        maxItemsMin: MIN_SCREENSHOT_MAX_ITEMS,
        maxItemsMax: MAX_SCREENSHOT_MAX_ITEMS,
        maxTotalSizeMin: MIN_SCREENSHOT_MAX_TOTAL_SIZE_MB,
        maxTotalSizeMax: MAX_SCREENSHOT_MAX_TOTAL_SIZE_MB,
        pinMaxInstancesMin: MIN_SCREENSHOT_PIN_MAX_INSTANCES,
        pinMaxInstancesMax: MAX_SCREENSHOT_PIN_MAX_INSTANCES,
      },
      onShortcutChange: onScreenshotShortcutChange,
      onAutoSaveEnabledChange: onScreenshotAutoSaveEnabledChange,
      onMaxItemsChange: onScreenshotMaxItemsChange,
      onMaxTotalSizeChange: onScreenshotMaxTotalSizeChange,
      onPinMaxInstancesChange: onScreenshotPinMaxInstancesChange,
      onSave: handleSaveScreenshot,
    },
    launcher,
    logging: {
      minLevel: logMinLevel,
      keepDaysInput: logKeepDaysInput,
      realtimeEnabled: logRealtimeEnabled,
      highFreqWindowMsInput: logHighFreqWindowMsInput,
      highFreqMaxPerKeyInput: logHighFreqMaxPerKeyInput,
      allowRawView: logAllowRawView,
      keepDaysOptions: logKeepDaysOptions,
      windowMsOptions: logWindowMsOptions,
      maxPerKeyOptions: logMaxPerKeyOptions,
      keepDaysInvalid: logKeepDaysInvalid,
      highFreqWindowInvalid: logHighFreqWindowInvalid,
      highFreqMaxPerKeyInvalid: logHighFreqMaxPerKeyInvalid,
      limits: {
        keepDaysMin: MIN_KEEP_DAYS,
        keepDaysMax: MAX_KEEP_DAYS,
        windowMsMin: MIN_HIGH_FREQ_WINDOW_MS,
        windowMsMax: MAX_HIGH_FREQ_WINDOW_MS,
        maxPerKeyMin: MIN_HIGH_FREQ_MAX_PER_KEY,
        maxPerKeyMax: MAX_HIGH_FREQ_MAX_PER_KEY,
      },
      invalid: loggingInvalid,
      unchanged: loggingUnchanged,
      error: loggingError,
      saveMessage: loggingSaveMessage,
      onMinLevelChange,
      onKeepDaysChange,
      onRealtimeEnabledChange,
      onHighFreqWindowMsChange,
      onHighFreqMaxPerKeyChange,
      onAllowRawViewChange,
      onSave: handleSaveLogging,
    },
  };
}

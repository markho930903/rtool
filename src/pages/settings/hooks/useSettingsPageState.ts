import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";

import type { SelectOptionInput } from "@/components/ui";
import { SUPPORTED_LOCALES } from "@/i18n/constants";
import { useLocaleStore } from "@/i18n/store";
import type { LocalePreference } from "@/i18n/types";
import { useLayoutStore } from "@/layouts/layout.store";
import type { LayoutPreference } from "@/layouts/layout.types";
import {
  launcherGetIndexStatus,
  launcherResetSearchSettings,
  launcherGetSearchSettings,
  launcherRebuildIndex,
  launcherUpdateSearchSettings,
  type LauncherIndexStatus,
  type LauncherSearchSettings,
} from "@/services/launcher.service";
import { transferGetSettings, transferUpdateSettings } from "@/services/transfer.service";
import { useLoggingStore } from "@/stores/logging.store";
import { useSettingsStore } from "@/stores/settings.store";
import { useThemeStore } from "@/theme/store";
import type { ResolvedTheme } from "@/theme/types";

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
const MIN_LAUNCHER_DEPTH = 2;
const MAX_LAUNCHER_DEPTH = 32;
const DEFAULT_LAUNCHER_DEPTH = 20;
const MIN_LAUNCHER_ITEMS_PER_ROOT = 500;
const MAX_LAUNCHER_ITEMS_PER_ROOT = 1_000_000;
const DEFAULT_LAUNCHER_ITEMS_PER_ROOT = 200_000;
const MIN_LAUNCHER_TOTAL_ITEMS = 2_000;
const MAX_LAUNCHER_TOTAL_ITEMS = 2_000_000;
const DEFAULT_LAUNCHER_TOTAL_ITEMS = 500_000;
const MIN_LAUNCHER_REFRESH_INTERVAL = 60;
const MAX_LAUNCHER_REFRESH_INTERVAL = 86_400;
const DEFAULT_LAUNCHER_REFRESH_INTERVAL = 600;

type SizeThresholdMode = "preset" | "custom";

export type SettingsSection = "general" | "clipboard" | "transfer" | "launcher" | "logging";

export interface SettingsNavItem {
  key: SettingsSection;
  label: string;
  description: string;
  icon: string;
}

export interface MessageState {
  text: string;
  isError: boolean;
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

  glassTargetTheme: ResolvedTheme;
  glassThemeOptions: SelectOptionInput[];
  effectiveThemeLabel: string;
  activeGlassProfile: {
    opacity: number;
    blur: number;
    saturate: number;
    brightness: number;
  };
  onGlassThemeChange: (value: string) => void;
  onPreviewGlassField: (field: "opacity" | "blur" | "saturate" | "brightness", value: number) => void;
  onCommitGlassField: (field: "opacity" | "blur" | "saturate" | "brightness", value: number) => void;
  onResetGlassTheme: () => void;
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
  saveMessage: MessageState | null;

  onMaxItemsChange: (value: string) => void;
  onSizeCleanupEnabledChange: (checked: boolean) => void;
  onPresetSelect: (presetValue: string) => void;
  onCustomModeSelect: () => void;
  onCustomSizeChange: (value: string) => void;
  onSave: () => Promise<void>;
}

export interface TransferSettingsSectionState {
  loading: boolean;
  saving: boolean;

  defaultDirInput: string;
  autoCleanupDaysInput: string;
  resumeEnabled: boolean;
  discoveryEnabled: boolean;
  pairingRequired: boolean;

  transferDirInvalid: boolean;
  transferCleanupInvalid: boolean;
  saveMessage: MessageState | null;

  onDefaultDirChange: (value: string) => void;
  onAutoCleanupDaysChange: (value: string) => void;
  onResumeEnabledChange: (checked: boolean) => void;
  onDiscoveryEnabledChange: (checked: boolean) => void;
  onPairingRequiredChange: (checked: boolean) => void;
  onSave: () => Promise<void>;
}

export interface LauncherSettingsSectionState {
  loading: boolean;
  saving: boolean;
  rebuilding: boolean;
  resetting: boolean;

  rootsInput: string;
  excludeInput: string;
  depthInput: string;
  itemsPerRootInput: string;
  totalItemsInput: string;
  refreshInput: string;

  rootsInvalid: boolean;
  depthInvalid: boolean;
  itemsPerRootInvalid: boolean;
  totalItemsInvalid: boolean;
  refreshInvalid: boolean;

  limits: {
    depthMin: number;
    depthMax: number;
    itemsPerRootMin: number;
    itemsPerRootMax: number;
    totalItemsMin: number;
    totalItemsMax: number;
    refreshMin: number;
    refreshMax: number;
  };

  invalid: boolean;
  unchanged: boolean;

  status: LauncherIndexStatus | null;
  launcherLastBuildText: string;
  launcherLastDurationText: string;
  launcherTruncatedHintText: string | null;

  message: MessageState | null;

  onRootsChange: (value: string) => void;
  onExcludeChange: (value: string) => void;
  onDepthChange: (value: string) => void;
  onItemsPerRootChange: (value: string) => void;
  onTotalItemsChange: (value: string) => void;
  onRefreshInputChange: (value: string) => void;

  onSave: () => Promise<void>;
  onRefreshStatus: () => Promise<void>;
  onRebuildIndex: () => Promise<void>;
  onResetRecommended: () => Promise<void>;
}

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
  transfer: TransferSettingsSectionState;
  launcher: LauncherSettingsSectionState;
  logging: LoggingSettingsSectionState;
}

function parsePositiveInt(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }

  const parsed = Number.parseInt(trimmed, 10);
  if (!Number.isSafeInteger(parsed)) {
    return null;
  }

  return parsed;
}

function parseLineArray(value: string): string[] {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function formatLines(values: string[] | undefined): string {
  if (!values || values.length === 0) {
    return "";
  }
  return values.join("\n");
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
  const themeResolved = useThemeStore((state) => state.resolved);
  const glassSettings = useThemeStore((state) => state.glassSettings);
  const previewGlassProfile = useThemeStore((state) => state.previewGlassProfile);
  const commitGlassProfile = useThemeStore((state) => state.commitGlassProfile);
  const resetGlassProfile = useThemeStore((state) => state.resetGlassProfile);

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
  const [saveMessage, setSaveMessage] = useState<MessageState | null>(null);
  const [activeSection, setActiveSection] = useState<SettingsSection>("general");
  const [glassTargetTheme, setGlassTargetTheme] = useState<ResolvedTheme>(themeResolved);
  const [transferLoading, setTransferLoading] = useState(false);
  const [transferSaving, setTransferSaving] = useState(false);
  const [transferDefaultDirInput, setTransferDefaultDirInput] = useState("");
  const [transferAutoCleanupDaysInput, setTransferAutoCleanupDaysInput] = useState("30");
  const [transferResumeEnabled, setTransferResumeEnabled] = useState(true);
  const [transferDiscoveryEnabled, setTransferDiscoveryEnabled] = useState(true);
  const [transferPairingRequired, setTransferPairingRequired] = useState(true);
  const [transferSaveMessage, setTransferSaveMessage] = useState<MessageState | null>(null);
  const [launcherLoading, setLauncherLoading] = useState(false);
  const [launcherSaving, setLauncherSaving] = useState(false);
  const [launcherRebuilding, setLauncherRebuilding] = useState(false);
  const [launcherResetting, setLauncherResetting] = useState(false);
  const [launcherSettings, setLauncherSettings] = useState<LauncherSearchSettings | null>(null);
  const [launcherStatus, setLauncherStatus] = useState<LauncherIndexStatus | null>(null);
  const [launcherRootsInput, setLauncherRootsInput] = useState("");
  const [launcherExcludeInput, setLauncherExcludeInput] = useState("");
  const [launcherDepthInput, setLauncherDepthInput] = useState(String(DEFAULT_LAUNCHER_DEPTH));
  const [launcherItemsPerRootInput, setLauncherItemsPerRootInput] = useState(String(DEFAULT_LAUNCHER_ITEMS_PER_ROOT));
  const [launcherTotalItemsInput, setLauncherTotalItemsInput] = useState(String(DEFAULT_LAUNCHER_TOTAL_ITEMS));
  const [launcherRefreshInput, setLauncherRefreshInput] = useState(String(DEFAULT_LAUNCHER_REFRESH_INTERVAL));
  const [launcherMessage, setLauncherMessage] = useState<MessageState | null>(null);

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
        key: "transfer",
        label: t("section.transfer.label"),
        description: t("section.transfer.description"),
        icon: "i-noto:outbox-tray",
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

    const loadTransfer = async () => {
      setTransferLoading(true);
      try {
        const settings = await transferGetSettings();
        setTransferDefaultDirInput(settings.defaultDownloadDir);
        setTransferAutoCleanupDaysInput(String(settings.autoCleanupDays));
        setTransferResumeEnabled(settings.resumeEnabled);
        setTransferDiscoveryEnabled(settings.discoveryEnabled);
        setTransferPairingRequired(settings.pairingRequired);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setTransferSaveMessage({ text: message, isError: true });
      } finally {
        setTransferLoading(false);
      }
    };

    void loadTransfer();

    const loadLauncher = async () => {
      setLauncherLoading(true);
      try {
        const [settings, status] = await Promise.all([launcherGetSearchSettings(), launcherGetIndexStatus()]);
        setLauncherSettings(settings);
        setLauncherStatus(status);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setLauncherMessage({ text: message, isError: true });
      } finally {
        setLauncherLoading(false);
      }
    };

    void loadLauncher();
  }, [fetchClipboardSettings, fetchLoggingConfig]);

  useEffect(() => {
    setGlassTargetTheme(themeResolved);
  }, [themeResolved]);

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
    if (!launcherSettings) {
      return;
    }
    setLauncherRootsInput(formatLines(launcherSettings.roots));
    setLauncherExcludeInput(formatLines(launcherSettings.excludePatterns));
    setLauncherDepthInput(String(launcherSettings.maxScanDepth));
    setLauncherItemsPerRootInput(String(launcherSettings.maxItemsPerRoot));
    setLauncherTotalItemsInput(String(launcherSettings.maxTotalItems));
    setLauncherRefreshInput(String(launcherSettings.refreshIntervalSecs));
  }, [launcherSettings]);

  useEffect(() => {
    if (activeSection !== "launcher" || !launcherStatus?.building) {
      return;
    }
    const timer = window.setInterval(() => {
      void launcherGetIndexStatus()
        .then((status) => setLauncherStatus(status))
        .catch(() => undefined);
    }, 3000);
    return () => window.clearInterval(timer);
  }, [activeSection, launcherStatus?.building]);

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

  const parsedTransferCleanupDays = useMemo(
    () => parsePositiveInt(transferAutoCleanupDaysInput),
    [transferAutoCleanupDaysInput],
  );
  const transferDirInvalid = transferDefaultDirInput.trim().length === 0;
  const transferCleanupInvalid =
    parsedTransferCleanupDays === null || parsedTransferCleanupDays < 1 || parsedTransferCleanupDays > 365;

  const parsedLauncherDepth = useMemo(() => parsePositiveInt(launcherDepthInput), [launcherDepthInput]);
  const parsedLauncherItemsPerRoot = useMemo(
    () => parsePositiveInt(launcherItemsPerRootInput),
    [launcherItemsPerRootInput],
  );
  const parsedLauncherTotalItems = useMemo(() => parsePositiveInt(launcherTotalItemsInput), [launcherTotalItemsInput]);
  const parsedLauncherRefresh = useMemo(() => parsePositiveInt(launcherRefreshInput), [launcherRefreshInput]);
  const launcherRoots = useMemo(() => parseLineArray(launcherRootsInput), [launcherRootsInput]);
  const launcherExcludes = useMemo(() => parseLineArray(launcherExcludeInput), [launcherExcludeInput]);

  const launcherDepthInvalid =
    parsedLauncherDepth === null ||
    parsedLauncherDepth < MIN_LAUNCHER_DEPTH ||
    parsedLauncherDepth > MAX_LAUNCHER_DEPTH;
  const launcherItemsPerRootInvalid =
    parsedLauncherItemsPerRoot === null ||
    parsedLauncherItemsPerRoot < MIN_LAUNCHER_ITEMS_PER_ROOT ||
    parsedLauncherItemsPerRoot > MAX_LAUNCHER_ITEMS_PER_ROOT;
  const launcherTotalItemsInvalid =
    parsedLauncherTotalItems === null ||
    parsedLauncherTotalItems < MIN_LAUNCHER_TOTAL_ITEMS ||
    parsedLauncherTotalItems > MAX_LAUNCHER_TOTAL_ITEMS;
  const launcherRefreshInvalid =
    parsedLauncherRefresh === null ||
    parsedLauncherRefresh < MIN_LAUNCHER_REFRESH_INTERVAL ||
    parsedLauncherRefresh > MAX_LAUNCHER_REFRESH_INTERVAL;
  const launcherRootsInvalid = launcherRoots.length === 0;

  const launcherInvalid =
    launcherRootsInvalid ||
    launcherDepthInvalid ||
    launcherItemsPerRootInvalid ||
    launcherTotalItemsInvalid ||
    launcherRefreshInvalid;

  const launcherUnchanged =
    launcherSettings !== null &&
    launcherRoots.join("\n") === launcherSettings.roots.join("\n") &&
    launcherExcludes.join("\n") === launcherSettings.excludePatterns.join("\n") &&
    parsedLauncherDepth === launcherSettings.maxScanDepth &&
    parsedLauncherItemsPerRoot === launcherSettings.maxItemsPerRoot &&
    parsedLauncherTotalItems === launcherSettings.maxTotalItems &&
    parsedLauncherRefresh === launcherSettings.refreshIntervalSecs;

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

  const glassThemeOptions = useMemo(
    () => [
      { value: "light", label: t("general.glass.theme.light") },
      { value: "dark", label: t("general.glass.theme.dark") },
    ],
    [t],
  );

  const activeGlassProfile = useMemo(
    () => (glassTargetTheme === "light" ? glassSettings.light : glassSettings.dark),
    [glassSettings.dark, glassSettings.light, glassTargetTheme],
  );

  const effectiveThemeLabel = themeResolved === "dark" ? t("general.glass.theme.dark") : t("general.glass.theme.light");

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

  const launcherLastBuildText =
    launcherStatus?.lastBuildMs !== undefined &&
    launcherStatus?.lastBuildMs !== null &&
    Number.isFinite(launcherStatus.lastBuildMs)
      ? new Date(launcherStatus.lastBuildMs).toLocaleString()
      : t("launcher.statusUnknown");

  const launcherLastDurationText =
    launcherStatus?.lastDurationMs !== undefined &&
    launcherStatus?.lastDurationMs !== null &&
    Number.isFinite(launcherStatus.lastDurationMs)
      ? t("launcher.durationValue", { value: launcherStatus.lastDurationMs })
      : t("launcher.statusUnknown");

  const launcherTruncatedHintText = launcherStatus?.truncated ? t("launcher.status.truncatedHint") : null;

  const previewGlassField = (field: "opacity" | "blur" | "saturate" | "brightness", value: number) => {
    previewGlassProfile(glassTargetTheme, { [field]: value });
  };

  const commitGlassField = (field: "opacity" | "blur" | "saturate" | "brightness", value: number) => {
    void commitGlassProfile(glassTargetTheme, { [field]: value });
  };

  const handleResetGlassTheme = () => {
    void resetGlassProfile(glassTargetTheme);
  };

  const handleGlassThemeChange = (value: string) => {
    const nextTheme: ResolvedTheme = value === "dark" ? "dark" : "light";
    setGlassTargetTheme(nextTheme);
    if (themePreference !== nextTheme) {
      void setThemePreference(nextTheme);
    }
  };

  const handleSaveTransfer = async () => {
    if (transferDirInvalid || transferCleanupInvalid) {
      setTransferSaveMessage({
        text: t("transfer.saveFailedInvalid"),
        isError: true,
      });
      return;
    }

    setTransferSaving(true);
    try {
      await transferUpdateSettings({
        defaultDownloadDir: transferDefaultDirInput.trim(),
        autoCleanupDays: parsedTransferCleanupDays ?? 30,
        resumeEnabled: transferResumeEnabled,
        discoveryEnabled: transferDiscoveryEnabled,
        pairingRequired: transferPairingRequired,
      });
      setTransferSaveMessage({ text: t("transfer.saved"), isError: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setTransferSaveMessage({ text: t("transfer.saveFailed", { message }), isError: true });
    } finally {
      setTransferSaving(false);
    }
  };

  const handleRefreshLauncherStatus = async () => {
    try {
      const status = await launcherGetIndexStatus();
      setLauncherStatus(status);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setLauncherMessage({ text: message, isError: true });
    }
  };

  const handleSaveLauncher = async () => {
    if (
      launcherRootsInvalid ||
      launcherDepthInvalid ||
      launcherItemsPerRootInvalid ||
      launcherTotalItemsInvalid ||
      launcherRefreshInvalid
    ) {
      setLauncherMessage({
        text: t("launcher.saveFailedInvalid"),
        isError: true,
      });
      return;
    }

    setLauncherSaving(true);
    try {
      const settings = await launcherUpdateSearchSettings({
        roots: launcherRoots,
        excludePatterns: launcherExcludes,
        maxScanDepth: parsedLauncherDepth ?? DEFAULT_LAUNCHER_DEPTH,
        maxItemsPerRoot: parsedLauncherItemsPerRoot ?? DEFAULT_LAUNCHER_ITEMS_PER_ROOT,
        maxTotalItems: parsedLauncherTotalItems ?? DEFAULT_LAUNCHER_TOTAL_ITEMS,
        refreshIntervalSecs: parsedLauncherRefresh ?? DEFAULT_LAUNCHER_REFRESH_INTERVAL,
      });
      setLauncherSettings(settings);
      setLauncherMessage({ text: t("launcher.saved"), isError: false });
      await handleRefreshLauncherStatus();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setLauncherMessage({ text: t("launcher.saveFailed", { message }), isError: true });
    } finally {
      setLauncherSaving(false);
    }
  };

  const handleRebuildLauncherIndex = async () => {
    setLauncherRebuilding(true);
    try {
      await launcherRebuildIndex();
      await handleRefreshLauncherStatus();
      setLauncherMessage({ text: t("launcher.rebuildSuccess"), isError: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setLauncherMessage({ text: t("launcher.rebuildFailed", { message }), isError: true });
    } finally {
      setLauncherRebuilding(false);
    }
  };

  const handleResetLauncherSettings = async () => {
    setLauncherResetting(true);
    try {
      const settings = await launcherResetSearchSettings();
      setLauncherSettings(settings);
      await launcherRebuildIndex();
      await handleRefreshLauncherStatus();
      setLauncherMessage({ text: t("launcher.resetSuccess"), isError: false });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setLauncherMessage({ text: t("launcher.resetFailed", { message }), isError: true });
    } finally {
      setLauncherResetting(false);
    }
  };

  const handleSaveClipboard = async () => {
    if (parsedMaxItems === null || maxItemsInvalid) {
      setSaveMessage({
        text: t("clipboard.saveFailedInvalid", { min: MIN_MAX_ITEMS, max: MAX_MAX_ITEMS }),
        isError: true,
      });
      return;
    }

    if (effectiveMaxTotalSizeMb === null || maxTotalSizeInvalid) {
      setSaveMessage({
        text: t("clipboard.saveFailedInvalidSize", {
          min: MIN_MAX_TOTAL_SIZE_MB,
          max: MAX_MAX_TOTAL_SIZE_MB,
        }),
        isError: true,
      });
      return;
    }

    try {
      await updateClipboardSettings({
        maxItems: parsedMaxItems,
        sizeCleanupEnabled,
        maxTotalSizeMb: effectiveMaxTotalSizeMb,
      });
      setSaveMessage({ text: t("clipboard.saved"), isError: false });
    } catch (saveError) {
      const message = saveError instanceof Error ? saveError.message : String(saveError);
      const isDiskLowError = message.includes("clipboard_disk_space_low");
      setSaveMessage({
        text: isDiskLowError
          ? t("clipboard.saveFailedDiskLow", { minMb: 512 })
          : t("clipboard.saveFailed", { message }),
        isError: true,
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
    setSaveMessage(null);
  }, []);

  const onSizeCleanupEnabledChange = useCallback((checked: boolean) => {
    setSizeCleanupEnabled(checked);
    setSaveMessage(null);
  }, []);

  const onPresetSelect = useCallback((presetValue: string) => {
    setSizeThresholdMode("preset");
    setSelectedPresetMb(presetValue);
    setSaveMessage(null);
  }, []);

  const onCustomModeSelect = useCallback(() => {
    setSizeThresholdMode("custom");
    setSaveMessage(null);
  }, []);

  const onCustomSizeChange = useCallback((value: string) => {
    setCustomSizeMbInput(value);
    setSaveMessage(null);
  }, []);

  const onDefaultDirChange = useCallback((value: string) => {
    setTransferDefaultDirInput(value);
    setTransferSaveMessage(null);
  }, []);

  const onAutoCleanupDaysChange = useCallback((value: string) => {
    setTransferAutoCleanupDaysInput(value);
    setTransferSaveMessage(null);
  }, []);

  const onResumeEnabledChange = useCallback((checked: boolean) => {
    setTransferResumeEnabled(checked);
    setTransferSaveMessage(null);
  }, []);

  const onDiscoveryEnabledChange = useCallback((checked: boolean) => {
    setTransferDiscoveryEnabled(checked);
    setTransferSaveMessage(null);
  }, []);

  const onPairingRequiredChange = useCallback((checked: boolean) => {
    setTransferPairingRequired(checked);
    setTransferSaveMessage(null);
  }, []);

  const onRootsChange = useCallback((value: string) => {
    setLauncherRootsInput(value);
    setLauncherMessage(null);
  }, []);

  const onExcludeChange = useCallback((value: string) => {
    setLauncherExcludeInput(value);
    setLauncherMessage(null);
  }, []);

  const onDepthChange = useCallback((value: string) => {
    setLauncherDepthInput(value);
    setLauncherMessage(null);
  }, []);

  const onItemsPerRootChange = useCallback((value: string) => {
    setLauncherItemsPerRootInput(value);
    setLauncherMessage(null);
  }, []);

  const onTotalItemsChange = useCallback((value: string) => {
    setLauncherTotalItemsInput(value);
    setLauncherMessage(null);
  }, []);

  const onRefreshInputChange = useCallback((value: string) => {
    setLauncherRefreshInput(value);
    setLauncherMessage(null);
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
      glassTargetTheme,
      glassThemeOptions,
      effectiveThemeLabel,
      activeGlassProfile,
      onGlassThemeChange: handleGlassThemeChange,
      onPreviewGlassField: previewGlassField,
      onCommitGlassField: commitGlassField,
      onResetGlassTheme: handleResetGlassTheme,
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
      saveMessage,
      onMaxItemsChange,
      onSizeCleanupEnabledChange,
      onPresetSelect,
      onCustomModeSelect,
      onCustomSizeChange,
      onSave: handleSaveClipboard,
    },
    transfer: {
      loading: transferLoading,
      saving: transferSaving,
      defaultDirInput: transferDefaultDirInput,
      autoCleanupDaysInput: transferAutoCleanupDaysInput,
      resumeEnabled: transferResumeEnabled,
      discoveryEnabled: transferDiscoveryEnabled,
      pairingRequired: transferPairingRequired,
      transferDirInvalid,
      transferCleanupInvalid,
      saveMessage: transferSaveMessage,
      onDefaultDirChange,
      onAutoCleanupDaysChange,
      onResumeEnabledChange,
      onDiscoveryEnabledChange,
      onPairingRequiredChange,
      onSave: handleSaveTransfer,
    },
    launcher: {
      loading: launcherLoading,
      saving: launcherSaving,
      rebuilding: launcherRebuilding,
      resetting: launcherResetting,
      rootsInput: launcherRootsInput,
      excludeInput: launcherExcludeInput,
      depthInput: launcherDepthInput,
      itemsPerRootInput: launcherItemsPerRootInput,
      totalItemsInput: launcherTotalItemsInput,
      refreshInput: launcherRefreshInput,
      rootsInvalid: launcherRootsInvalid,
      depthInvalid: launcherDepthInvalid,
      itemsPerRootInvalid: launcherItemsPerRootInvalid,
      totalItemsInvalid: launcherTotalItemsInvalid,
      refreshInvalid: launcherRefreshInvalid,
      limits: {
        depthMin: MIN_LAUNCHER_DEPTH,
        depthMax: MAX_LAUNCHER_DEPTH,
        itemsPerRootMin: MIN_LAUNCHER_ITEMS_PER_ROOT,
        itemsPerRootMax: MAX_LAUNCHER_ITEMS_PER_ROOT,
        totalItemsMin: MIN_LAUNCHER_TOTAL_ITEMS,
        totalItemsMax: MAX_LAUNCHER_TOTAL_ITEMS,
        refreshMin: MIN_LAUNCHER_REFRESH_INTERVAL,
        refreshMax: MAX_LAUNCHER_REFRESH_INTERVAL,
      },
      invalid: launcherInvalid,
      unchanged: launcherUnchanged,
      status: launcherStatus,
      launcherLastBuildText,
      launcherLastDurationText,
      launcherTruncatedHintText,
      message: launcherMessage,
      onRootsChange,
      onExcludeChange,
      onDepthChange,
      onItemsPerRootChange,
      onTotalItemsChange,
      onRefreshInputChange,
      onSave: handleSaveLauncher,
      onRefreshStatus: handleRefreshLauncherStatus,
      onRebuildIndex: handleRebuildLauncherIndex,
      onResetRecommended: handleResetLauncherSettings,
    },
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

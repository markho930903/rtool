import { useCallback, useEffect, useMemo, useState } from "react";

import {
  launcherGetStatus,
  launcherResetSearchSettings,
  launcherRebuildIndex,
  launcherUpdateSearchSettings,
  type LauncherSearchSettings,
  type LauncherStatus,
} from "@/services/launcher.service";

import { formatLines, parseLineArray, parsePositiveInt, type MessageState } from "./settingsShared";

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

const LAUNCHER_LIMITS = {
  depthMin: MIN_LAUNCHER_DEPTH,
  depthMax: MAX_LAUNCHER_DEPTH,
  itemsPerRootMin: MIN_LAUNCHER_ITEMS_PER_ROOT,
  itemsPerRootMax: MAX_LAUNCHER_ITEMS_PER_ROOT,
  totalItemsMin: MIN_LAUNCHER_TOTAL_ITEMS,
  totalItemsMax: MAX_LAUNCHER_TOTAL_ITEMS,
  refreshMin: MIN_LAUNCHER_REFRESH_INTERVAL,
  refreshMax: MAX_LAUNCHER_REFRESH_INTERVAL,
} as const;

type Translate = (key: string, options?: Record<string, unknown>) => string;

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

  status: LauncherStatus | null;
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

interface UseLauncherSettingsSectionStateOptions {
  active: boolean;
  t: Translate;
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useLauncherSettingsSectionState({
  active,
  t,
}: UseLauncherSettingsSectionStateOptions): LauncherSettingsSectionState {
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [launcherSettings, setLauncherSettings] = useState<LauncherSearchSettings | null>(null);
  const [status, setStatus] = useState<LauncherStatus | null>(null);
  const [rootsInput, setRootsInput] = useState("");
  const [excludeInput, setExcludeInput] = useState("");
  const [depthInput, setDepthInput] = useState(String(DEFAULT_LAUNCHER_DEPTH));
  const [itemsPerRootInput, setItemsPerRootInput] = useState(String(DEFAULT_LAUNCHER_ITEMS_PER_ROOT));
  const [totalItemsInput, setTotalItemsInput] = useState(String(DEFAULT_LAUNCHER_TOTAL_ITEMS));
  const [refreshInput, setRefreshInput] = useState(String(DEFAULT_LAUNCHER_REFRESH_INTERVAL));
  const [message, setMessage] = useState<MessageState | null>(null);

  const fetchLauncherStatus = useCallback(async (syncSettings = false) => {
    const nextStatus = await launcherGetStatus();
    setStatus(nextStatus);

    if (syncSettings) {
      setLauncherSettings(nextStatus.settings);
    }

    return nextStatus;
  }, []);

  useEffect(() => {
    const loadLauncherStatus = async () => {
      setLoading(true);
      try {
        await fetchLauncherStatus(true);
      } catch (error) {
        setMessage({ text: toErrorMessage(error), isError: true });
      } finally {
        setLoading(false);
      }
    };

    void loadLauncherStatus();
  }, [fetchLauncherStatus]);

  useEffect(() => {
    if (!launcherSettings) {
      return;
    }

    setRootsInput(formatLines(launcherSettings.roots));
    setExcludeInput(formatLines(launcherSettings.excludePatterns));
    setDepthInput(String(launcherSettings.maxScanDepth));
    setItemsPerRootInput(String(launcherSettings.maxItemsPerRoot));
    setTotalItemsInput(String(launcherSettings.maxTotalItems));
    setRefreshInput(String(launcherSettings.refreshIntervalSecs));
  }, [launcherSettings]);

  useEffect(() => {
    if (!active || !status?.runtime.building) {
      return;
    }

    const timer = window.setInterval(() => {
      void fetchLauncherStatus().catch(() => undefined);
    }, 3000);

    return () => {
      window.clearInterval(timer);
    };
  }, [active, fetchLauncherStatus, status?.runtime.building]);

  const parsedDepth = useMemo(() => parsePositiveInt(depthInput), [depthInput]);
  const parsedItemsPerRoot = useMemo(() => parsePositiveInt(itemsPerRootInput), [itemsPerRootInput]);
  const parsedTotalItems = useMemo(() => parsePositiveInt(totalItemsInput), [totalItemsInput]);
  const parsedRefresh = useMemo(() => parsePositiveInt(refreshInput), [refreshInput]);
  const roots = useMemo(() => parseLineArray(rootsInput), [rootsInput]);
  const excludes = useMemo(() => parseLineArray(excludeInput), [excludeInput]);

  const depthInvalid =
    parsedDepth === null || parsedDepth < MIN_LAUNCHER_DEPTH || parsedDepth > MAX_LAUNCHER_DEPTH;
  const itemsPerRootInvalid =
    parsedItemsPerRoot === null ||
    parsedItemsPerRoot < MIN_LAUNCHER_ITEMS_PER_ROOT ||
    parsedItemsPerRoot > MAX_LAUNCHER_ITEMS_PER_ROOT;
  const totalItemsInvalid =
    parsedTotalItems === null ||
    parsedTotalItems < MIN_LAUNCHER_TOTAL_ITEMS ||
    parsedTotalItems > MAX_LAUNCHER_TOTAL_ITEMS;
  const refreshInvalid =
    parsedRefresh === null ||
    parsedRefresh < MIN_LAUNCHER_REFRESH_INTERVAL ||
    parsedRefresh > MAX_LAUNCHER_REFRESH_INTERVAL;
  const rootsInvalid = roots.length === 0;

  const invalid = rootsInvalid || depthInvalid || itemsPerRootInvalid || totalItemsInvalid || refreshInvalid;

  const unchanged =
    launcherSettings !== null &&
    roots.join("\n") === launcherSettings.roots.join("\n") &&
    excludes.join("\n") === launcherSettings.excludePatterns.join("\n") &&
    parsedDepth === launcherSettings.maxScanDepth &&
    parsedItemsPerRoot === launcherSettings.maxItemsPerRoot &&
    parsedTotalItems === launcherSettings.maxTotalItems &&
    parsedRefresh === launcherSettings.refreshIntervalSecs;

  const launcherLastBuildText =
    status?.index.lastBuildMs !== undefined &&
    status?.index.lastBuildMs !== null &&
    Number.isFinite(status.index.lastBuildMs)
      ? new Date(status.index.lastBuildMs).toLocaleString()
      : t("launcher.statusUnknown");

  const launcherLastDurationText =
    status?.index.lastDurationMs !== undefined &&
    status?.index.lastDurationMs !== null &&
    Number.isFinite(status.index.lastDurationMs)
      ? t("launcher.durationValue", { value: status.index.lastDurationMs })
      : t("launcher.statusUnknown");

  const launcherTruncatedHintText = status?.index.truncated ? t("launcher.status.truncatedHint") : null;

  const onRefreshStatus = useCallback(async () => {
    try {
      await fetchLauncherStatus();
    } catch (error) {
      setMessage({ text: toErrorMessage(error), isError: true });
    }
  }, [fetchLauncherStatus]);

  const onSave = useCallback(async () => {
    if (invalid) {
      setMessage({ text: t("launcher.saveFailedInvalid"), isError: true });
      return;
    }

    setSaving(true);
    try {
      const nextSettings = await launcherUpdateSearchSettings({
        roots,
        excludePatterns: excludes,
        maxScanDepth: parsedDepth ?? DEFAULT_LAUNCHER_DEPTH,
        maxItemsPerRoot: parsedItemsPerRoot ?? DEFAULT_LAUNCHER_ITEMS_PER_ROOT,
        maxTotalItems: parsedTotalItems ?? DEFAULT_LAUNCHER_TOTAL_ITEMS,
        refreshIntervalSecs: parsedRefresh ?? DEFAULT_LAUNCHER_REFRESH_INTERVAL,
      });
      setLauncherSettings(nextSettings);
      setMessage({ text: t("launcher.saved"), isError: false });
      await onRefreshStatus();
    } catch (error) {
      setMessage({ text: t("launcher.saveFailed", { message: toErrorMessage(error) }), isError: true });
    } finally {
      setSaving(false);
    }
  }, [excludes, invalid, onRefreshStatus, parsedDepth, parsedItemsPerRoot, parsedRefresh, parsedTotalItems, roots, t]);

  const onRebuildIndex = useCallback(async () => {
    setRebuilding(true);
    try {
      await launcherRebuildIndex();
      await onRefreshStatus();
      setMessage({ text: t("launcher.rebuildSuccess"), isError: false });
    } catch (error) {
      setMessage({ text: t("launcher.rebuildFailed", { message: toErrorMessage(error) }), isError: true });
    } finally {
      setRebuilding(false);
    }
  }, [onRefreshStatus, t]);

  const onResetRecommended = useCallback(async () => {
    setResetting(true);
    try {
      const nextSettings = await launcherResetSearchSettings();
      setLauncherSettings(nextSettings);
      await launcherRebuildIndex();
      await onRefreshStatus();
      setMessage({ text: t("launcher.resetSuccess"), isError: false });
    } catch (error) {
      setMessage({ text: t("launcher.resetFailed", { message: toErrorMessage(error) }), isError: true });
    } finally {
      setResetting(false);
    }
  }, [onRefreshStatus, t]);

  const onRootsChange = useCallback((value: string) => {
    setRootsInput(value);
    setMessage(null);
  }, []);

  const onExcludeChange = useCallback((value: string) => {
    setExcludeInput(value);
    setMessage(null);
  }, []);

  const onDepthChange = useCallback((value: string) => {
    setDepthInput(value);
    setMessage(null);
  }, []);

  const onItemsPerRootChange = useCallback((value: string) => {
    setItemsPerRootInput(value);
    setMessage(null);
  }, []);

  const onTotalItemsChange = useCallback((value: string) => {
    setTotalItemsInput(value);
    setMessage(null);
  }, []);

  const onRefreshInputChange = useCallback((value: string) => {
    setRefreshInput(value);
    setMessage(null);
  }, []);

  return {
    loading,
    saving,
    rebuilding,
    resetting,
    rootsInput,
    excludeInput,
    depthInput,
    itemsPerRootInput,
    totalItemsInput,
    refreshInput,
    rootsInvalid,
    depthInvalid,
    itemsPerRootInvalid,
    totalItemsInvalid,
    refreshInvalid,
    limits: LAUNCHER_LIMITS,
    invalid,
    unchanged,
    status,
    launcherLastBuildText,
    launcherLastDurationText,
    launcherTruncatedHintText,
    message,
    onRootsChange,
    onExcludeChange,
    onDepthChange,
    onItemsPerRootChange,
    onTotalItemsChange,
    onRefreshInputChange,
    onSave,
    onRefreshStatus,
    onRebuildIndex,
    onResetRecommended,
  };
}

import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { SUPPORTED_LOCALES } from "@/i18n/constants";
import { localeActions, useLocaleStore } from "@/i18n/store";
import type { LocalePreference } from "@/i18n/types";
import { useLayoutStore } from "@/layouts/layout.store";
import type { LayoutPreference } from "@/layouts/layout.types";
import { Button, Checkbox, Input, Select } from "@/components/ui";
import type { SelectOptionInput } from "@/components/ui";
import {
  importBackendLocaleFile,
  listBackendLocales,
  reloadBackendLocales,
  type BackendLocaleCatalogList,
} from "@/services/locale.service";
import { useLoggingStore } from "@/stores/logging.store";
import { useSettingsStore } from "@/stores/settings.store";

const MIN_MAX_ITEMS = 100;
const MAX_MAX_ITEMS = 10_000;
const MIN_KEEP_DAYS = 1;
const MAX_KEEP_DAYS = 90;
const MIN_HIGH_FREQ_WINDOW_MS = 100;
const MAX_HIGH_FREQ_WINDOW_MS = 60_000;
const MIN_HIGH_FREQ_MAX_PER_KEY = 1;
const MAX_HIGH_FREQ_MAX_PER_KEY = 200;
const LOG_KEEP_DAYS_PRESETS = ["1", "3", "7", "14", "30", "60", "90"];
const LOG_WINDOW_MS_PRESETS = ["100", "250", "500", "1000", "2000", "5000", "10000", "30000", "60000"];
const LOG_MAX_PER_KEY_PRESETS = ["1", "5", "10", "20", "50", "100", "200"];

type SettingsSection = "general" | "clipboard" | "launcher" | "logging";

interface SettingsNavItem {
  key: SettingsSection;
  label: string;
  description: string;
  icon: string;
}

interface MessageState {
  text: string;
  isError: boolean;
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

export default function SettingsPage() {
  const { t } = useTranslation("settings");

  const localePreference = useLocaleStore((state) => state.preference);
  const resolvedLocale = useLocaleStore((state) => state.resolved);
  const setLocalePreference = useLocaleStore((state) => state.setPreference);
  const layoutPreference = useLayoutStore((state) => state.preference);
  const setLayoutPreference = useLayoutStore((state) => state.setPreference);

  const clipboardSettings = useSettingsStore((state) => state.clipboardSettings);
  const clipboardLoading = useSettingsStore((state) => state.loading);
  const clipboardSaving = useSettingsStore((state) => state.saving);
  const clipboardError = useSettingsStore((state) => state.error);
  const fetchClipboardSettings = useSettingsStore((state) => state.fetchClipboardSettings);
  const updateClipboardMaxItems = useSettingsStore((state) => state.updateClipboardMaxItems);

  const loggingConfig = useLoggingStore((state) => state.config);
  const loggingError = useLoggingStore((state) => state.error);
  const fetchLoggingConfig = useLoggingStore((state) => state.fetchConfig);
  const updateLoggingConfig = useLoggingStore((state) => state.saveConfig);

  const [maxItemsInput, setMaxItemsInput] = useState(String(clipboardSettings?.maxItems ?? 1000));
  const [saveMessage, setSaveMessage] = useState<MessageState | null>(null);
  const [activeSection, setActiveSection] = useState<SettingsSection>("general");

  const [logMinLevel, setLogMinLevel] = useState("info");
  const [logKeepDaysInput, setLogKeepDaysInput] = useState(String(MIN_KEEP_DAYS));
  const [logRealtimeEnabled, setLogRealtimeEnabled] = useState(true);
  const [logHighFreqWindowMsInput, setLogHighFreqWindowMsInput] = useState(String(1000));
  const [logHighFreqMaxPerKeyInput, setLogHighFreqMaxPerKeyInput] = useState(String(20));
  const [logAllowRawView, setLogAllowRawView] = useState(false);
  const [loggingSaveMessage, setLoggingSaveMessage] = useState<MessageState | null>(null);
  const [localeCatalog, setLocaleCatalog] = useState<BackendLocaleCatalogList | null>(null);
  const [localeCatalogLoading, setLocaleCatalogLoading] = useState(false);
  const [localeCatalogError, setLocaleCatalogError] = useState<string | null>(null);
  const [importLocale, setImportLocale] = useState("zh-CN");
  const [importNamespace, setImportNamespace] = useState("native");
  const [importFileName, setImportFileName] = useState("");
  const [importFileContent, setImportFileContent] = useState("");
  const [importingLocaleFile, setImportingLocaleFile] = useState(false);
  const [importMessage, setImportMessage] = useState<MessageState | null>(null);

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
  }, [fetchClipboardSettings, fetchLoggingConfig]);

  const refreshLocaleCatalog = useCallback(async () => {
    setLocaleCatalogLoading(true);
    setLocaleCatalogError(null);
    try {
      const next = await listBackendLocales();
      setLocaleCatalog(next);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setLocaleCatalogError(message);
    } finally {
      setLocaleCatalogLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshLocaleCatalog();
  }, [refreshLocaleCatalog]);

  useEffect(() => {
    if (clipboardSettings) {
      setMaxItemsInput(String(clipboardSettings.maxItems));
    }
  }, [clipboardSettings]);

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
  const clipboardInvalid = parsedMaxItems === null || parsedMaxItems < MIN_MAX_ITEMS || parsedMaxItems > MAX_MAX_ITEMS;
  const clipboardUnchanged = parsedMaxItems !== null && parsedMaxItems === (clipboardSettings?.maxItems ?? null);

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

  const localePreferenceOptions = useMemo(() => {
    const values = new Set<string>(SUPPORTED_LOCALES);
    for (const item of localeCatalog?.effectiveLocales ?? []) {
      values.add(item.locale);
    }

    const sortedValues = [...values].sort((left, right) => left.localeCompare(right));
    return [
      { value: "system", label: t("general.option.system") },
      ...sortedValues.map((value) => ({
        value,
        label: localeDisplayLabel(value, t),
      })),
    ];
  }, [localeCatalog, t]);

  const importLocaleOptions = useMemo(() => {
    const values = new Set<string>(SUPPORTED_LOCALES);
    for (const item of localeCatalog?.effectiveLocales ?? []) {
      values.add(item.locale);
    }
    const sorted = [...values].sort((left, right) => left.localeCompare(right));
    return sorted.map((value) => ({ value, label: value }));
  }, [localeCatalog]);

  const importNamespaceOptions = useMemo(() => {
    const localeItem = localeCatalog?.effectiveLocales.find((item) => item.locale === importLocale);
    const values = localeItem?.namespaces.length ? localeItem.namespaces : ["native"];
    return values.map((value) => ({ value, label: value }));
  }, [localeCatalog, importLocale]);

  const layoutPreferenceOptions = useMemo(
    () => [
      { value: "topbar", label: t("general.layout.option.topbar") },
      { value: "sidebar", label: t("general.layout.option.sidebar") },
    ],
    [t],
  );

  useEffect(() => {
    if (!importLocaleOptions.some((item) => item.value === importLocale)) {
      setImportLocale(importLocaleOptions[0]?.value ?? "zh-CN");
    }
  }, [importLocale, importLocaleOptions]);

  useEffect(() => {
    if (!importNamespaceOptions.some((item) => item.value === importNamespace)) {
      setImportNamespace(importNamespaceOptions[0]?.value ?? "native");
    }
  }, [importNamespace, importNamespaceOptions]);

  const catalogSummary = useMemo(() => {
    const builtin = localeCatalog?.builtinLocales.map((item) => item.locale).join(", ") || "--";
    const overlay = localeCatalog?.overlayLocales.map((item) => item.locale).join(", ") || "--";
    const effective = localeCatalog?.effectiveLocales.map((item) => item.locale).join(", ") || "--";
    return { builtin, overlay, effective };
  }, [localeCatalog]);

  const clipboardHelperText = clipboardInvalid
    ? t("clipboard.invalid", { min: MIN_MAX_ITEMS, max: MAX_MAX_ITEMS })
    : t("clipboard.helper");

  const handleSaveClipboard = async () => {
    if (parsedMaxItems === null || clipboardInvalid) {
      setSaveMessage({
        text: t("clipboard.saveFailedInvalid", { min: MIN_MAX_ITEMS, max: MAX_MAX_ITEMS }),
        isError: true,
      });
      return;
    }

    try {
      await updateClipboardMaxItems(parsedMaxItems);
      setSaveMessage({ text: t("clipboard.saved"), isError: false });
    } catch (saveError) {
      const message = saveError instanceof Error ? saveError.message : String(saveError);
      setSaveMessage({ text: t("clipboard.saveFailed", { message }), isError: true });
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

  const handleSelectImportFile = async (file: File | null) => {
    if (!file) {
      setImportFileName("");
      setImportFileContent("");
      return;
    }

    const content = await file.text();
    setImportFileName(file.name);
    setImportFileContent(content);
    setImportMessage(null);
  };

  const handleReloadLocales = async () => {
    setLocaleCatalogLoading(true);
    setLocaleCatalogError(null);
    try {
      const result = await reloadBackendLocales();
      await refreshLocaleCatalog();
      await localeActions.syncFromBackend();
      if (result.warnings.length > 0) {
        setImportMessage({
          text: t("general.import.reloadWithWarnings", { count: result.warnings.length }),
          isError: false,
        });
      } else {
        setImportMessage({
          text: t("general.import.reloadSuccess", { count: result.reloadedFiles }),
          isError: false,
        });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setImportMessage({
        text: t("general.import.reloadFailed", { message }),
        isError: true,
      });
    } finally {
      setLocaleCatalogLoading(false);
    }
  };

  const handleImportLocaleFile = async () => {
    if (!importFileContent.trim()) {
      setImportMessage({
        text: t("general.import.emptyFile"),
        isError: true,
      });
      return;
    }

    setImportingLocaleFile(true);
    setImportMessage(null);
    try {
      const result = await importBackendLocaleFile({
        locale: importLocale,
        namespace: importNamespace,
        content: importFileContent,
        replace: true,
      });
      await refreshLocaleCatalog();
      await localeActions.syncFromBackend();
      if (result.warnings.length > 0) {
        setImportMessage({
          text: t("general.import.successWithWarnings", {
            count: result.importedKeys,
            warnings: result.warnings.length,
          }),
          isError: false,
        });
      } else {
        setImportMessage({
          text: t("general.import.success", { count: result.importedKeys }),
          isError: false,
        });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setImportMessage({
        text: t("general.import.failed", { message }),
        isError: true,
      });
    } finally {
      setImportingLocaleFile(false);
    }
  };

  return (
    <div className="h-full min-h-0">
      <div className="grid h-full min-h-0 grid-cols-1 md:grid-cols-[220px_1fr]">
        <aside className="h-full min-h-0 border-b border-border-muted bg-surface md:border-b-0 md:border-r">
          <nav className="flex h-full flex-col py-5" aria-label={t("nav.aria")}>
            {settingsNavItems.map((item) => {
              const active = item.key === activeSection;
              return (
                <Button
                  unstyled
                  key={item.key}
                  type="button"
                  className={[
                    "w-full border-b border-border-muted/70 px-4 py-3 text-left transition-colors last:border-b-0",
                    active
                      ? "bg-accent-soft text-text-primary"
                      : "text-text-secondary hover:bg-surface-soft hover:text-text-primary",
                  ].join(" ")}
                  onClick={() => setActiveSection(item.key)}
                  aria-current={active ? "page" : undefined}
                >
                  <div className="flex items-start gap-2.5">
                    <span
                      className={`settings-nav-icon btn-icon mt-0.5 shrink-0 text-[1rem] ${item.icon}`}
                      aria-hidden="true"
                    />
                    <div className="min-w-0">
                      <div className="text-sm font-semibold">{item.label}</div>
                      <div className="mt-0.5 text-xs text-text-muted">{item.description}</div>
                    </div>
                  </div>
                </Button>
              );
            })}
          </nav>
        </aside>

        <div className="min-h-0 overflow-y-auto p-4">
          {activeSection === "general" ? (
            <section className="h-full min-h-0">
              <div className="space-y-3">
                <h2 className="m-0 text-sm font-semibold text-text-primary">{t("general.title")}</h2>
                <p className="m-0 text-xs text-text-muted">{t("general.desc")}</p>

                <div className="grid max-w-[420px] gap-2">
                  <label htmlFor="locale-preference" className="text-xs text-text-secondary">
                    {t("general.preference")}
                  </label>
                  <Select
                    id="locale-preference"
                    value={localePreference}
                    options={localePreferenceOptions}
                    onChange={(event) => setLocalePreference(event.currentTarget.value as LocalePreference)}
                  />
                  <p className="m-0 text-xs text-text-muted">
                    {t("general.effective", {
                      locale: localeDisplayLabel(resolvedLocale, t),
                    })}
                  </p>
                </div>

                <div className="grid max-w-[420px] gap-2">
                  <label htmlFor="layout-preference" className="text-xs text-text-secondary">
                    {t("general.layoutPreference")}
                  </label>
                  <Select
                    id="layout-preference"
                    value={layoutPreference}
                    options={layoutPreferenceOptions}
                    onChange={(event) => setLayoutPreference(event.currentTarget.value as LayoutPreference)}
                  />
                </div>

                <div className="mt-5 space-y-3 rounded-lg border border-border-muted bg-surface-soft px-3 py-3">
                  <h3 className="m-0 text-sm font-semibold text-text-primary">{t("general.import.title")}</h3>
                  <p className="m-0 text-xs text-text-muted">{t("general.import.desc")}</p>

                  <p className="m-0 text-xs text-text-secondary">
                    {t("general.import.catalog", {
                      builtin: catalogSummary.builtin,
                      overlay: catalogSummary.overlay,
                      effective: catalogSummary.effective,
                    })}
                  </p>

                  <div className="grid max-w-[520px] gap-2 md:grid-cols-2">
                    <div className="space-y-1">
                      <label htmlFor="locale-import-locale" className="text-xs text-text-secondary">
                        {t("general.import.locale")}
                      </label>
                      <Select
                        id="locale-import-locale"
                        value={importLocale}
                        options={importLocaleOptions}
                        onChange={(event) => {
                          setImportLocale(event.currentTarget.value);
                          setImportMessage(null);
                        }}
                      />
                    </div>

                    <div className="space-y-1">
                      <label htmlFor="locale-import-namespace" className="text-xs text-text-secondary">
                        {t("general.import.namespace")}
                      </label>
                      <Select
                        id="locale-import-namespace"
                        value={importNamespace}
                        options={importNamespaceOptions}
                        onChange={(event) => {
                          setImportNamespace(event.currentTarget.value);
                          setImportMessage(null);
                        }}
                      />
                    </div>
                  </div>

                  <div className="grid max-w-[520px] gap-2">
                    <label htmlFor="locale-import-file" className="text-xs text-text-secondary">
                      {t("general.import.file")}
                    </label>
                    <Input
                      id="locale-import-file"
                      type="file"
                      accept=".json,application/json"
                      onChange={(event) => {
                        void handleSelectImportFile(event.currentTarget.files?.[0] ?? null);
                      }}
                    />
                    <p className="m-0 text-xs text-text-muted">
                      {importFileName
                        ? t("general.import.selected", { name: importFileName })
                        : t("general.import.noFile")}
                    </p>
                  </div>

                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      size="sm"
                      variant="primary"
                      disabled={importingLocaleFile || !importFileContent.trim()}
                      onClick={() => void handleImportLocaleFile()}
                    >
                      {importingLocaleFile ? t("general.import.importing") : t("general.import.import")}
                    </Button>
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={localeCatalogLoading}
                      onClick={() => void handleReloadLocales()}
                    >
                      {t("general.import.reload")}
                    </Button>
                    {localeCatalogLoading ? (
                      <span className="text-xs text-text-muted">{t("common:status.loading")}</span>
                    ) : null}
                  </div>

                  {localeCatalogError ? <p className="m-0 text-xs text-danger">{localeCatalogError}</p> : null}
                  {importMessage ? (
                    <p className={`m-0 text-xs ${importMessage.isError ? "text-danger" : "text-text-secondary"}`}>
                      {importMessage.text}
                    </p>
                  ) : null}
                </div>
              </div>
            </section>
          ) : null}

          {activeSection === "clipboard" ? (
            <section className="h-full min-h-0">
              <div className="space-y-3">
                <h2 className="m-0 text-sm font-semibold text-text-primary">{t("clipboard.title")}</h2>
                <p className="m-0 text-xs text-text-muted">{t("clipboard.desc")}</p>

                <div className="grid max-w-[420px] gap-2">
                  <label htmlFor="clipboard-max-items" className="text-xs text-text-secondary">
                    {t("clipboard.maxItems")}
                  </label>
                  <Input
                    id="clipboard-max-items"
                    type="number"
                    min={MIN_MAX_ITEMS}
                    max={MAX_MAX_ITEMS}
                    value={maxItemsInput}
                    invalid={clipboardInvalid}
                    onChange={(event) => {
                      setMaxItemsInput(event.currentTarget.value);
                      setSaveMessage(null);
                    }}
                  />
                  <p className={`m-0 text-xs ${clipboardInvalid ? "text-danger" : "text-text-muted"}`}>
                    {clipboardHelperText}
                  </p>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    size="sm"
                    variant="primary"
                    disabled={clipboardLoading || clipboardSaving || clipboardInvalid || clipboardUnchanged}
                    onClick={() => void handleSaveClipboard()}
                  >
                    {clipboardSaving ? t("common:action.saving") : t("common:action.save")}
                  </Button>
                  {clipboardLoading ? (
                    <span className="text-xs text-text-muted">{t("common:status.loading")}</span>
                  ) : null}
                  {clipboardError ? <span className="text-xs text-danger">{clipboardError}</span> : null}
                  {saveMessage ? (
                    <span className={`text-xs ${saveMessage.isError ? "text-danger" : "text-text-secondary"}`}>
                      {saveMessage.text}
                    </span>
                  ) : null}
                </div>
              </div>
            </section>
          ) : null}

          {activeSection === "launcher" ? (
            <section className="h-full min-h-0">
              <div className="space-y-3">
                <h2 className="m-0 text-sm font-semibold text-text-primary">{t("launcher.title")}</h2>
                <p className="m-0 text-xs text-text-muted">{t("launcher.desc")}</p>
                <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-2 text-xs text-text-secondary">
                  {t("launcher.tip")}
                </div>
              </div>
            </section>
          ) : null}

          {activeSection === "logging" ? (
            <section className="h-full min-h-0">
              <div className="space-y-3">
                <h2 className="m-0 text-sm font-semibold text-text-primary">{t("logging.title")}</h2>
                <p className="m-0 text-xs text-text-muted">{t("logging.desc")}</p>

                <div className="max-w-[560px] space-y-3">
                  <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-3">
                    <div className="space-y-1">
                      <label className="text-xs text-text-secondary" htmlFor="logging-min-level">
                        {t("logging.minLevel")}
                      </label>
                      <Select
                        id="logging-min-level"
                        value={logMinLevel}
                        options={[
                          { value: "trace", label: "trace" },
                          { value: "debug", label: "debug" },
                          { value: "info", label: "info" },
                          { value: "warn", label: "warn" },
                          { value: "error", label: "error" },
                        ]}
                        onChange={(event) => {
                          setLogMinLevel(event.currentTarget.value);
                          setLoggingSaveMessage(null);
                        }}
                      />
                    </div>
                  </div>

                  <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-3">
                    <div className="space-y-1">
                      <label className="text-xs text-text-secondary" htmlFor="logging-keep-days">
                        {t("logging.keepDays", { min: MIN_KEEP_DAYS, max: MAX_KEEP_DAYS })}
                      </label>
                      <Select
                        id="logging-keep-days"
                        value={logKeepDaysInput}
                        invalid={logKeepDaysInvalid}
                        options={logKeepDaysOptions}
                        onChange={(event) => {
                          setLogKeepDaysInput(event.currentTarget.value);
                          setLoggingSaveMessage(null);
                        }}
                      />
                    </div>
                  </div>

                  <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-3">
                    <div className="space-y-1">
                      <label className="text-xs text-text-secondary" htmlFor="logging-high-freq-window">
                        {t("logging.windowMs", { min: MIN_HIGH_FREQ_WINDOW_MS, max: MAX_HIGH_FREQ_WINDOW_MS })}
                      </label>
                      <Select
                        id="logging-high-freq-window"
                        value={logHighFreqWindowMsInput}
                        invalid={logHighFreqWindowInvalid}
                        options={logWindowMsOptions}
                        onChange={(event) => {
                          setLogHighFreqWindowMsInput(event.currentTarget.value);
                          setLoggingSaveMessage(null);
                        }}
                      />
                    </div>
                  </div>

                  <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-3">
                    <div className="space-y-1">
                      <label className="text-xs text-text-secondary" htmlFor="logging-high-freq-max">
                        {t("logging.maxPerKey", { min: MIN_HIGH_FREQ_MAX_PER_KEY, max: MAX_HIGH_FREQ_MAX_PER_KEY })}
                      </label>
                      <Select
                        id="logging-high-freq-max"
                        value={logHighFreqMaxPerKeyInput}
                        invalid={logHighFreqMaxPerKeyInvalid}
                        options={logMaxPerKeyOptions}
                        onChange={(event) => {
                          setLogHighFreqMaxPerKeyInput(event.currentTarget.value);
                          setLoggingSaveMessage(null);
                        }}
                      />
                    </div>
                  </div>

                  <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-3">
                    <Checkbox
                      size="sm"
                      className="mt-0.5"
                      checked={logRealtimeEnabled}
                      onChange={(event) => {
                        setLogRealtimeEnabled(event.currentTarget.checked);
                        setLoggingSaveMessage(null);
                      }}
                      wrapperClassName="items-start gap-2 text-sm text-text-primary"
                      labelClassName="gap-1"
                      label={<span className="text-sm font-medium leading-5">{t("logging.realtime.label")}</span>}
                      description={<span className="leading-5">{t("logging.realtime.desc")}</span>}
                    />
                  </div>

                  <div className="rounded-lg border border-border-muted bg-surface-soft px-3 py-3">
                    <Checkbox
                      size="sm"
                      className="mt-0.5"
                      checked={logAllowRawView}
                      onChange={(event) => {
                        setLogAllowRawView(event.currentTarget.checked);
                        setLoggingSaveMessage(null);
                      }}
                      wrapperClassName="items-start gap-2 text-sm text-text-primary"
                      labelClassName="gap-1"
                      label={<span className="text-sm font-medium leading-5">{t("logging.raw.label")}</span>}
                      description={<span className="leading-5">{t("logging.raw.desc")}</span>}
                    />
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    size="sm"
                    variant="primary"
                    disabled={loggingInvalid || loggingUnchanged}
                    onClick={() => void handleSaveLogging()}
                  >
                    {t("logging.save")}
                  </Button>
                  {loggingError ? <span className="text-xs text-danger">{loggingError}</span> : null}
                  {loggingSaveMessage ? (
                    <span className={`text-xs ${loggingSaveMessage.isError ? "text-danger" : "text-text-secondary"}`}>
                      {loggingSaveMessage.text}
                    </span>
                  ) : null}
                </div>
              </div>
            </section>
          ) : null}
        </div>
      </div>
    </div>
  );
}

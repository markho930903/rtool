import { listen } from "@tauri-apps/api/event";
import { currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppEntityIcon } from "@/components/icons/AppEntityIcon";
import { BootOverlay, useBootState } from "@/components/loading";
import PaletteInput from "@/components/palette/PaletteInput";
import PalettePreview from "@/components/palette/PalettePreview";
import type { PaletteItem } from "@/components/palette/types";
import { Button } from "@/components/ui";
import { useWindowFocusAutoHide } from "@/hooks/window/useWindowFocusAutoHide";
import { useWindowLayoutPersistence } from "@/hooks/window/useWindowLayoutPersistence";
import { useLocaleStore } from "@/i18n/store";
import { useLauncherStore } from "@/stores/launcher.store";
import { useThemeStore } from "@/theme/store";

const LAUNCHER_WINDOW_LABEL = "launcher";
const WINDOW_LAYOUT_KEY = "launcher-window-layout";

interface GroupedItem {
  item: PaletteItem;
  index: number;
}

interface GroupedCategory {
  key: string;
  label: string;
  items: GroupedItem[];
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function renderHighlightedText(text: string, query: string): ReactNode {
  const tokens = query
    .trim()
    .split(/\s+/)
    .map((token) => token.trim())
    .filter(Boolean);

  if (tokens.length === 0) {
    return text;
  }

  const matcher = new RegExp(`(${tokens.map(escapeRegExp).join("|")})`, "ig");
  const tokenSet = new Set(tokens.map((token) => token.toLowerCase()));
  const parts = text.split(matcher);

  return parts.map((part, index) => {
    const lower = part.toLowerCase();
    if (tokenSet.has(lower)) {
      return (
        <mark key={`${lower}-${index}`} className="rounded bg-accent-soft px-[1px] font-semibold text-accent">
          {part}
        </mark>
      );
    }
    return <span key={`${lower}-${index}`}>{part}</span>;
  });
}

function categoryLabel(key: string, t: (key: string, options?: Record<string, unknown>) => string): string {
  if (key === "builtin") {
    return t("category.builtin");
  }

  if (key === "application") {
    return t("category.application");
  }

  if (key === "directory") {
    return t("category.directory");
  }

  if (key === "file") {
    return t("category.file");
  }

  if (key === "action") {
    return t("category.action");
  }

  return t("category.other");
}

function groupItems(
  items: PaletteItem[],
  t: (key: string, options?: Record<string, unknown>) => string,
): GroupedCategory[] {
  const groups = new Map<string, GroupedCategory>();

  items.forEach((item, index) => {
    const key = item.category || "other";
    const label = categoryLabel(key, t);

    if (!groups.has(key)) {
      groups.set(key, { key, label, items: [] });
    }

    groups.get(key)?.items.push({ item, index });
  });

  return ["builtin", "application", "directory", "file", "action", "other"]
    .map((key) => groups.get(key))
    .filter((group): group is GroupedCategory => Boolean(group));
}

function LauncherItemIcon({ item }: { item: PaletteItem }) {
  return (
    <AppEntityIcon
      iconKind={item.iconKind}
      iconValue={item.iconValue}
      fallbackIcon="i-noto:card-index-dividers"
      imgClassName="h-5 w-5 shrink-0 rounded-sm object-cover"
      iconClassName="h-5 w-5 shrink-0 text-[1.05rem] text-text-muted"
    />
  );
}

export default function LauncherWindowPage() {
  const { t } = useTranslation("palette");
  const appWindow = useMemo(() => getCurrentWindow(), []);
  const inputRef = useRef<HTMLInputElement>(null);
  const launcherItemRefs = useRef<Map<string, HTMLLIElement>>(new Map());
  const [openCycle, setOpenCycle] = useState(1);
  const [searchSeed, setSearchSeed] = useState(0);
  const [hasSearchedOnce, setHasSearchedOnce] = useState(false);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const alwaysOnTopRef = useRef(false);

  const query = useLauncherStore((state) => state.query);
  const items = useLauncherStore((state) => state.items);
  const selectedIndex = useLauncherStore((state) => state.selectedIndex);
  const loading = useLauncherStore((state) => state.loading);
  const launcherError = useLauncherStore((state) => state.error);
  const reset = useLauncherStore((state) => state.reset);
  const search = useLauncherStore((state) => state.search);
  const moveSelection = useLauncherStore((state) => state.moveSelection);
  const setSelectedIndex = useLauncherStore((state) => state.setSelectedIndex);
  const executeSelected = useLauncherStore((state) => state.executeSelected);
  const setQuery = useLauncherStore((state) => state.setQuery);
  const syncFromStorage = useThemeStore((state) => state.syncFromStorage);
  const syncLocaleFromBackend = useLocaleStore((state) => state.syncFromBackend);
  const searchWithBootMark = useCallback(
    async (limit?: number) => {
      setHasSearchedOnce(true);
      await search(limit);
    },
    [search],
  );
  const bootReady = hasSearchedOnce && !loading;
  const { mounted: bootMounted, visible: bootVisible } = useBootState({
    cycleKey: openCycle,
    ready: bootReady,
    delayMs: 220,
    minVisibleMs: 180,
    maxWaitMs: 1500,
    exitMs: 160,
  });

  const selectedItem = useMemo(() => items[selectedIndex] ?? null, [items, selectedIndex]);
  const groupedItems = useMemo(() => groupItems(items, t), [items, t]);
  const enabled = appWindow.label === LAUNCHER_WINDOW_LABEL;
  const resolveLayoutBounds = useCallback(async () => {
    const monitor = await currentMonitor();
    const monitorSize = monitor?.size;
    if (!monitorSize) {
      return null;
    }

    return {
      monitorWidth: monitorSize.width,
      monitorHeight: monitorSize.height,
      minWidth: 780,
      minHeight: 540,
    };
  }, []);

  useWindowLayoutPersistence({
    appWindow,
    storageKey: WINDOW_LAYOUT_KEY,
    scope: "launcher-window",
    enabled,
    resolveBounds: resolveLayoutBounds,
  });

  const shouldSkipHide = useCallback(() => alwaysOnTopRef.current, []);

  const { cancelScheduledHide } = useWindowFocusAutoHide({
    appWindow,
    enabled,
    shouldSkipHide,
    onFocus: () => {
      syncFromStorage();
      void syncLocaleFromBackend();
    },
  });

  const syncAlwaysOnTopState = useCallback(() => {
    void appWindow
      .isAlwaysOnTop()
      .then((result) => {
        alwaysOnTopRef.current = result;
        setAlwaysOnTop(result);
      })
      .catch((caughtError: unknown) => {
        console.warn("[launcher-window] read always-on-top failed", { error: caughtError });
      });
  }, [appWindow]);

  const handleAlwaysOnTopToggle = useCallback(() => {
    const next = !alwaysOnTop;
    void appWindow
      .setAlwaysOnTop(next)
      .then(() => {
        if (next) {
          cancelScheduledHide();
        }
        alwaysOnTopRef.current = next;
        setAlwaysOnTop(next);
      })
      .catch((caughtError: unknown) => {
        console.warn("[launcher-window] toggle always-on-top failed", { next, error: caughtError });
      });
  }, [alwaysOnTop, appWindow, cancelScheduledHide]);

  useEffect(() => {
    if (!enabled) {
      return;
    }
    syncAlwaysOnTopState();
  }, [enabled, syncAlwaysOnTopState]);

  useEffect(() => {
    alwaysOnTopRef.current = alwaysOnTop;
  }, [alwaysOnTop]);

  useEffect(() => {
    if (!selectedItem?.id) {
      return;
    }

    const selectedNode = launcherItemRefs.current.get(selectedItem.id);
    selectedNode?.scrollIntoView({ block: "nearest" });
  }, [selectedItem?.id]);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const setup = async () => {
      const unlistenOpened = await listen("rtool://launcher/opened", () => {
        cancelScheduledHide();
        syncAlwaysOnTopState();

        setOpenCycle((value) => value + 1);
        setSearchSeed((value) => value + 1);
        setHasSearchedOnce(false);
        reset();
        syncFromStorage();
        void syncLocaleFromBackend();

        window.setTimeout(() => {
          inputRef.current?.focus();
        }, 40);
      });

      const onKeyDown = (event: KeyboardEvent) => {
        if (event.key === "Escape") {
          event.preventDefault();
          void appWindow.hide();
          return;
        }

        if (event.key === "ArrowDown") {
          event.preventDefault();
          moveSelection(1);
          return;
        }

        if (event.key === "ArrowUp") {
          event.preventDefault();
          moveSelection(-1);
          return;
        }

        if (event.key === "Enter") {
          if (event.isComposing || event.keyCode === 229) {
            return;
          }
          event.preventDefault();
          void executeSelected().then((result) => {
            if (result?.ok) {
              void appWindow.hide();
            }
          });
        }
      };

      window.addEventListener("keydown", onKeyDown);

      return () => {
        unlistenOpened();
        window.removeEventListener("keydown", onKeyDown);
      };
    };

    let cleanup: (() => void) | undefined;
    let disposed = false;

    void setup().then((fn) => {
      if (disposed) {
        fn?.();
        return;
      }
      cleanup = fn;
    });

    return () => {
      disposed = true;
      cleanup?.();
    };
  }, [
    appWindow,
    cancelScheduledHide,
    enabled,
    executeSelected,
    moveSelection,
    reset,
    searchWithBootMark,
    syncAlwaysOnTopState,
    syncFromStorage,
    syncLocaleFromBackend,
  ]);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const timer = window.setTimeout(() => {
      void searchWithBootMark(60);
    }, 120);

    return () => window.clearTimeout(timer);
  }, [enabled, query, searchSeed, searchWithBootMark]);

  if (!enabled) {
    return null;
  }

  const alwaysOnTopLabel = alwaysOnTop ? t("launcher.pinWindowOff") : t("launcher.pinWindowOn");

  return (
    <div className="relative h-screen w-screen overflow-hidden rounded-md bg-transparent p-0">
      <section className="flex h-full w-full overflow-hidden rounded-md border border-border-muted/85 bg-surface-overlay shadow-overlay backdrop-blur-[24px] backdrop-saturate-140">
        <div className="flex min-w-0 flex-[1.4] flex-col border-r border-border-muted/85">
          <PaletteInput
            query={query}
            loading={false}
            onQueryChange={setQuery}
            inputRef={inputRef}
            trailingActions={
              <Button
                size="xs"
                variant={alwaysOnTop ? "secondary" : "ghost"}
                iconOnly
                title={alwaysOnTopLabel}
                aria-label={alwaysOnTopLabel}
                onClick={handleAlwaysOnTopToggle}
              >
                <span
                  className={[
                    "inline-block leading-none text-[1.35rem]",
                    alwaysOnTop ? "i-noto:pushpin" : "i-noto:round-pushpin",
                  ].join(" ")}
                  aria-hidden="true"
                />
              </Button>
            }
          />

          {launcherError ? <div className="px-4 py-3 text-[13px] text-danger">{launcherError}</div> : null}

          <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-1.5 pt-1">
            {loading ? (
              <div
                className="relative sticky top-0 z-10 mb-2 h-[2px] overflow-hidden rounded bg-border-muted/65"
                role="status"
                aria-live="polite"
              >
                <span className="sr-only">{t("input.searching")}</span>
                <span
                  className="rtool-boot-shimmer-layer absolute inset-y-0 bg-gradient-to-r from-transparent via-shimmer-highlight/26 to-transparent"
                  style={{
                    left: "-30%",
                    width: "30%",
                    animation: "rtool-boot-shimmer 1s linear infinite",
                  }}
                />
              </div>
            ) : null}
            {loading && items.length === 0 ? (
              <div className="space-y-1.5 p-2">
                {[0, 1, 2, 3, 4, 5].map((index) => (
                  <div
                    key={`launcher-skeleton-${index}`}
                    className="relative overflow-hidden rounded-md border border-border-muted/65 bg-surface-soft px-2.5 py-2.5"
                  >
                    <div className="h-3 w-[62%] rounded bg-border-muted/70" />
                    <div className="mt-2 h-2.5 w-[78%] rounded bg-border-muted/55" />
                    <span
                      className="rtool-boot-shimmer-layer absolute inset-y-0 bg-gradient-to-r from-transparent via-shimmer-highlight/26 to-transparent"
                      style={{
                        left: "-45%",
                        width: "45%",
                        animation: "rtool-boot-shimmer 1.2s linear infinite",
                        animationDelay: `${index * 80}ms`,
                      }}
                    />
                  </div>
                ))}
              </div>
            ) : null}

            {!loading && groupedItems.length === 0 ? (
              <div className="p-3 text-[13px] text-text-muted">{t("launcher.noResults")}</div>
            ) : null}

            {!loading || items.length > 0
              ? groupedItems.map((group) => (
                  <div key={group.key} className="mb-1 last:mb-0">
                    <div className="px-2 py-1 text-[11px] uppercase tracking-wide text-text-muted">{group.label}</div>
                    <ul
                      className="m-0 list-none p-0"
                      role="list"
                      aria-label={t("launcher.groupAria", { label: group.label })}
                    >
                      {group.items.map(({ item, index }) => {
                        const isSelected = selectedIndex === index;
                        return (
                          <li
                            key={item.id}
                            className="mb-[3px] last:mb-0"
                            ref={(node) => {
                              if (node) {
                                launcherItemRefs.current.set(item.id, node);
                                return;
                              }
                              launcherItemRefs.current.delete(item.id);
                            }}
                          >
                            <Button
                              unstyled
                              type="button"
                              className={
                                isSelected
                                  ? "w-full rounded-md border border-accent bg-accent-soft px-2.5 py-2.25 text-left transition-colors duration-[140ms] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                                  : "w-full rounded-md border border-transparent px-2.5 py-2.25 text-left transition-colors duration-[140ms] hover:bg-surface-soft focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                              }
                              onMouseEnter={() => setSelectedIndex(index)}
                              onFocus={() => setSelectedIndex(index)}
                              onClick={() => {
                                setSelectedIndex(index);
                                void executeSelected().then((result) => {
                                  if (result?.ok) {
                                    void appWindow.hide();
                                  }
                                });
                              }}
                              aria-current={isSelected ? "true" : undefined}
                            >
                              <div className="flex items-start gap-2.5">
                                <LauncherItemIcon item={item} />
                                <div className="min-w-0">
                                  <div className="truncate text-sm font-semibold text-text-primary">
                                    {renderHighlightedText(item.title, query)}
                                  </div>
                                  <div className="mt-[3px] truncate text-xs text-text-secondary">
                                    {renderHighlightedText(item.subtitle, query)}
                                  </div>
                                  {item.shortcut ? (
                                    <div className="mt-1 text-[11px] text-text-muted">
                                      {t("launcher.shortcut", { value: item.shortcut })}
                                    </div>
                                  ) : null}
                                </div>
                              </div>
                            </Button>
                          </li>
                        );
                      })}
                    </ul>
                  </div>
                ))
              : null}
          </div>

          <footer className="flex gap-4 border-t border-border-muted/85 px-3 py-2 text-[11px] text-text-muted">
            <span>{t("launcher.footer.select")}</span>
            <span>{t("launcher.footer.open")}</span>
            <span>{t("launcher.footer.close")}</span>
          </footer>
        </div>

        <div className="hidden min-w-[280px] flex-[0.95] md:block">
          <PalettePreview selectedItem={selectedItem} context="launcher" />
        </div>
      </section>
      {bootMounted ? <BootOverlay variant="launcher" visible={bootVisible} /> : null}
    </div>
  );
}

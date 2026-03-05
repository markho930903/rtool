import { listen } from "@tauri-apps/api/event";
import { currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppEntityIcon } from "@/components/icons/AppEntityIcon";
import { BootOverlay, SkeletonComposer, type SkeletonItemSpec, useBootState } from "@/components/loading";
import type { PaletteItem } from "@/components/palette/types";
import { Button, Input } from "@/components/ui";
import { useAsyncEffect } from "@/hooks/useAsyncEffect";
import { useWindowFocusAutoHide } from "@/hooks/window/useWindowFocusAutoHide";
import { useWindowLayoutPersistence } from "@/hooks/window/useWindowLayoutPersistence";
import { useLocaleStore } from "@/i18n/store";
import { useLauncherStore } from "@/stores/launcher.store";

const LAUNCHER_WINDOW_LABEL = "launcher";
const WINDOW_LAYOUT_KEY = "launcher-window-layout";
const LAUNCHER_GRID_CARD_MIN_WIDTH = 90;
const LAUNCHER_GRID_GAP = 4;
const LAUNCHER_GRID_MAX_COLUMNS = 6;

type LauncherTopTab = "all" | "application" | "file" | "builtin";
type LauncherSectionKey = "builtin" | "application" | "file" | "other";

interface FlatLauncherItem {
  item: PaletteItem;
  absoluteIndex: number;
  section: LauncherSectionKey;
  flatIndex: number;
}

interface LauncherSection {
  key: LauncherSectionKey;
  label: string;
  showHeader: boolean;
  items: FlatLauncherItem[];
}

const LAUNCHER_LIST_SKELETON_ITEMS: SkeletonItemSpec[] = Array.from({ length: 8 }, (_, index) => ({
  key: `launcher-skeleton-${index}`,
  body: [
    {
      nodes: [{ widthClassName: "w-full", heightClassName: "h-20", className: "bg-border-muted/55 rounded-lg" }],
    },
  ],
  shimmerDelayMs: index * 70,
}));

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

interface HighlightContext {
  matcher: RegExp | null;
  tokenSet: Set<string>;
}

function createHighlightContext(query: string): HighlightContext {
  const tokens = query
    .trim()
    .split(/\s+/)
    .map((token) => token.trim())
    .filter(Boolean);

  if (tokens.length === 0) {
    return {
      matcher: null,
      tokenSet: new Set(),
    };
  }

  return {
    matcher: new RegExp(`(${tokens.map(escapeRegExp).join("|")})`, "ig"),
    tokenSet: new Set(tokens.map((token) => token.toLowerCase())),
  };
}

function renderHighlightedText(text: string, context: HighlightContext): ReactNode {
  if (!context.matcher || context.tokenSet.size === 0) {
    return text;
  }

  const parts = text.split(context.matcher);

  return parts.map((part, index) => {
    const lower = part.toLowerCase();
    if (context.tokenSet.has(lower)) {
      return (
        <mark key={`${lower}-${index}`} className="rounded bg-accent-soft px-[1px] font-semibold text-accent">
          {part}
        </mark>
      );
    }
    return <span key={`${lower}-${index}`}>{part}</span>;
  });
}

function resolveSectionByCategory(category: string | undefined): LauncherSectionKey {
  if (category === "builtin") {
    return "builtin";
  }

  if (category === "application") {
    return "application";
  }

  if (category === "file" || category === "directory") {
    return "file";
  }

  return "other";
}

function nextTopTab(current: LauncherTopTab, step: 1 | -1): LauncherTopTab {
  const ordered: LauncherTopTab[] = ["all", "application", "file", "builtin"];
  const currentIndex = ordered.indexOf(current);
  const safeIndex = currentIndex < 0 ? 0 : currentIndex;
  const nextIndex = (safeIndex + step + ordered.length) % ordered.length;
  return ordered[nextIndex];
}

function LauncherItemIcon({ item }: { item: PaletteItem }) {
  return (
    <AppEntityIcon
      iconKind={item.iconKind}
      iconValue={item.iconValue}
      fallbackIcon="i-noto:card-index-dividers"
      imgClassName="h-[52%] w-[52%] shrink-0 rounded-xl object-cover"
      iconClassName="h-[52%] w-[52%] shrink-0 text-[1.9rem] text-text-muted"
    />
  );
}

export default function LauncherWindowPage() {
  const { t } = useTranslation("palette");
  const appWindow = useMemo(() => getCurrentWindow(), []);
  const inputRef = useRef<HTMLInputElement>(null);
  const launcherItemRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const gridRef = useRef<HTMLDivElement>(null);
  const [openCycle, setOpenCycle] = useState(1);
  const [searchSeed, setSearchSeed] = useState(0);
  const [hasSearchedOnce, setHasSearchedOnce] = useState(false);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const [activeTab, setActiveTab] = useState<LauncherTopTab>("all");
  const [selectedVisibleIndex, setSelectedVisibleIndex] = useState(0);
  const [gridColumnCount, setGridColumnCount] = useState(1);
  const alwaysOnTopRef = useRef(false);

  const query = useLauncherStore((state) => state.query);
  const items = useLauncherStore((state) => state.items);
  const loading = useLauncherStore((state) => state.loading);
  const launcherError = useLauncherStore((state) => state.error);
  const reset = useLauncherStore((state) => state.reset);
  const search = useLauncherStore((state) => state.search);
  const setStoreSelectedIndex = useLauncherStore((state) => state.setSelectedIndex);
  const executeSelected = useLauncherStore((state) => state.executeSelected);
  const setQuery = useLauncherStore((state) => state.setQuery);
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

  const sectionLabelMap = useMemo(
    () => ({
      builtin: t("launcher.section.builtin"),
      application: t("launcher.section.application"),
      file: t("launcher.section.file"),
      other: t("launcher.section.other"),
    }),
    [t],
  );

  const tabs = useMemo(
    () => [
      { key: "all" as const, label: t("launcher.tab.all") },
      { key: "application" as const, label: t("launcher.tab.application") },
      { key: "file" as const, label: t("launcher.tab.file") },
      { key: "builtin" as const, label: t("launcher.tab.builtin") },
    ],
    [t],
  );

  const { sections, flatItems } = useMemo(() => {
    const base = items.map((item, absoluteIndex) => ({
      item,
      absoluteIndex,
      section: resolveSectionByCategory(item.category),
    }));

    const filtered =
      activeTab === "all"
        ? base
        : base.filter((entry) => {
            if (activeTab === "application") {
              return entry.section === "application";
            }

            if (activeTab === "file") {
              return entry.section === "file";
            }

            return entry.section === "builtin";
          });

    const pushWithFlatIndex = (entries: typeof filtered, offset: number) =>
      entries.map((entry, localIndex) => ({
        ...entry,
        flatIndex: offset + localIndex,
      }));

    if (activeTab !== "all") {
      const keyed = pushWithFlatIndex(filtered, 0);
      const sectionKey: LauncherSectionKey =
        activeTab === "application" ? "application" : activeTab === "file" ? "file" : "builtin";

      return {
        sections: [
          {
            key: sectionKey,
            label: sectionLabelMap[sectionKey],
            showHeader: false,
            items: keyed,
          },
        ],
        flatItems: keyed,
      };
    }

    let cursor = 0;
    const orderedSections: LauncherSectionKey[] = ["builtin", "application", "file", "other"];
    const builtSections: LauncherSection[] = [];

    for (const key of orderedSections) {
      const chunk = filtered.filter((entry) => entry.section === key);
      if (chunk.length === 0) {
        continue;
      }

      const keyedChunk = pushWithFlatIndex(chunk, cursor);
      cursor += keyedChunk.length;

      builtSections.push({
        key,
        label: sectionLabelMap[key],
        showHeader: true,
        items: keyedChunk,
      });
    }

    return {
      sections: builtSections,
      flatItems: builtSections.flatMap((section) => section.items),
    };
  }, [activeTab, items, sectionLabelMap]);

  const selectedEntry = flatItems[selectedVisibleIndex] ?? null;
  const selectedItem = selectedEntry?.item ?? null;
  const highlightContext = useMemo(() => createHighlightContext(query), [query]);
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

    const current = selectedEntry;
    if (!current) {
      return;
    }

    setStoreSelectedIndex(current.absoluteIndex);
  }, [enabled, selectedEntry, setStoreSelectedIndex]);

  useEffect(() => {
    setSelectedVisibleIndex((current) => {
      if (flatItems.length === 0) {
        return 0;
      }
      return Math.min(current, flatItems.length - 1);
    });
  }, [flatItems]);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const host = gridRef.current;
    if (!host) {
      return;
    }

    const updateColumns = () => {
      const width = host.clientWidth;
      const rawColumnCount = Math.floor((width + LAUNCHER_GRID_GAP) / (LAUNCHER_GRID_CARD_MIN_WIDTH + LAUNCHER_GRID_GAP));
      const columnCount = Math.max(1, Math.min(LAUNCHER_GRID_MAX_COLUMNS, rawColumnCount));
      setGridColumnCount(columnCount);
    };

    updateColumns();

    const observer = new ResizeObserver(updateColumns);
    observer.observe(host);

    return () => {
      observer.disconnect();
    };
  }, [enabled]);

  const executeCurrentSelection = useCallback(() => {
    const current = flatItems[selectedVisibleIndex];
    if (!current) {
      return;
    }

    setStoreSelectedIndex(current.absoluteIndex);
    void executeSelected().then((result) => {
      if (result?.ok) {
        void appWindow.hide();
      }
    });
  }, [appWindow, executeSelected, selectedVisibleIndex, setStoreSelectedIndex, flatItems]);

  const moveGridSelection = useCallback(
    (delta: number) => {
      if (flatItems.length === 0) {
        return;
      }

      setSelectedVisibleIndex((current) => {
        const next = current + delta;
        if (next < 0) {
          return 0;
        }

        if (next >= flatItems.length) {
          return flatItems.length - 1;
        }

        return next;
      });
    },
    [flatItems.length],
  );

  useAsyncEffect(
    async ({ stack }) => {
      if (!enabled) {
        return;
      }

      const unlistenOpened = await listen("rtool://launcher/opened", () => {
        cancelScheduledHide();
        syncAlwaysOnTopState();

        setOpenCycle((value) => value + 1);
        setSearchSeed((value) => value + 1);
        setHasSearchedOnce(false);
        setActiveTab("all");
        setSelectedVisibleIndex(0);
        reset();

        window.setTimeout(() => {
          inputRef.current?.focus();
        }, 40);
      });
      stack.add(unlistenOpened, "opened");

      const onKeyDown = (event: KeyboardEvent) => {
        if (event.key === "Escape") {
          event.preventDefault();
          void appWindow.hide();
          return;
        }

        if ((event.metaKey || event.ctrlKey) && event.key === "ArrowLeft") {
          event.preventDefault();
          setActiveTab((current) => nextTopTab(current, -1));
          setSelectedVisibleIndex(0);
          return;
        }

        if ((event.metaKey || event.ctrlKey) && event.key === "ArrowRight") {
          event.preventDefault();
          setActiveTab((current) => nextTopTab(current, 1));
          setSelectedVisibleIndex(0);
          return;
        }

        if (event.key === "ArrowLeft") {
          event.preventDefault();
          moveGridSelection(-1);
          return;
        }

        if (event.key === "ArrowRight") {
          event.preventDefault();
          moveGridSelection(1);
          return;
        }

        if (event.key === "ArrowDown") {
          event.preventDefault();
          moveGridSelection(gridColumnCount);
          return;
        }

        if (event.key === "ArrowUp") {
          event.preventDefault();
          moveGridSelection(-gridColumnCount);
          return;
        }

        if (event.key === "Enter") {
          if (event.isComposing || event.keyCode === 229) {
            return;
          }
          event.preventDefault();
          executeCurrentSelection();
        }
      };

      window.addEventListener("keydown", onKeyDown);
      stack.add(() => {
        window.removeEventListener("keydown", onKeyDown);
      }, "remove-keydown-listener");
    },
    [
      appWindow,
      cancelScheduledHide,
      enabled,
      executeCurrentSelection,
      gridColumnCount,
      moveGridSelection,
      reset,
      syncAlwaysOnTopState,
    ],
    {
      scope: "launcher-window",
      onError: (error) => {
        if (import.meta.env.DEV) {
          console.warn("[launcher-window] event setup failed", error);
        }
      },
    },
  );

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

  const activeTabLabel = tabs.find((tab) => tab.key === activeTab)?.label ?? t("launcher.tab.all");
  const alwaysOnTopLabel = alwaysOnTop ? t("launcher.pinWindowOff") : t("launcher.pinWindowOn");

  return (
    <div className="relative h-full w-full overflow-hidden bg-transparent p-0">
      <section className="rtool-glass-sheen-clip flex h-full w-full flex-col overflow-hidden bg-layout-titlebar shadow-surface backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]">
        <header className="border-b border-layout-divider px-3 pb-2 pt-2.5">
          <div className="flex items-center gap-2 rounded-xl border border-border-glass bg-surface-glass-soft px-2.5 py-1.5">
            <span className="i-noto:magnifying-glass-tilted-right text-[1.05rem] text-text-muted" aria-hidden="true" />
            <Input
              id="launcher-query-input"
              variant="palette"
              ref={inputRef}
              name="launcherQuery"
              autoComplete="off"
              spellCheck={false}
              aria-label={t("input.aria")}
              value={query}
              onChange={(event) => {
                setQuery(event.currentTarget.value);
                setSelectedVisibleIndex(0);
              }}
              placeholder={t("input.placeholder")}
              className="text-sm"
            />
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
                  "inline-block leading-none text-[1.25rem]",
                  alwaysOnTop ? "i-noto:pushpin" : "i-noto:round-pushpin",
                ].join(" ")}
                aria-hidden="true"
              />
            </Button>
          </div>

          <div className="mt-2.5 grid grid-cols-4 gap-2" role="tablist" aria-label={t("launcher.tab.aria")}>
            {tabs.map((tab) => {
              const active = tab.key === activeTab;
              return (
                <Button
                  key={tab.key}
                  unstyled
                  className={
                    active
                      ? "rounded-lg border border-border-glass-strong bg-surface-glass-soft px-2 py-1.5 text-center text-xs font-semibold text-text-primary shadow-inset-soft"
                      : "rounded-lg border border-border-glass bg-transparent px-2 py-1.5 text-center text-xs text-text-secondary hover:bg-surface-soft/60 hover:text-text-primary"
                  }
                  role="tab"
                  aria-selected={active}
                  onClick={() => {
                    setActiveTab(tab.key);
                    setSelectedVisibleIndex(0);
                  }}
                >
                  {tab.label}
                </Button>
              );
            })}
          </div>
        </header>

        {launcherError ? <div className="px-4 py-2 text-[13px] text-danger">{launcherError}</div> : null}

        <div ref={gridRef} className="min-h-0 flex-1 overflow-y-auto px-2 pb-1 pt-1">
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
            <SkeletonComposer
              items={LAUNCHER_LIST_SKELETON_ITEMS}
              className="p-0"
              gapClassName="grid grid-cols-4 gap-2"
              itemSurfaceClassName="bg-surface-soft"
            />
          ) : null}

          {!loading && flatItems.length === 0 ? (
            <div className="rounded-lg border border-border-glass bg-surface-soft/40 px-3 py-3 text-[13px] text-text-muted">
              {activeTab === "all" ? t("launcher.noResults") : t("launcher.noResultsInGroup", { group: activeTabLabel })}
            </div>
          ) : null}

          {!loading || items.length > 0 ? (
            <div aria-label={t("launcher.gridAria", { group: activeTabLabel })}>
              {sections.map((section) => (
                <section key={section.key} className="mb-1 last:mb-0">
                  {section.showHeader ? (
                    <h3 className="mb-0 px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-muted">{section.label}</h3>
                  ) : null}
                  <div
                    className="grid"
                    style={{
                      gap: `${LAUNCHER_GRID_GAP}px`,
                      gridTemplateColumns: `repeat(${gridColumnCount}, minmax(0, 1fr))`,
                    }}
                  >
                    {section.items.map((entry) => {
                      const isSelected = selectedVisibleIndex === entry.flatIndex;
                      return (
                        <Button
                          key={entry.item.id}
                          unstyled
                          ref={(node) => {
                            if (node instanceof HTMLButtonElement) {
                              launcherItemRefs.current.set(entry.item.id, node);
                              return;
                            }
                            launcherItemRefs.current.delete(entry.item.id);
                          }}
                          className={
                            isSelected
                              ? "group flex aspect-square w-full flex-col items-center justify-center gap-0.5 rounded-xl px-0.5 py-0.5 text-center transition-colors duration-[140ms] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                              : "group flex aspect-square w-full flex-col items-center justify-center gap-0.5 rounded-xl px-0.5 py-0.5 text-center transition-colors duration-[140ms] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                          }
                          onFocus={() => {
                            if (selectedVisibleIndex !== entry.flatIndex) {
                              setSelectedVisibleIndex(entry.flatIndex);
                            }
                          }}
                          onClick={() => {
                            if (selectedVisibleIndex !== entry.flatIndex) {
                              setSelectedVisibleIndex(entry.flatIndex);
                            }
                            setStoreSelectedIndex(entry.absoluteIndex);
                            void executeSelected().then((result) => {
                              if (result?.ok) {
                                void appWindow.hide();
                              }
                            });
                          }}
                          aria-current={isSelected ? "true" : undefined}
                        >
                          <LauncherItemIcon item={entry.item} />
                          <div className="w-full truncate whitespace-nowrap text-xs font-medium leading-4 text-text-primary">
                            {renderHighlightedText(entry.item.title, highlightContext)}
                          </div>
                        </Button>
                      );
                    })}
                  </div>
                </section>
              ))}
            </div>
          ) : null}
        </div>

        <footer className="flex gap-4 border-t border-border-glass px-3 py-2 text-[11px] text-text-muted">
          <span>{t("launcher.footer.move")}</span>
          <span>{t("launcher.footer.switchCategory")}</span>
          <span>{t("launcher.footer.open")}</span>
          <span>{t("launcher.footer.close")}</span>
        </footer>
      </section>
      {bootMounted ? <BootOverlay variant="launcher" visible={bootVisible} /> : null}
    </div>
  );
}

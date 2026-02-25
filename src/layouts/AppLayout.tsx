import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { NavLink, Outlet, useLocation } from "react-router";

import { BootOverlay, useBootState } from "@/components/loading";
import { Button } from "@/components/ui";
import { useLocaleStore } from "@/i18n/store";
import { useLayoutStore } from "@/layouts/layout.store";
import {
  getMainMenuRouteConfig,
  isRouteActiveById,
  resolveActiveMainMenuByPath,
  resolveWindowModeByPath,
} from "@/routers/routes.config";
import { useAppStore } from "@/stores/app.store";
import { useThemeStore } from "@/theme/store";
import type { ThemePreference } from "@/theme/types";

const THEME_ICON_MAP: Record<ThemePreference, string> = {
  system: "i-noto:desktop-computer",
  dark: "i-noto:crescent-moon",
  light: "i-noto:sun",
};

const NAV_ITEMS = getMainMenuRouteConfig();

const SIDEBAR_ITEM_BASE_CLASS =
  "ui-glass-hover relative inline-flex h-14 w-14 select-none flex-col items-center justify-center gap-0.5 overflow-hidden rounded-3 border border-transparent text-text-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/45 [&_.sidebar-item-icon]:transition-transform [&_.sidebar-item-icon]:duration-200 [&_.sidebar-item-label]:transition-colors [&_.sidebar-item-label]:duration-200";

const SIDEBAR_ITEM_ACTIVE_CLASS =
  "border-border-glass-strong bg-surface-glass-soft text-text-primary shadow-inset-soft [&_.sidebar-item-icon]:-translate-y-[0.5px] [&_.sidebar-item-label]:text-text-primary";

const SIDEBAR_ITEM_IDLE_CLASS =
  "text-text-secondary hover:-translate-y-[1px] hover:text-text-primary hover:[&_.sidebar-item-icon]:-translate-y-[1px] hover:[&_.sidebar-item-label]:text-text-primary active:translate-y-0 active:scale-[0.98]";

const TITLEBAR_ICON_BUTTON_CLASS =
  "ui-glass-hover inline-flex h-9 w-9 select-none items-center justify-center rounded-3 border border-transparent text-text-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/45 active:scale-[0.97]";

const TITLEBAR_MENU_TRIGGER_BUTTON_CLASS =
  "ui-glass-hover inline-flex h-9 max-w-[12rem] select-none items-center gap-1.5 rounded-3 border border-transparent px-2.5 text-text-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/45 active:scale-[0.97]";

const TITLEBAR_MENU_ITEM_BASE_CLASS =
  "ui-glass-hover flex items-center gap-2.5 rounded-md px-2.5 py-2.25 text-left text-text-secondary transition-colors duration-[140ms]";

const TITLEBAR_MENU_ITEM_ACTIVE_CLASS = "bg-surface-glass-soft text-text-primary shadow-inset-soft";

const TITLEBAR_MENU_ITEM_IDLE_CLASS = "hover:text-text-primary";

const MENU_MIN_WIDTH = 220;
const MENU_MAX_WIDTH = 360;
const MENU_MIN_HEIGHT = 140;
const MENU_VIEWPORT_PADDING = 8;
const MENU_GAP = 8;

function getNextTheme(preference: ThemePreference): ThemePreference {
  if (preference === "system") {
    return "dark";
  }

  if (preference === "dark") {
    return "light";
  }

  return "system";
}

function MainContent({ isFullHeightRoute }: { isFullHeightRoute: boolean }) {
  return (
    <div
      className={
        isFullHeightRoute
          ? "relative z-10 min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-hidden"
          : "relative z-10 min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto"
      }
    >
      <main className="h-full w-full p-4">
        <Outlet />
      </main>
    </div>
  );
}

function ThemeToggleButton() {
  const { t } = useTranslation("layout");
  const preference = useThemeStore((state) => state.preference);
  const setPreference = useThemeStore((state) => state.setPreference);
  const currentLabel = t(`theme.${preference}`);
  const currentIcon = THEME_ICON_MAP[preference];
  const nextPreference = getNextTheme(preference);
  const nextLabel = t(`theme.${nextPreference}`);
  const title = t("theme.title", { current: currentLabel, next: nextLabel });

  return (
    <Button
      unstyled
      type="button"
      className={[SIDEBAR_ITEM_BASE_CLASS, SIDEBAR_ITEM_IDLE_CLASS].join(" ")}
      onClick={() => void setPreference(nextPreference)}
      title={title}
      aria-label={title}
    >
      <span className={`sidebar-item-icon btn-icon text-[1.15rem] ${currentIcon}`} aria-hidden="true" />
      <span className="sidebar-item-label whitespace-nowrap text-[0.62rem] leading-none font-semibold tracking-[0.005em]">
        {t("theme.button")}
      </span>
    </Button>
  );
}

function ThemeToggleIconButton() {
  const { t } = useTranslation("layout");
  const preference = useThemeStore((state) => state.preference);
  const setPreference = useThemeStore((state) => state.setPreference);
  const currentLabel = t(`theme.${preference}`);
  const currentIcon = THEME_ICON_MAP[preference];
  const nextPreference = getNextTheme(preference);
  const nextLabel = t(`theme.${nextPreference}`);
  const title = t("theme.title", { current: currentLabel, next: nextLabel });

  return (
    <Button
      unstyled
      type="button"
      className={TITLEBAR_ICON_BUTTON_CLASS}
      onClick={() => void setPreference(nextPreference)}
      title={title}
      aria-label={title}
    >
      <span className={`btn-icon text-[1.05rem] ${currentIcon}`} aria-hidden="true" />
    </Button>
  );
}

function SideBar() {
  const { t } = useTranslation("layout");

  return (
    <aside className="rtool-glass-sheen-clip z-20 flex h-full w-[84px] shrink-0 flex-col items-center overflow-hidden border-r border-border-glass bg-surface-glass-soft shadow-inset-soft">
      <div className="h-16 w-full shrink-0" data-tauri-drag-region />
      <nav className="mt-1 flex flex-1 flex-col items-center gap-2 py-2" aria-label={t("nav.mainAria")}>
        {NAV_ITEMS.map((item) => {
          const label = t(item.labelKey);
          return (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                [SIDEBAR_ITEM_BASE_CLASS, isActive ? SIDEBAR_ITEM_ACTIVE_CLASS : SIDEBAR_ITEM_IDLE_CLASS].join(" ")
              }
              title={label}
              aria-label={label}
            >
              <span className={`sidebar-item-icon btn-icon text-[1.15rem] ${item.icon}`} aria-hidden="true" />
              <span className="sidebar-item-label whitespace-nowrap text-[0.62rem] leading-none font-semibold tracking-[0.005em]">
                {label}
              </span>
            </NavLink>
          );
        })}
      </nav>

      <div className="mb-4 mt-auto flex flex-col items-center gap-2.5">
        <ThemeToggleButton />
      </div>
    </aside>
  );
}

export default function AppLayout() {
  const { t } = useTranslation("layout");
  const location = useLocation();
  const layoutPreference = useLayoutStore((state) => state.preference);
  const layoutInitialized = useLayoutStore((state) => state.initialized);
  const localeInitialized = useLocaleStore((state) => state.initialized);
  const themeInitialized = useThemeStore((state) => state.initialized);
  const setWindowMode = useAppStore((state) => state.setWindowMode);
  const isSettingsRoute = useMemo(() => isRouteActiveById("settings", location.pathname), [location.pathname]);
  const isAppManagerRoute = useMemo(() => isRouteActiveById("app_manager", location.pathname), [location.pathname]);
  const isFullHeightRoute = isSettingsRoute || isAppManagerRoute;
  const currentNavItem = useMemo(() => resolveActiveMainMenuByPath(location.pathname), [location.pathname]);
  const currentLabel = t(currentNavItem.labelKey);
  const switchMenuLabel = t("titlebar.switchMenu", { current: currentLabel });

  const [menuOpen, setMenuOpen] = useState(false);
  const [menuOpenedByKeyboard, setMenuOpenedByKeyboard] = useState(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const [menuPosition, setMenuPosition] = useState({
    top: 0,
    left: 0,
    maxHeight: 320,
  });
  const ready = themeInitialized && localeInitialized && layoutInitialized;
  const { mounted: bootMounted, visible: bootVisible } = useBootState({
    cycleKey: 1,
    ready,
    delayMs: 160,
    minVisibleMs: 180,
    maxWaitMs: 1200,
    exitMs: 160,
  });

  useEffect(() => {
    setWindowMode(resolveWindowModeByPath(location.pathname));
  }, [location.pathname, setWindowMode]);

  useEffect(() => {
    setMenuOpen(false);
    setMenuOpenedByKeyboard(false);
  }, [location.pathname]);

  const updateMenuPosition = useCallback((anchorRect: DOMRect) => {
    const measuredWidth = menuRef.current?.offsetWidth ?? MENU_MIN_WIDTH;
    const measuredHeight = menuRef.current?.offsetHeight ?? 0;
    const viewportMaxWidth = Math.max(180, window.innerWidth - MENU_VIEWPORT_PADDING * 2);
    const allowedMaxWidth = Math.min(MENU_MAX_WIDTH, viewportMaxWidth);
    const popupWidth = Math.min(Math.max(MENU_MIN_WIDTH, measuredWidth, anchorRect.width), allowedMaxWidth);
    const maxLeft = Math.max(MENU_VIEWPORT_PADDING, window.innerWidth - popupWidth - MENU_VIEWPORT_PADDING);
    const nextLeft = Math.min(Math.max(anchorRect.left, MENU_VIEWPORT_PADDING), maxLeft);

    const spaceBelow = window.innerHeight - anchorRect.bottom - MENU_GAP - MENU_VIEWPORT_PADDING;
    const spaceAbove = anchorRect.top - MENU_GAP - MENU_VIEWPORT_PADDING;
    const renderBelow = spaceBelow >= 180 || spaceBelow >= spaceAbove;
    const nextMaxHeight = Math.max(MENU_MIN_HEIGHT, renderBelow ? spaceBelow : spaceAbove);

    let nextTop = renderBelow ? anchorRect.bottom + MENU_GAP : anchorRect.top - MENU_GAP - measuredHeight;
    if (measuredHeight > 0) {
      const maxTop = Math.max(MENU_VIEWPORT_PADDING, window.innerHeight - measuredHeight - MENU_VIEWPORT_PADDING);
      nextTop = Math.min(Math.max(nextTop, MENU_VIEWPORT_PADDING), maxTop);
    } else {
      nextTop = Math.max(nextTop, MENU_VIEWPORT_PADDING);
    }

    setMenuPosition({
      top: nextTop,
      left: nextLeft,
      maxHeight: nextMaxHeight,
    });
  }, []);

  const openMenu = useCallback(
    (anchorElement: HTMLElement, viaKeyboard: boolean) => {
      updateMenuPosition(anchorElement.getBoundingClientRect());
      setMenuOpenedByKeyboard(viaKeyboard);
      setMenuOpen(true);
    },
    [updateMenuPosition],
  );

  const closeMenu = useCallback(() => {
    setMenuOpen(false);
    setMenuOpenedByKeyboard(false);
    triggerRef.current?.focus();
  }, []);

  useLayoutEffect(() => {
    if (!menuOpen) {
      return;
    }

    const syncMenuPosition = () => {
      const trigger = triggerRef.current;
      if (!trigger) {
        return;
      }
      updateMenuPosition(trigger.getBoundingClientRect());
    };

    const rafId = window.requestAnimationFrame(syncMenuPosition);
    window.addEventListener("resize", syncMenuPosition);
    window.addEventListener("scroll", syncMenuPosition, true);

    return () => {
      window.cancelAnimationFrame(rafId);
      window.removeEventListener("resize", syncMenuPosition);
      window.removeEventListener("scroll", syncMenuPosition, true);
    };
  }, [menuOpen, updateMenuPosition]);

  useEffect(() => {
    if (!menuOpen) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        closeMenu();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [closeMenu, menuOpen]);

  useEffect(() => {
    if (!menuOpen || !menuOpenedByKeyboard) {
      return;
    }

    const rafId = window.requestAnimationFrame(() => {
      const firstItem = menuRef.current?.querySelector<HTMLElement>('[role="menuitem"]');
      firstItem?.focus();
    });

    return () => window.cancelAnimationFrame(rafId);
  }, [menuOpen, menuOpenedByKeyboard]);

  const menuPopup = menuOpen
    ? createPortal(
        <div className="fixed inset-0 z-[70]" onPointerDown={closeMenu}>
          <div
            ref={menuRef}
            className="fixed z-[80] overflow-hidden rounded-md border border-border-glass bg-surface-glass-strong p-1.5 shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]"
            style={{
              top: `${menuPosition.top}px`,
              left: `${menuPosition.left}px`,
              minWidth: `${MENU_MIN_WIDTH}px`,
              maxWidth: `calc(100vw - ${MENU_VIEWPORT_PADDING * 2}px)`,
            }}
            role="menu"
            aria-label={t("titlebar.menuAria")}
            onPointerDown={(event) => event.stopPropagation()}
            onKeyDown={(event) => {
              if (event.key === "Tab") {
                closeMenu();
                return;
              }
              if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
                return;
              }

              const items = Array.from(menuRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? []);
              if (items.length === 0) {
                return;
              }

              event.preventDefault();
              const activeIndex = items.findIndex((item) => item === document.activeElement);
              if (event.key === "Home") {
                items[0]?.focus();
                return;
              }
              if (event.key === "End") {
                items[items.length - 1]?.focus();
                return;
              }

              const direction = event.key === "ArrowUp" ? -1 : 1;
              const current = activeIndex < 0 ? 0 : activeIndex;
              const nextIndex = (current + direction + items.length) % items.length;
              items[nextIndex]?.focus();
            }}
          >
            <div className="overflow-y-auto" style={{ maxHeight: `${menuPosition.maxHeight}px` }}>
              <nav className="flex w-full flex-col items-stretch gap-1" aria-label={t("titlebar.menuAria")}>
                {NAV_ITEMS.map((item) => {
                  const label = t(item.labelKey);
                  return (
                    <NavLink
                      key={item.to}
                      to={item.to}
                      end={item.end}
                      role="menuitem"
                      title={label}
                      aria-label={label}
                      onClick={() => {
                        setMenuOpen(false);
                        setMenuOpenedByKeyboard(false);
                      }}
                      className={({ isActive }) => {
                        const focusClass =
                          "w-full focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent/35";
                        return [
                          TITLEBAR_MENU_ITEM_BASE_CLASS,
                          focusClass,
                          isActive ? TITLEBAR_MENU_ITEM_ACTIVE_CLASS : TITLEBAR_MENU_ITEM_IDLE_CLASS,
                        ].join(" ");
                      }}
                    >
                      <span className={`btn-icon shrink-0 text-[1.1rem] ${item.icon}`} aria-hidden="true" />
                      <span className="truncate text-xs font-medium">{label}</span>
                    </NavLink>
                  );
                })}
              </nav>
            </div>
          </div>
        </div>,
        document.body,
      )
    : null;

  if (layoutPreference === "sidebar") {
    return (
      <div className="relative h-screen w-screen overflow-hidden rounded-md bg-transparent p-0 text-text-primary">
        <section className="rtool-glass-sheen-clip relative z-10 flex h-full w-full overflow-hidden rounded-md border border-border-glass bg-surface-glass-strong shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]">
          <div aria-hidden className="rtool-glass-atmosphere" />
          <SideBar />
          <MainContent isFullHeightRoute={isFullHeightRoute} />
        </section>
        {bootMounted ? <BootOverlay variant="main" visible={bootVisible} /> : null}
      </div>
    );
  }

  return (
    <div className="relative h-screen w-screen overflow-hidden rounded-md bg-transparent p-0 text-text-primary">
      <section className="rtool-glass-sheen-clip relative z-10 flex h-full w-full flex-col overflow-hidden rounded-md border border-border-glass bg-surface-glass-strong shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]">
        <div aria-hidden className="rtool-glass-atmosphere" />
        <header className="rtool-glass-sheen-open relative z-20 flex h-12 shrink-0 items-center border-b border-border-glass bg-surface-glass-soft shadow-inset-soft">
          <div className="relative flex h-full items-center pl-[4.75rem] pr-2">
            <div className="relative">
              <Button
                unstyled
                ref={triggerRef}
                type="button"
                className={TITLEBAR_MENU_TRIGGER_BUTTON_CLASS}
                title={switchMenuLabel}
                aria-label={switchMenuLabel}
                aria-expanded={menuOpen}
                aria-haspopup="menu"
                onClick={(event) => {
                  if (menuOpen) {
                    closeMenu();
                    return;
                  }
                  openMenu(event.currentTarget, false);
                }}
                onKeyDown={(event) => {
                  if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    openMenu(event.currentTarget, true);
                  }
                }}
              >
                <span className={`btn-icon shrink-0 text-[1.05rem] ${currentNavItem.icon}`} aria-hidden="true" />
                <span className="truncate text-xs font-medium">{currentLabel}</span>
              </Button>
            </div>
          </div>

          <div className="h-full flex-1" data-tauri-drag-region />

          <div className="flex h-full items-center pr-3">
            <ThemeToggleIconButton />
          </div>
        </header>

        <MainContent isFullHeightRoute={isFullHeightRoute} />
      </section>
      {menuPopup}
      {bootMounted ? <BootOverlay variant="main" visible={bootVisible} /> : null}
    </div>
  );
}

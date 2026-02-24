import { useEffect, useMemo, useRef, useState } from "react";
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
  "relative inline-flex h-14 w-14 select-none flex-col items-center justify-center gap-0.5 overflow-hidden rounded-3 text-text-secondary transition-[background-color,color,box-shadow,transform] duration-200 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/45 [&_.sidebar-item-icon]:transition-transform [&_.sidebar-item-icon]:duration-200 [&_.sidebar-item-label]:transition-colors [&_.sidebar-item-label]:duration-200";

const SIDEBAR_ITEM_ACTIVE_CLASS =
  "bg-sidebar-item-active text-text-primary shadow-sidebar-item-active [&_.sidebar-item-icon]:-translate-y-[0.5px] [&_.sidebar-item-label]:text-text-primary";

const SIDEBAR_ITEM_IDLE_CLASS =
  "text-text-secondary hover:-translate-y-[1px] hover:bg-sidebar-item-hover hover:text-text-primary hover:shadow-sidebar-item-hover hover:[&_.sidebar-item-icon]:-translate-y-[1px] hover:[&_.sidebar-item-label]:text-text-primary active:translate-y-0 active:scale-[0.98]";

const TITLEBAR_ICON_BUTTON_CLASS =
  "inline-flex h-9 w-9 select-none items-center justify-center rounded-3 text-text-secondary transition-[background-color,color,transform] duration-200 ease-out hover:bg-sidebar-item-hover hover:text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/45 active:scale-[0.97]";

const TITLEBAR_MENU_ITEM_BASE_CLASS =
  "flex items-center gap-2.5 rounded-md border px-2.5 py-2 text-text-secondary transition-[border-color,background-color,color] duration-[140ms]";

const TITLEBAR_MENU_ITEM_ACTIVE_CLASS = "border-accent/45 bg-accent-soft text-text-primary shadow-inset-soft";

const TITLEBAR_MENU_ITEM_IDLE_CLASS =
  "border-transparent hover:border-border-muted/70 hover:bg-surface-soft hover:text-text-primary";

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
          ? "min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-hidden"
          : "min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto"
      }
    >
      <main className={isFullHeightRoute ? "h-full w-full" : "mx-auto w-full max-w-6xl px-4 py-5 md:px-5"}>
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
      onClick={() => setPreference(nextPreference)}
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
      onClick={() => setPreference(nextPreference)}
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
    <aside className="z-20 flex h-full w-[80px] shrink-0 flex-col items-center overflow-hidden border-r border-border-muted bg-elevated shadow-inset-divider backdrop-blur-xl backdrop-saturate-125">
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
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
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
  }, [location.pathname]);

  useEffect(() => {
    if (!menuOpen) {
      return;
    }

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) {
        return;
      }

      if (menuRef.current?.contains(target) || triggerRef.current?.contains(target)) {
        return;
      }

      setMenuOpen(false);
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") {
        return;
      }

      event.preventDefault();
      setMenuOpen(false);
      triggerRef.current?.focus();
    };

    window.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [menuOpen]);

  if (layoutPreference === "sidebar") {
    return (
      <div className="relative flex h-screen overflow-hidden bg-app text-text-primary">
        <SideBar />
        <MainContent isFullHeightRoute={isFullHeightRoute} />
        {bootMounted ? <BootOverlay variant="main" visible={bootVisible} /> : null}
      </div>
    );
  }

  return (
    <div className="relative flex h-screen flex-col overflow-hidden bg-app text-text-primary">
      <header className="relative z-20 flex h-12 shrink-0 items-center border-b border-border-muted bg-elevated backdrop-blur-xl backdrop-saturate-125">
        <div className="relative flex h-full items-center pl-[4.75rem] pr-2">
          <div className="relative">
            <Button
              unstyled
              ref={triggerRef}
              type="button"
              className={TITLEBAR_ICON_BUTTON_CLASS}
              title={switchMenuLabel}
              aria-label={switchMenuLabel}
              aria-expanded={menuOpen}
              aria-haspopup="menu"
              onClick={() => setMenuOpen((open) => !open)}
            >
              <span className={`btn-icon text-[1.05rem] ${currentNavItem.icon}`} aria-hidden="true" />
            </Button>

            {menuOpen ? (
              <div
                ref={menuRef}
                className="absolute left-0 top-[calc(100%+0.45rem)] z-50 min-w-[220px] rounded-md border border-border-muted/85 bg-surface-overlay p-2 shadow-overlay backdrop-blur-[24px] backdrop-saturate-140"
                role="menu"
                aria-label={t("titlebar.menuAria")}
              >
                <nav className="flex flex-col gap-1" aria-label={t("titlebar.menuAria")}>
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
                        onClick={() => setMenuOpen(false)}
                        className={({ isActive }) =>
                          [
                            TITLEBAR_MENU_ITEM_BASE_CLASS,
                            isActive ? TITLEBAR_MENU_ITEM_ACTIVE_CLASS : TITLEBAR_MENU_ITEM_IDLE_CLASS,
                          ].join(" ")
                        }
                      >
                        <span className={`btn-icon shrink-0 text-[1.1rem] ${item.icon}`} aria-hidden="true" />
                        <span className="truncate text-xs font-medium">{label}</span>
                      </NavLink>
                    );
                  })}
                </nav>
              </div>
            ) : null}
          </div>
        </div>

        <div className="h-full flex-1" data-tauri-drag-region />

        <div className="flex h-full items-center pr-3">
          <ThemeToggleIconButton />
        </div>
      </header>

      <MainContent isFullHeightRoute={isFullHeightRoute} />
      {bootMounted ? <BootOverlay variant="main" visible={bootVisible} /> : null}
    </div>
  );
}

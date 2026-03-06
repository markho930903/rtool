import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useLayoutEffect, useRef } from "react";
import { HashRouter, useLocation, useNavigate, useRoutes } from "react-router";

import type { ClipboardSyncPayload } from "@/components/clipboard/types";
import { MessageProvider } from "@ui/message/MessageProvider";
import type { LocaleStateDto, SettingsDto } from "@/contracts";
import { useAsyncEffect } from "@/hooks/useAsyncEffect";
import { useLocaleStore } from "@/i18n/store";
import { useLayoutStore } from "@/layouts/layout.store";
import { routes } from "@/routers";
import { launcherExecute } from "@/services/launcher.service";
import { listenWithCleanup } from "@/services/tauri-event";
import { getStartupSettings } from "@/services/startup-settings-cache";
import { useClipboardStore } from "@/stores/clipboard.store";
import { useThemeStore } from "@/theme/store";

function AppEventBridge() {
  const location = useLocation();
  const navigate = useNavigate();
  const currentRouteRef = useRef("/");
  const applySync = useClipboardStore((state) => state.applySync);
  const hydrateThemeFromSettings = useThemeStore((state) => state.hydrateFromSettings);
  const hydrateLocaleFromSettings = useLocaleStore((state) => state.hydrateFromSettings);
  const hydrateLayoutFromSettings = useLayoutStore((state) => state.hydrateFromSettings);
  const syncLocaleFromBackend = useLocaleStore((state) => state.syncFromBackend);
  const hydrateLocaleFromBackendState = useLocaleStore((state) => state.hydrateFromBackendState);

  useEffect(() => {
    currentRouteRef.current = `${location.pathname}${location.search}`;
  }, [location.pathname, location.search]);

  useAsyncEffect(
    async ({ stack }) => {
      const currentWindow = getCurrentWindow();

      listenWithCleanup<ClipboardSyncPayload>(
        stack,
        "rtool://clipboard/sync",
        (event) => {
          applySync(event.payload ?? {});
        },
        "app-event-bridge:clipboard-sync",
        "clipboard-sync",
      );

      listenWithCleanup<{ route: string }>(
        stack,
        "rtool://main/navigate",
        (event) => {
          if (currentWindow.label !== "main") {
            return;
          }

          if (!event.payload?.route) {
            return;
          }

          if (event.payload.route === currentRouteRef.current) {
            return;
          }

          navigate(event.payload.route);
        },
        "app-event-bridge:main-navigate",
        "main-navigate",
      );

      listenWithCleanup<SettingsDto>(
        stack,
        "rtool://settings/sync",
        (event) => {
          if (event.payload) {
            hydrateThemeFromSettings(event.payload);
            hydrateLocaleFromSettings(event.payload);
            hydrateLayoutFromSettings(event.payload);
            return;
          }
        },
        "app-event-bridge:settings-sync",
        "settings-sync",
      );

      listenWithCleanup<LocaleStateDto>(
        stack,
        "rtool://settings/locale_sync",
        (event) => {
          if (event.payload) {
            hydrateLocaleFromBackendState(event.payload);
            return;
          }
          void syncLocaleFromBackend();
        },
        "app-event-bridge:locale-sync",
        "locale-sync",
      );
    },
    [
      applySync,
      hydrateLocaleFromBackendState,
      hydrateLocaleFromSettings,
      hydrateLayoutFromSettings,
      hydrateThemeFromSettings,
      navigate,
      syncLocaleFromBackend,
    ],
    {
      scope: "app-event-bridge",
      onError: (error) => {
        if (import.meta.env.DEV) {
          console.warn("[app-event-bridge] setup failed", error);
        }
      },
    },
  );

  useEffect(() => {
    const currentWindow = getCurrentWindow();

    const isEditableTarget = (target: EventTarget | null): boolean => {
      if (!(target instanceof Element)) {
        return false;
      }

      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement
      ) {
        return true;
      }

      return target instanceof HTMLElement && target.isContentEditable;
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      const isSettingsShortcut =
        (event.metaKey || event.ctrlKey) &&
        !event.altKey &&
        !event.shiftKey &&
        !event.repeat &&
        event.code === "Comma";

      if (isSettingsShortcut) {
        event.preventDefault();
        void launcherExecute({ kind: "open_builtin_route", route: "/settings" }).catch((error) => {
          if (import.meta.env.DEV) {
            console.warn("[app-event-bridge] open settings shortcut failed", error);
          }
        });
        return;
      }

      if (currentWindow.label !== "main") {
        return;
      }

      if (event.key !== "Escape") {
        return;
      }

      if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
        return;
      }

      if (isEditableTarget(event.target)) {
        return;
      }

      event.preventDefault();
      void currentWindow.hide();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  return null;
}

function AppRoutes() {
  return useRoutes(routes);
}

function StartupBootstrap() {
  const initTheme = useThemeStore((state) => state.init);
  const hydrateThemeFromSettings = useThemeStore((state) => state.hydrateFromSettings);
  const initLocale = useLocaleStore((state) => state.init);
  const hydrateLocaleFromSettings = useLocaleStore((state) => state.hydrateFromSettings);
  const initLayout = useLayoutStore((state) => state.init);
  const hydrateLayoutFromSettings = useLayoutStore((state) => state.hydrateFromSettings);

  useEffect(() => {
    let active = true;

    void (async () => {
      try {
        const settings = await getStartupSettings();
        if (!active) {
          return;
        }

        hydrateThemeFromSettings(settings);
        hydrateLocaleFromSettings(settings);
        hydrateLayoutFromSettings(settings);
      } catch (error) {
        if (!active) {
          return;
        }

        if (import.meta.env.DEV) {
          console.warn("[startup-bootstrap] failed to load startup settings", error);
        }

        await Promise.all([initTheme(), initLocale(), initLayout()]);
      }
    })();

    return () => {
      active = false;
    };
  }, [
    hydrateLayoutFromSettings,
    hydrateLocaleFromSettings,
    hydrateThemeFromSettings,
    initLayout,
    initLocale,
    initTheme,
  ]);

  return null;
}

function WindowLabelBootstrap() {
  useLayoutEffect(() => {
    const currentWindow = getCurrentWindow();
    const { label } = currentWindow;
    document.documentElement.setAttribute("data-window-label", label);
    document.body.setAttribute("data-window-label", label);
    if (!document.documentElement.hasAttribute("data-window-transparency")) {
      document.documentElement.setAttribute("data-window-transparency", "off");
    }
  }, []);

  return null;
}

export default function App() {
  return (
    <HashRouter>
      <MessageProvider defaultPlacement="top-right" defaultDuration={3000}>
        <WindowLabelBootstrap />
        <StartupBootstrap />
        <AppEventBridge />
        <AppRoutes />
      </MessageProvider>
    </HashRouter>
  );
}

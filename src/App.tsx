import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useLayoutEffect, useRef } from "react";
import { HashRouter, useLocation, useNavigate, useRoutes } from "react-router";

import type { ClipboardSyncPayload } from "@/components/clipboard/types";
import type { TransferPeer, TransferProgressSnapshot } from "@/components/transfer/types";
import { useAsyncEffect } from "@/hooks/useAsyncEffect";
import { useLocaleStore } from "@/i18n/store";
import { useLayoutStore } from "@/layouts/layout.store";
import { routes } from "@/routers";
import { launcherExecute } from "@/services/launcher.service";
import { useClipboardStore } from "@/stores/clipboard.store";
import { useTransferStore } from "@/stores/transfer.store";
import { useThemeStore } from "@/theme/store";

function AppEventBridge() {
  const location = useLocation();
  const navigate = useNavigate();
  const currentRouteRef = useRef("/");
  const applySync = useClipboardStore((state) => state.applySync);
  const applyTransferPeerSync = useTransferStore((state) => state.applyPeerSync);
  const applyTransferSessionSync = useTransferStore((state) => state.applySessionSync);
  const refreshTransferHistory = useTransferStore((state) => state.refreshHistory);

  useEffect(() => {
    currentRouteRef.current = `${location.pathname}${location.search}`;
  }, [location.pathname, location.search]);

  useAsyncEffect(
    async ({ stack }) => {
      const currentWindow = getCurrentWindow();

      const unlistenClipboardSync = await listen<ClipboardSyncPayload>("rtool://clipboard/sync", (event) => {
        applySync(event.payload ?? {});
      });
      stack.add(unlistenClipboardSync, "clipboard-sync");

      const unlistenMainNavigate = await listen<{ route: string }>("rtool://main/navigate", (event) => {
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
      });
      stack.add(unlistenMainNavigate, "main-navigate");

      const unlistenTransferPeerSync = await listen<TransferPeer[]>("rtool://transfer/peer_sync", (event) => {
        if (currentWindow.label !== "main") {
          return;
        }
        applyTransferPeerSync(event.payload ?? []);
      });
      stack.add(unlistenTransferPeerSync, "transfer-peer-sync");

      const unlistenTransferSessionSync = await listen<TransferProgressSnapshot>("rtool://transfer/session_sync", (event) => {
        if (currentWindow.label !== "main") {
          return;
        }
        if (!event.payload) {
          return;
        }
        applyTransferSessionSync(event.payload);
      });
      stack.add(unlistenTransferSessionSync, "transfer-session-sync");

      const unlistenTransferHistorySync = await listen("rtool://transfer/history_sync", () => {
        if (currentWindow.label !== "main") {
          return;
        }
        void refreshTransferHistory();
      });
      stack.add(unlistenTransferHistorySync, "transfer-history-sync");
    },
    [applySync, applyTransferPeerSync, applyTransferSessionSync, navigate, refreshTransferHistory],
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

function ThemeBootstrap() {
  const initTheme = useThemeStore((state) => state.init);

  useEffect(() => {
    void initTheme();
  }, [initTheme]);

  return null;
}

function LocaleBootstrap() {
  const initLocale = useLocaleStore((state) => state.init);

  useEffect(() => {
    void initLocale();
  }, [initLocale]);

  return null;
}

function LayoutBootstrap() {
  const initLayout = useLayoutStore((state) => state.init);

  useEffect(() => {
    void initLayout();
  }, [initLayout]);

  return null;
}

function WindowLabelBootstrap() {
  useLayoutEffect(() => {
    const currentWindow = getCurrentWindow();
    const { label } = currentWindow;
    document.documentElement.setAttribute("data-window-label", label);
    document.body.setAttribute("data-window-label", label);
  }, []);

  return null;
}

export default function App() {
  return (
    <HashRouter>
      <WindowLabelBootstrap />
      <ThemeBootstrap />
      <LocaleBootstrap />
      <LayoutBootstrap />
      <AppEventBridge />
      <AppRoutes />
    </HashRouter>
  );
}

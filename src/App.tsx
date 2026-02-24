import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useLayoutEffect, useRef } from "react";
import { HashRouter, useLocation, useNavigate, useRoutes } from "react-router";

import type { AppManagerIndexUpdatedPayload } from "@/components/app-manager/types";
import type { ClipboardSyncPayload } from "@/components/clipboard/types";
import type { TransferPeer, TransferProgressSnapshot } from "@/components/transfer/types";
import { useLocaleStore } from "@/i18n/store";
import { useLayoutStore } from "@/layouts/layout.store";
import { routes } from "@/routers";
import { useAppManagerStore } from "@/stores/app-manager.store";
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
  const handleAppManagerIndexUpdated = useAppManagerStore((state) => state.handleIndexUpdated);

  useEffect(() => {
    currentRouteRef.current = `${location.pathname}${location.search}`;
  }, [location.pathname, location.search]);

  useEffect(() => {
    let unlistenClipboardSync: UnlistenFn | undefined;
    let unlistenMainNavigate: UnlistenFn | undefined;
    let unlistenTransferPeerSync: UnlistenFn | undefined;
    let unlistenTransferSessionSync: UnlistenFn | undefined;
    let unlistenTransferHistorySync: UnlistenFn | undefined;
    let unlistenAppManagerIndexUpdated: UnlistenFn | undefined;

    const setup = async () => {
      const currentWindow = getCurrentWindow();
      unlistenClipboardSync = await listen<ClipboardSyncPayload>("rtool://clipboard/sync", (event) => {
        applySync(event.payload ?? {});
      });

      unlistenMainNavigate = await listen<{ route: string }>("rtool://main/navigate", (event) => {
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

      unlistenTransferPeerSync = await listen<TransferPeer[]>("rtool://transfer/peer_sync", (event) => {
        if (currentWindow.label !== "main") {
          return;
        }
        applyTransferPeerSync(event.payload ?? []);
      });

      unlistenTransferSessionSync = await listen<TransferProgressSnapshot>("rtool://transfer/session_sync", (event) => {
        if (currentWindow.label !== "main") {
          return;
        }
        if (!event.payload) {
          return;
        }
        applyTransferSessionSync(event.payload);
      });

      unlistenTransferHistorySync = await listen("rtool://transfer/history_sync", () => {
        if (currentWindow.label !== "main") {
          return;
        }
        void refreshTransferHistory();
      });

      unlistenAppManagerIndexUpdated = await listen<AppManagerIndexUpdatedPayload>(
        "rtool://app-manager/index-updated",
        (event) => {
          if (currentWindow.label !== "main") {
            return;
          }
          if (!event.payload) {
            return;
          }
          handleAppManagerIndexUpdated(event.payload, currentRouteRef.current.startsWith("/app-manager"));
        },
      );
    };

    void setup();

    return () => {
      unlistenClipboardSync?.();
      unlistenMainNavigate?.();
      unlistenTransferPeerSync?.();
      unlistenTransferSessionSync?.();
      unlistenTransferHistorySync?.();
      unlistenAppManagerIndexUpdated?.();
    };
  }, [
    applySync,
    applyTransferPeerSync,
    applyTransferSessionSync,
    handleAppManagerIndexUpdated,
    navigate,
    refreshTransferHistory,
  ]);

  useEffect(() => {
    const currentWindow = getCurrentWindow();
    if (currentWindow.label !== "main") {
      return;
    }

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
    initTheme();
  }, [initTheme]);

  return null;
}

function LocaleBootstrap() {
  const initLocale = useLocaleStore((state) => state.init);

  useEffect(() => {
    initLocale();
  }, [initLocale]);

  return null;
}

function LayoutBootstrap() {
  const initLayout = useLayoutStore((state) => state.init);

  useEffect(() => {
    initLayout();
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

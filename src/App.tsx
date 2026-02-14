import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { HashRouter, useRoutes } from "react-router";
import { useEffect, useLayoutEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useNavigate } from "react-router";

import type { ClipboardSyncPayload } from "@/components/clipboard/types";
import { useLocaleStore } from "@/i18n/store";
import { useLayoutStore } from "@/layouts/layout.store";
import { routes } from "@/routers";
import { useClipboardStore } from "@/stores/clipboard.store";
import { useThemeStore } from "@/theme/store";

function AppEventBridge() {
  const navigate = useNavigate();
  const applySync = useClipboardStore((state) => state.applySync);

  useEffect(() => {
    let unlistenClipboardSync: UnlistenFn | undefined;
    let unlistenMainNavigate: UnlistenFn | undefined;

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

        navigate(event.payload.route);
      });
    };

    void setup();

    return () => {
      unlistenClipboardSync?.();
      unlistenMainNavigate?.();
    };
  }, [applySync, navigate]);

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

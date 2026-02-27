import type { ReactElement } from "react";
import type { RouteObject } from "react-router";

import AppLayout from "@/layouts/AppLayout";
import AppManagerPage from "@/pages/app-manager/AppManagerPage";
import ClipboardWindowPage from "@/pages/ClipboardWindowPage";
import HomePage from "@/pages/HomePage";
import LauncherWindowPage from "@/pages/LauncherWindowPage";
import LogCenterPage from "@/pages/LogCenterPage";
import NotFoundPage from "@/pages/NotFoundPage";
import SettingsPage from "@/pages/settings/SettingsPage";
import ToolsPage from "@/pages/ToolsPage";
import TransferPage from "@/pages/TransferPage";
import { getMainLayoutRouteConfig, getStandaloneRouteConfig, type AppRouteId } from "@/routers/routes.config";

const routeElementMap: Record<AppRouteId, ReactElement> = {
  dashboard: <HomePage />,
  tools: <ToolsPage />,
  transfer: <TransferPage />,
  logs: <LogCenterPage />,
  app_manager: <AppManagerPage />,
  settings: <SettingsPage />,
  not_found: <NotFoundPage />,
  clipboard: <ClipboardWindowPage />,
  launcher: <LauncherWindowPage />,
};

const mainLayoutChildren: RouteObject[] = getMainLayoutRouteConfig().map((item) => {
  const element = routeElementMap[item.id];
  if (item.index) {
    return {
      index: true,
      element,
    };
  }

  return {
    path: item.routePath,
    element,
  };
});

const standaloneRoutes: RouteObject[] = getStandaloneRouteConfig().map((item) => ({
  path: item.to,
  element: routeElementMap[item.id],
}));

export const routes: RouteObject[] = [
  {
    path: "/",
    element: <AppLayout />,
    children: mainLayoutChildren,
  },
  ...standaloneRoutes,
];

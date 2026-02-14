import type { RouteObject } from "react-router";

import AppLayout from "@/layouts/AppLayout";
import ClipboardWindowPage from "@/pages/ClipboardWindowPage";
import HomePage from "@/pages/HomePage";
import LauncherWindowPage from "@/pages/LauncherWindowPage";
import LogCenterPage from "@/pages/LogCenterPage";
import NotFoundPage from "@/pages/NotFoundPage";
import SettingsPage from "@/pages/SettingsPage";
import ToolsPage from "@/pages/ToolsPage";

export const routes: RouteObject[] = [
  {
    path: "/",
    element: <AppLayout />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "tools", element: <ToolsPage /> },
      { path: "logs", element: <LogCenterPage /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "*", element: <NotFoundPage /> },
    ],
  },
  {
    path: "/clipboard",
    element: <ClipboardWindowPage />,
  },
  {
    path: "/launcher",
    element: <LauncherWindowPage />,
  },
];

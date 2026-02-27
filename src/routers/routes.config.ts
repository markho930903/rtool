import type { WindowMode } from "@/stores/types";

export type AppRouteId =
  | "dashboard"
  | "tools"
  | "transfer"
  | "logs"
  | "app_manager"
  | "settings"
  | "not_found"
  | "clipboard"
  | "launcher";

export interface AppRouteConfig {
  id: AppRouteId;
  to: string;
  routePath?: string;
  index?: boolean;
  mainLayout: boolean;
  windowMode: WindowMode | null;
  menu?: {
    labelKey: string;
    icon: string;
    end?: boolean;
    order: number;
  };
  homeModule?: {
    nameKey: string;
    detailKey: string;
    state: "online";
    order: number;
  };
}

const ROUTE_CONFIG: AppRouteConfig[] = [
  {
    id: "dashboard",
    to: "/",
    index: true,
    mainLayout: true,
    windowMode: "dashboard",
    menu: {
      labelKey: "nav.dashboard",
      icon: "i-noto:desktop-computer",
      end: true,
      order: 0,
    },
    homeModule: {
      nameKey: "module.mainWindow.name",
      detailKey: "module.mainWindow.detail",
      state: "online",
      order: 0,
    },
  },
  {
    id: "tools",
    to: "/tools",
    routePath: "tools",
    mainLayout: true,
    windowMode: "tools",
    menu: {
      labelKey: "nav.tools",
      icon: "i-noto:hammer-and-wrench",
      order: 1,
    },
    homeModule: {
      nameKey: "module.tools.name",
      detailKey: "module.tools.detail",
      state: "online",
      order: 3,
    },
  },
  {
    id: "transfer",
    to: "/transfer",
    routePath: "transfer",
    mainLayout: true,
    windowMode: "transfer",
    menu: {
      labelKey: "nav.transfer",
      icon: "i-noto:outbox-tray",
      order: 2,
    },
    homeModule: {
      nameKey: "module.transfer.name",
      detailKey: "module.transfer.detail",
      state: "online",
      order: 4,
    },
  },
  {
    id: "logs",
    to: "/logs",
    routePath: "logs",
    mainLayout: true,
    windowMode: "logs",
    menu: {
      labelKey: "nav.logs",
      icon: "i-noto:scroll",
      order: 3,
    },
  },
  {
    id: "app_manager",
    to: "/app-manager",
    routePath: "app-manager",
    mainLayout: true,
    windowMode: "app-manager",
    menu: {
      labelKey: "nav.appManager",
      icon: "i-noto:card-index-dividers",
      order: 4,
    },
    homeModule: {
      nameKey: "layout:nav.appManager",
      detailKey: "app_manager:desc",
      state: "online",
      order: 5,
    },
  },
  {
    id: "settings",
    to: "/settings",
    routePath: "settings",
    mainLayout: true,
    windowMode: "dashboard",
    menu: {
      labelKey: "nav.settings",
      icon: "i-noto:gear",
      order: 6,
    },
  },
  {
    id: "not_found",
    to: "/*",
    routePath: "*",
    mainLayout: true,
    windowMode: null,
  },
  {
    id: "clipboard",
    to: "/clipboard",
    mainLayout: false,
    windowMode: null,
    homeModule: {
      nameKey: "module.clipboard.name",
      detailKey: "module.clipboard.detail",
      state: "online",
      order: 2,
    },
  },
  {
    id: "launcher",
    to: "/launcher",
    mainLayout: false,
    windowMode: "launcher",
    homeModule: {
      nameKey: "module.launcher.name",
      detailKey: "module.launcher.detail",
      state: "online",
      order: 1,
    },
  },
];

const MAIN_LAYOUT_ROUTE_CONFIG = ROUTE_CONFIG.filter((item) => item.mainLayout);
const STANDALONE_ROUTE_CONFIG = ROUTE_CONFIG.filter((item) => !item.mainLayout);

export function getMainLayoutRouteConfig(): AppRouteConfig[] {
  return MAIN_LAYOUT_ROUTE_CONFIG;
}

export function getStandaloneRouteConfig(): AppRouteConfig[] {
  return STANDALONE_ROUTE_CONFIG;
}

export function getMainMenuRouteConfig() {
  return ROUTE_CONFIG.filter((item): item is AppRouteConfig & { menu: NonNullable<AppRouteConfig["menu"]> } =>
    Boolean(item.menu),
  )
    .sort((left, right) => left.menu.order - right.menu.order)
    .map((item) => ({
      id: item.id,
      to: item.to,
      end: item.menu.end ?? false,
      labelKey: item.menu.labelKey,
      icon: item.menu.icon,
    }));
}

export function getHomeModuleRouteConfig() {
  return ROUTE_CONFIG.filter(
    (item): item is AppRouteConfig & { homeModule: NonNullable<AppRouteConfig["homeModule"]> } =>
      Boolean(item.homeModule),
  )
    .sort((left, right) => left.homeModule.order - right.homeModule.order)
    .map((item) => ({
      id: item.id,
      nameKey: item.homeModule.nameKey,
      detailKey: item.homeModule.detailKey,
      state: item.homeModule.state,
    }));
}

export function getRoutePathById(id: AppRouteId): string {
  return ROUTE_CONFIG.find((item) => item.id === id)?.to ?? "/";
}

function normalizePathname(pathname: string): string {
  const trimmed = pathname.trim();
  if (!trimmed) {
    return "/";
  }
  return trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
}

function isRouteMatch(pathname: string, routePath: string): boolean {
  if (routePath === "/") {
    return pathname === "/";
  }
  if (!pathname.startsWith(routePath)) {
    return false;
  }
  return pathname.length === routePath.length || pathname[routePath.length] === "/";
}

export function resolveWindowModeByPath(pathname: string): WindowMode {
  const normalized = normalizePathname(pathname);
  const matched = ROUTE_CONFIG.find(
    (item) => item.windowMode !== null && item.to !== "/*" && isRouteMatch(normalized, item.to),
  );
  return matched?.windowMode ?? "dashboard";
}

export function resolveActiveMainMenuByPath(pathname: string) {
  const normalized = normalizePathname(pathname);
  const menuItems = getMainMenuRouteConfig();
  return menuItems.find((item) => isRouteMatch(normalized, item.to)) ?? menuItems[0];
}

export function isRouteActiveById(id: AppRouteId, pathname: string): boolean {
  const route = ROUTE_CONFIG.find((item) => item.id === id);
  if (!route || route.to === "/*") {
    return false;
  }
  return isRouteMatch(normalizePathname(pathname), route.to);
}

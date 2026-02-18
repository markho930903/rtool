import { createInstance } from "i18next";
import ICU from "i18next-icu";
import { initReactI18next } from "react-i18next";

import { FALLBACK_LOCALE } from "@/i18n/constants";
import { applyLocaleToDocument, getStoredLocalePreference, resolveLocale } from "@/i18n/runtime";
import appManagerEnUS from "../../i18n/source/en-US/app_manager.json";
import clipboardEnUS from "../../i18n/source/en-US/clipboard.json";
import commonEnUS from "../../i18n/source/en-US/common.json";
import homeEnUS from "../../i18n/source/en-US/home.json";
import layoutEnUS from "../../i18n/source/en-US/layout.json";
import logsEnUS from "../../i18n/source/en-US/logs.json";
import notFoundEnUS from "../../i18n/source/en-US/not_found.json";
import paletteEnUS from "../../i18n/source/en-US/palette.json";
import settingsEnUS from "../../i18n/source/en-US/settings.json";
import transferEnUS from "../../i18n/source/en-US/transfer.json";
import toolsEnUS from "../../i18n/source/en-US/tools.json";
import appManagerZhCN from "../../i18n/source/zh-CN/app_manager.json";
import clipboardZhCN from "../../i18n/source/zh-CN/clipboard.json";
import commonZhCN from "../../i18n/source/zh-CN/common.json";
import homeZhCN from "../../i18n/source/zh-CN/home.json";
import layoutZhCN from "../../i18n/source/zh-CN/layout.json";
import logsZhCN from "../../i18n/source/zh-CN/logs.json";
import notFoundZhCN from "../../i18n/source/zh-CN/not_found.json";
import paletteZhCN from "../../i18n/source/zh-CN/palette.json";
import settingsZhCN from "../../i18n/source/zh-CN/settings.json";
import transferZhCN from "../../i18n/source/zh-CN/transfer.json";
import toolsZhCN from "../../i18n/source/zh-CN/tools.json";

const resources = {
  "zh-CN": {
    common: commonZhCN,
    layout: layoutZhCN,
    home: homeZhCN,
    app_manager: appManagerZhCN,
    tools: toolsZhCN,
    transfer: transferZhCN,
    logs: logsZhCN,
    settings: settingsZhCN,
    clipboard: clipboardZhCN,
    palette: paletteZhCN,
    not_found: notFoundZhCN,
  },
  "en-US": {
    common: commonEnUS,
    layout: layoutEnUS,
    home: homeEnUS,
    app_manager: appManagerEnUS,
    tools: toolsEnUS,
    transfer: transferEnUS,
    logs: logsEnUS,
    settings: settingsEnUS,
    clipboard: clipboardEnUS,
    palette: paletteEnUS,
    not_found: notFoundEnUS,
  },
} as const;

const initialLocale = resolveLocale(getStoredLocalePreference());

const i18n = createInstance();

void i18n
  .use(ICU)
  .use(initReactI18next)
  .init({
    resources,
    lng: initialLocale,
    fallbackLng: FALLBACK_LOCALE,
    defaultNS: "common",
    ns: [
      "common",
      "layout",
      "home",
      "app_manager",
      "tools",
      "transfer",
      "logs",
      "settings",
      "clipboard",
      "palette",
      "not_found",
    ],
    interpolation: {
      escapeValue: false,
    },
    returnNull: false,
  });

applyLocaleToDocument(initialLocale);

export default i18n;

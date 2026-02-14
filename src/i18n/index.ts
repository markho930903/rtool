import { createInstance } from "i18next";
import ICU from "i18next-icu";
import { initReactI18next } from "react-i18next";

import { FALLBACK_LOCALE } from "@/i18n/constants";
import { applyLocaleToDocument, getStoredLocalePreference, resolveLocale } from "@/i18n/runtime";
import clipboardEnUS from "../../i18n/source/en-US/clipboard.json";
import commonEnUS from "../../i18n/source/en-US/common.json";
import homeEnUS from "../../i18n/source/en-US/home.json";
import layoutEnUS from "../../i18n/source/en-US/layout.json";
import logsEnUS from "../../i18n/source/en-US/logs.json";
import notFoundEnUS from "../../i18n/source/en-US/notFound.json";
import paletteEnUS from "../../i18n/source/en-US/palette.json";
import settingsEnUS from "../../i18n/source/en-US/settings.json";
import toolsEnUS from "../../i18n/source/en-US/tools.json";
import clipboardZhCN from "../../i18n/source/zh-CN/clipboard.json";
import commonZhCN from "../../i18n/source/zh-CN/common.json";
import homeZhCN from "../../i18n/source/zh-CN/home.json";
import layoutZhCN from "../../i18n/source/zh-CN/layout.json";
import logsZhCN from "../../i18n/source/zh-CN/logs.json";
import notFoundZhCN from "../../i18n/source/zh-CN/notFound.json";
import paletteZhCN from "../../i18n/source/zh-CN/palette.json";
import settingsZhCN from "../../i18n/source/zh-CN/settings.json";
import toolsZhCN from "../../i18n/source/zh-CN/tools.json";

const resources = {
  "zh-CN": {
    common: commonZhCN,
    layout: layoutZhCN,
    home: homeZhCN,
    tools: toolsZhCN,
    logs: logsZhCN,
    settings: settingsZhCN,
    clipboard: clipboardZhCN,
    palette: paletteZhCN,
    notFound: notFoundZhCN,
  },
  "en-US": {
    common: commonEnUS,
    layout: layoutEnUS,
    home: homeEnUS,
    tools: toolsEnUS,
    logs: logsEnUS,
    settings: settingsEnUS,
    clipboard: clipboardEnUS,
    palette: paletteEnUS,
    notFound: notFoundEnUS,
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
    ns: ["common", "layout", "home", "tools", "logs", "settings", "clipboard", "palette", "notFound"],
    interpolation: {
      escapeValue: false,
    },
    returnNull: false,
  });

applyLocaleToDocument(initialLocale);

export default i18n;

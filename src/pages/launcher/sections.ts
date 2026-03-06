import type { PaletteItem } from "@/components/palette/types";

export type LauncherTopTab = "all" | "application" | "file" | "builtin";
export type LauncherSectionKey = "builtin" | "application" | "file" | "other";

export interface FlatLauncherItem {
  item: PaletteItem;
  absoluteIndex: number;
  section: LauncherSectionKey;
  flatIndex: number;
}

export interface LauncherSection {
  key: LauncherSectionKey;
  label: string;
  showHeader: boolean;
  items: FlatLauncherItem[];
}

export interface LauncherTab {
  key: LauncherTopTab;
  label: string;
}

export type LauncherSectionLabelMap = Record<LauncherSectionKey, string>;

const ORDERED_TABS: LauncherTopTab[] = ["all", "application", "file", "builtin"];
const ORDERED_SECTIONS: LauncherSectionKey[] = ["builtin", "application", "file", "other"];

export function buildLauncherTabs(t: (key: string) => string): LauncherTab[] {
  return [
    { key: "all", label: t("launcher.tab.all") },
    { key: "application", label: t("launcher.tab.application") },
    { key: "file", label: t("launcher.tab.file") },
    { key: "builtin", label: t("launcher.tab.builtin") },
  ];
}

export function buildLauncherSectionLabelMap(t: (key: string) => string): LauncherSectionLabelMap {
  return {
    builtin: t("launcher.section.builtin"),
    application: t("launcher.section.application"),
    file: t("launcher.section.file"),
    other: t("launcher.section.other"),
  };
}

export function nextTopTab(current: LauncherTopTab, step: 1 | -1): LauncherTopTab {
  const currentIndex = ORDERED_TABS.indexOf(current);
  const safeIndex = currentIndex < 0 ? 0 : currentIndex;
  const nextIndex = (safeIndex + step + ORDERED_TABS.length) % ORDERED_TABS.length;
  return ORDERED_TABS[nextIndex];
}

export function resolveSectionByCategory(category: string | undefined): LauncherSectionKey {
  if (category === "builtin") {
    return "builtin";
  }

  if (category === "application") {
    return "application";
  }

  if (category === "file" || category === "directory") {
    return "file";
  }

  return "other";
}

export function buildLauncherSections(
  items: PaletteItem[],
  activeTab: LauncherTopTab,
  sectionLabelMap: LauncherSectionLabelMap,
): { sections: LauncherSection[]; flatItems: FlatLauncherItem[] } {
  const base = items.map((item, absoluteIndex) => ({
    item,
    absoluteIndex,
    section: resolveSectionByCategory(item.category),
  }));

  const filtered = base.filter((entry) => matchesActiveTab(entry.section, activeTab));

  if (activeTab !== "all") {
    const sectionKey = activeTab === "application" ? "application" : activeTab === "file" ? "file" : "builtin";
    const flatItems = addFlatIndices(filtered, 0);

    return {
      sections: [
        {
          key: sectionKey,
          label: sectionLabelMap[sectionKey],
          showHeader: false,
          items: flatItems,
        },
      ],
      flatItems,
    };
  }

  let cursor = 0;
  const sections: LauncherSection[] = [];

  for (const key of ORDERED_SECTIONS) {
    const chunk = filtered.filter((entry) => entry.section === key);
    if (chunk.length === 0) {
      continue;
    }

    const keyedChunk = addFlatIndices(chunk, cursor);
    cursor += keyedChunk.length;
    sections.push({
      key,
      label: sectionLabelMap[key],
      showHeader: true,
      items: keyedChunk,
    });
  }

  return {
    sections,
    flatItems: sections.flatMap((section) => section.items),
  };
}

function matchesActiveTab(section: LauncherSectionKey, activeTab: LauncherTopTab): boolean {
  if (activeTab === "all") {
    return true;
  }

  if (activeTab === "application") {
    return section === "application";
  }

  if (activeTab === "file") {
    return section === "file";
  }

  return section === "builtin";
}

function addFlatIndices(
  entries: Array<Omit<FlatLauncherItem, "flatIndex">>,
  offset: number,
): FlatLauncherItem[] {
  return entries.map((entry, localIndex) => ({
    ...entry,
    flatIndex: offset + localIndex,
  }));
}

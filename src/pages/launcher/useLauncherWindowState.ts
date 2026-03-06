import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { selectLauncherItems, useLauncherStore } from "@/stores/launcher.store";

import { createHighlightContext } from "./highlight";
import {
  buildLauncherSectionLabelMap,
  buildLauncherSections,
  buildLauncherTabs,
  nextTopTab,
  type LauncherTopTab,
} from "./sections";

interface UseLauncherWindowStateOptions {
  t: (key: string) => string;
}

export function useLauncherWindowState({ t }: UseLauncherWindowStateOptions) {
  const query = useLauncherStore((state) => state.query);
  const items = useLauncherStore(selectLauncherItems);
  const loading = useLauncherStore((state) => state.loading);
  const launcherError = useLauncherStore((state) => state.error);
  const reset = useLauncherStore((state) => state.reset);
  const search = useLauncherStore((state) => state.search);
  const setStoreSelectedIndex = useLauncherStore((state) => state.setSelectedIndex);
  const executeSelected = useLauncherStore((state) => state.executeSelected);
  const setQuery = useLauncherStore((state) => state.setQuery);

  const [activeTab, setActiveTab] = useState<LauncherTopTab>("all");
  const [selectedVisibleIndex, setSelectedVisibleIndex] = useState(0);
  const [gridColumnCount, setGridColumnCount] = useState(1);

  const tabs = useMemo(() => buildLauncherTabs(t), [t]);
  const sectionLabelMap = useMemo(() => buildLauncherSectionLabelMap(t), [t]);
  const { sections, flatItems } = useMemo(
    () => buildLauncherSections(items, activeTab, sectionLabelMap),
    [activeTab, items, sectionLabelMap],
  );
  const selectedEntry = flatItems[selectedVisibleIndex] ?? null;
  const selectedItem = selectedEntry?.item ?? null;
  const activeTabLabel = tabs.find((tab) => tab.key === activeTab)?.label ?? t("launcher.tab.all");
  const highlightContext = useMemo(() => createHighlightContext(query), [query]);

  const flatItemsRef = useRef(flatItems);
  const selectedVisibleIndexRef = useRef(selectedVisibleIndex);

  useEffect(() => {
    flatItemsRef.current = flatItems;
  }, [flatItems]);

  useEffect(() => {
    selectedVisibleIndexRef.current = selectedVisibleIndex;
  }, [selectedVisibleIndex]);

  useEffect(() => {
    if (flatItems.length === 0) {
      if (selectedVisibleIndex !== 0) {
        setSelectedVisibleIndex(0);
      }
      return;
    }

    const maxVisibleIndex = flatItems.length - 1;
    if (selectedVisibleIndex > maxVisibleIndex) {
      setSelectedVisibleIndex(maxVisibleIndex);
    }
  }, [flatItems.length, selectedVisibleIndex]);

  const updateQuery = useCallback(
    (value: string) => {
      setQuery(value);
      setSelectedVisibleIndex(0);
    },
    [setQuery],
  );

  const updateActiveTab = useCallback((tab: LauncherTopTab) => {
    setActiveTab(tab);
    setSelectedVisibleIndex(0);
  }, []);

  const cycleActiveTab = useCallback((step: 1 | -1) => {
    setActiveTab((current) => nextTopTab(current, step));
    setSelectedVisibleIndex(0);
  }, []);

  const resetViewState = useCallback(() => {
    setActiveTab("all");
    setSelectedVisibleIndex(0);
  }, []);

  const moveGridSelection = useCallback((delta: number) => {
    const currentFlatItems = flatItemsRef.current;
    if (currentFlatItems.length === 0) {
      return;
    }

    setSelectedVisibleIndex((current) => {
      const next = current + delta;
      if (next < 0) {
        return 0;
      }
      if (next >= currentFlatItems.length) {
        return currentFlatItems.length - 1;
      }
      return next;
    });
  }, []);

  const executeVisibleSelection = useCallback(
    async (index: number | undefined, onSuccess?: () => void) => {
      const currentSelectedIndex = selectedVisibleIndexRef.current;
      const targetIndex = index ?? currentSelectedIndex;
      const entry = flatItemsRef.current[targetIndex];
      if (!entry) {
        return null;
      }

      if (targetIndex !== currentSelectedIndex) {
        setSelectedVisibleIndex(targetIndex);
      }

      setStoreSelectedIndex(entry.absoluteIndex);
      const result = await executeSelected();
      if (result?.ok) {
        onSuccess?.();
      }
      return result;
    },
    [executeSelected, setStoreSelectedIndex],
  );

  return {
    query,
    items,
    loading,
    launcherError,
    reset,
    search,
    tabs,
    activeTab,
    activeTabLabel,
    selectedVisibleIndex,
    setSelectedVisibleIndex,
    gridColumnCount,
    setGridColumnCount,
    sections,
    flatItems,
    selectedEntry,
    selectedItem,
    highlightContext,
    updateQuery,
    updateActiveTab,
    cycleActiveTab,
    resetViewState,
    moveGridSelection,
    executeVisibleSelection,
  };
}

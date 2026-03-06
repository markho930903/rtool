import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type {
  AppManagerIndexState,
  AppManagerIndexUpdatedPayload,
  ManagedApp,
} from "@/components/app-manager/types";
import { useLatestRef } from "@/hooks/useLatestRef";
import { useAsyncEffect } from "@/hooks/useAsyncEffect";
import {
  appManagerList,
  appManagerRefreshIndex,
} from "@/services/app-manager.service";
import { listenWithCleanup } from "@/services/tauri-event";

import { formatIndexedAt } from "./state";
import { useAppManagerSizeResolution } from "./useAppManagerSizeResolution";

const PAGE_SIZE = 120;
const KEYWORD_DEBOUNCE_MS = 220;

interface UseAppManagerListStateOptions {
  onItemsReplaced?: (context: { items: ManagedApp[]; keepIds: Set<string> }) => void;
}

export function useAppManagerListState(options: UseAppManagerListStateOptions = {}) {
  const { onItemsReplaced } = options;

  const [items, setItems] = useState<ManagedApp[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [keyword, setKeyword] = useState("");
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [totalCount, setTotalCount] = useState(0);
  const [indexedAt, setIndexedAt] = useState<number | null>(null);
  const [revision, setRevision] = useState(0);
  const [indexState, setIndexState] = useState<AppManagerIndexState>("ready");
  const [listError, setListError] = useState<string | null>(null);
  const [selectedAppId, setSelectedAppId] = useState<string | null>(null);

  const listRequestSeqRef = useRef(0);
  const firstListLoadDoneRef = useRef(false);

  const keywordRef = useLatestRef(keyword);
  const selectedAppIdRef = useLatestRef(selectedAppId);
  const onItemsReplacedRef = useLatestRef(onItemsReplaced);

  const { enqueueSizeResolution, mergeListItems } = useAppManagerSizeResolution({
    items,
    setItems,
    setListError,
  });

  const loadListPage = useCallback(
    async (cursor?: string, replace = false, keywordValue?: string) => {
      const requestSeq = ++listRequestSeqRef.current;
      if (replace) {
        setLoading(true);
        setListError(null);
      } else {
        setLoadingMore(true);
      }

      try {
        const keywordText = (keywordValue ?? keywordRef.current).trim();
        const page = await appManagerList({
          keyword: keywordText ? keywordText : undefined,
          limit: PAGE_SIZE,
          cursor,
        });

        if (requestSeq !== listRequestSeqRef.current) {
          return;
        }

        const { mergedItems, keepIds } = mergeListItems(page.items, replace);
        if (replace) {
          onItemsReplacedRef.current?.({ items: mergedItems, keepIds });
        }

        setNextCursor(page.nextCursor);
        setIndexedAt(page.indexedAt);
        setRevision(page.revision);
        setIndexState(page.indexState);
        setTotalCount(page.totalCount);
        setSelectedAppId((current) => {
          if (current && mergedItems.some((item) => item.id === current)) {
            return current;
          }
          return null;
        });
      } catch (error) {
        if (requestSeq !== listRequestSeqRef.current) {
          return;
        }
        setListError(error instanceof Error ? error.message : String(error));
      } finally {
        if (requestSeq === listRequestSeqRef.current) {
          if (replace) {
            setLoading(false);
          } else {
            setLoadingMore(false);
          }
        }
      }
    },
    [keywordRef, mergeListItems, onItemsReplacedRef],
  );

  const loadListFirstPage = useCallback(
    async (keywordValue?: string) => {
      await loadListPage(undefined, true, keywordValue);
    },
    [loadListPage],
  );

  const refreshList = useCallback(async () => {
    setRefreshing(true);
    try {
      await appManagerRefreshIndex();
      await loadListFirstPage(keywordRef.current);
    } catch (error) {
      setListError(error instanceof Error ? error.message : String(error));
    } finally {
      setRefreshing(false);
    }
  }, [keywordRef, loadListFirstPage]);

  const selectApp = useCallback((appId: string) => {
    setSelectedAppId(appId);
  }, []);

  const loadMore = useCallback(async () => {
    if (!nextCursor || loadingMore) {
      return;
    }
    await loadListPage(nextCursor, false, keywordRef.current);
  }, [keywordRef, loadListPage, loadingMore, nextCursor]);

  useEffect(() => {
    const delayMs = firstListLoadDoneRef.current ? KEYWORD_DEBOUNCE_MS : 40;
    const timer = window.setTimeout(() => {
      firstListLoadDoneRef.current = true;
      void loadListFirstPage(keyword);
    }, delayMs);

    return () => {
      window.clearTimeout(timer);
    };
  }, [keyword, loadListFirstPage]);

  useAsyncEffect(
    ({ stack }) => {
      stack.add(() => {
        listRequestSeqRef.current += 1;
      }, "invalidate-list-request-seq");

      listenWithCleanup<AppManagerIndexUpdatedPayload>(
        stack,
        "rtool://app-manager/index-updated",
        (event) => {
          if (!event.payload) {
            return;
          }
          setRevision((previous) => Math.max(previous, event.payload.revision));
          setIndexedAt(event.payload.indexedAt);
          void loadListFirstPage(keywordRef.current);
        },
        "app-manager:index-updated",
        "index-updated",
      );
    },
    [keywordRef, loadListFirstPage],
    {
      scope: "app-manager-list-state",
      onError: (error) => {
        if (import.meta.env.DEV) {
          console.warn("[app-manager-list-state] event listen setup failed", error);
        }
      },
    },
  );

  const listModel = useMemo(
    () => ({
      items,
      loading: loading || refreshing,
      loadingMore,
      hasMore: Boolean(nextCursor),
      keyword,
      totalCount,
      revision,
      indexState,
      listError,
      selectedAppId,
      indexedAtText: formatIndexedAt(indexedAt),
      onKeywordChange: setKeyword,
      onSelect: selectApp,
      onRefresh: refreshList,
      onLoadMore: loadMore,
    }),
    [indexState, indexedAt, items, keyword, listError, loadMore, loading, loadingMore, nextCursor, refreshList, refreshing, revision, selectApp, selectedAppId, totalCount],
  );

  return {
    items,
    listError,
    keyword,
    selectedAppId,
    selectedAppIdRef,
    listModel,
    enqueueSizeResolution,
    loadListFirstPage,
    refreshList,
    selectApp,
    setKeyword,
    setSelectedAppId,
  };
}

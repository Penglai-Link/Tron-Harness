import {
  useCallback,
  useEffect,
  useState,
} from "react";

import { fetchAppPage } from "@/domain/app/api";
import { listAppPage } from "@/domain/app/mock";
import type {
  AppListItem,
  AppListQuery,
  AppPage,
} from "@/domain/app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function emptyPage(query: AppListQuery): AppPage<AppListItem> {
  return {
    current: query.current ?? 1,
    size: query.size ?? 15,
    total: 0,
    records: [],
  };
}

export interface AppListResult {
  loading: boolean;
  page: AppPage<AppListItem>;
  refresh: () => void;
}

export function useAppList(query: AppListQuery): AppListResult {
  const dataSource = getServiceDataSource("app");
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState<AppPage<AppListItem>>(() => (
    dataSource === "api" ? emptyPage(query) : listAppPage(query)
  ));
  const queryKey = JSON.stringify(query);

  useEffect(() => {
    if (dataSource === "api") {
      setPage(emptyPage(query));
      return;
    }
    setPage(listAppPage(query));
  }, [dataSource, queryKey, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);

    fetchAppPage(query, controller.signal)
      .then(setPage)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[app] App list request failed.", error);
        setPage(emptyPage(query));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, queryKey, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    loading,
    page,
    refresh,
  };
}

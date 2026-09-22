import {
  useCallback,
  useEffect,
  useState,
} from "react";

import { fetchAppOptions } from "@/domain/app/api";
import { listAppOptions } from "@/domain/app/mock";
import type {
  AppOption,
  AppOptionQuery,
} from "@/domain/app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface AppOptionsResult {
  loading: boolean;
  error: boolean;
  refresh: () => void;
  options: AppOption[];
}

export function useAppOptions(query: AppOptionQuery): AppOptionsResult {
  const dataSource = getServiceDataSource("app");
  const [revision, setRevision] = useState(0);
  const [error, setError] = useState(false);
  const refresh = useCallback(() => {
    setLoading(true);
    setError(false);
    setRevision((value) => value + 1);
  }, []);
  const [loading, setLoading] = useState(false);
  const [options, setOptions] = useState<AppOption[]>(() => (
    dataSource === "api" ? [] : listAppOptions(query)
  ));
  const queryKey = JSON.stringify(query);

  useEffect(() => {
    if (dataSource === "api") return;
    setOptions(listAppOptions(query));
    setLoading(false);
  }, [dataSource, queryKey, revision]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);
    setError(false);

    fetchAppOptions(query, controller.signal)
      .then((items) => {
        if (!controller.signal.aborted) setOptions(items);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[app] App options request failed.", error);
        setError(true);
        setOptions([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, queryKey, revision]);

  return {
    loading,
    error,
    refresh,
    options,
  };
}

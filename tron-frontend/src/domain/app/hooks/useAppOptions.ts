import {
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
  options: AppOption[];
}

export function useAppOptions(query: AppOptionQuery): AppOptionsResult {
  const dataSource = getServiceDataSource("app");
  const [loading, setLoading] = useState(false);
  const [options, setOptions] = useState<AppOption[]>(() => (
    dataSource === "api" ? [] : listAppOptions(query)
  ));
  const queryKey = JSON.stringify(query);

  useEffect(() => {
    if (dataSource === "api") return;
    setOptions(listAppOptions(query));
  }, [dataSource, queryKey]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);

    fetchAppOptions(query, controller.signal)
      .then(setOptions)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[app] App options request failed.", error);
        setOptions([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, queryKey]);

  return {
    loading,
    options,
  };
}

import {
  useCallback,
  useEffect,
  useState,
} from "react";

import { fetchAppDetail } from "@/domain/app/api";
import { getAppDetail } from "@/domain/app/mock";
import type { AppDetailData } from "@/domain/app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface AppDetailResult {
  app: AppDetailData | null;
  loading: boolean;
  refresh: () => void;
  setApp: (app: AppDetailData | null) => void;
}

export function useAppDetail(appCode: string): AppDetailResult {
  const dataSource = getServiceDataSource("app");
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [app, setApp] = useState<AppDetailData | null>(() => {
    if (dataSource === "api" || !appCode.trim()) return null;
    try {
      return getAppDetail(appCode);
    } catch {
      return null;
    }
  });

  useEffect(() => {
    if (!appCode.trim()) {
      setApp(null);
      return;
    }

    if (dataSource === "api") return;
    try {
      setApp(getAppDetail(appCode));
    } catch {
      setApp(null);
    }
  }, [appCode, dataSource, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api" || !appCode.trim()) return;
    const controller = new AbortController();
    setApp(null);
    setLoading(true);

    fetchAppDetail(appCode, controller.signal)
      .then((value) => setApp(value))
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[app] App detail request failed.", error);
        setApp(null);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [appCode, dataSource, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    app,
    loading,
    refresh,
    setApp,
  };
}

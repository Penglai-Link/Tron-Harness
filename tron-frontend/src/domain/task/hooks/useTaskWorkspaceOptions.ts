import {
  useEffect,
  useState,
} from "react";

import { fetchAppOptions } from "@/domain/app/api";
import { listAppOptions } from "@/domain/app/mock";
import type { AppOption } from "@/domain/app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface TaskWorkspaceOptions {
  appOptions: AppOption[];
  loading: boolean;
}

export function useTaskWorkspaceOptions(workspaceCode: string): TaskWorkspaceOptions {
  const appDataSource = getServiceDataSource("app");
  const [loadingApps, setLoadingApps] = useState(false);
  const [appOptions, setAppOptions] = useState(() => (
    appDataSource === "api" ? [] : listAppOptions({ workspaceCode })
  ));

  useEffect(() => {
    if (appDataSource === "api") return;
    setAppOptions(listAppOptions({ workspaceCode }));
  }, [appDataSource, workspaceCode]);

  useEffect(() => {
    if (appDataSource !== "api") return;
    const controller = new AbortController();
    setLoadingApps(true);

    const request = workspaceCode
      ? fetchAppOptions({ workspaceCode }, controller.signal)
      : Promise.resolve([]);

    request
      .then((records) => {
        setAppOptions(records);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Workspace app options request failed.", error);
        setAppOptions([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoadingApps(false);
      });

    return () => controller.abort();
  }, [appDataSource, workspaceCode]);


  return {
    appOptions,
    loading: loadingApps,
  };
}

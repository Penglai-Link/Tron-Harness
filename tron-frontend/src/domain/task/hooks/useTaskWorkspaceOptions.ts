import {
  useEffect,
  useState,
} from "react";

import { fetchAppOptions } from "@/domain/app/api";
import { listAppOptions } from "@/domain/app/mock";
import type { AppOption } from "@/domain/app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface TaskAssigneeOption {
  userCode: string;
  displayName: string;
  email?: string;
}

export interface TaskWorkspaceOptions {
  appOptions: AppOption[];
  effectiveUsers: TaskAssigneeOption[];
  loading: boolean;
  onUserPageChange: (page: number) => void;
  userPage: number;
  userPageCount: number;
}

export function useTaskWorkspaceOptions(workspaceCode: string): TaskWorkspaceOptions {
  const appDataSource = getServiceDataSource("app");
  const [loadingApps, setLoadingApps] = useState(false);
  const userPage = 1;
  const userPageCount = 1;
  const [appOptions, setAppOptions] = useState(() => (
    appDataSource === "api" ? [] : listAppOptions({ workspaceCode })
  ));
  const effectiveUsers: TaskAssigneeOption[] = [];

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

  const onUserPageChange = () => undefined;

  return {
    appOptions,
    effectiveUsers,
    loading: loadingApps,
    onUserPageChange,
    userPage,
    userPageCount,
  };
}

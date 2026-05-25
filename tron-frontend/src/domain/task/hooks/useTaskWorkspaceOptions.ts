import {
  useEffect,
  useState,
} from "react";

import { fetchAppOptions } from "@/domain/app/api";
import { listAppOptions } from "@/domain/app/mock";
import type { AppOption } from "@/domain/app/model";
import {
  fetchWorkspaceEffectiveUsers,
} from "@/domain/admin/api";
import {
  getWorkspaceEffectiveUsers,
} from "@/domain/admin/mock";
import type { WorkspaceEffectiveUserRecord } from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface TaskWorkspaceOptions {
  appOptions: AppOption[];
  effectiveUsers: WorkspaceEffectiveUserRecord[];
  loading: boolean;
}

export function useTaskWorkspaceOptions(workspaceCode: string): TaskWorkspaceOptions {
  const adminDataSource = getServiceDataSource("admin");
  const appDataSource = getServiceDataSource("app");
  const [loadingUsers, setLoadingUsers] = useState(false);
  const [loadingApps, setLoadingApps] = useState(false);
  const [appOptions, setAppOptions] = useState(() => (
    appDataSource === "api" ? [] : listAppOptions({ workspaceCode })
  ));
  const [effectiveUsers, setEffectiveUsers] = useState(() => (
    adminDataSource === "api" ? [] : getWorkspaceEffectiveUsers(workspaceCode)
  ));

  useEffect(() => {
    if (adminDataSource === "api") return;
    setEffectiveUsers(getWorkspaceEffectiveUsers(workspaceCode));
  }, [adminDataSource, workspaceCode]);

  useEffect(() => {
    if (appDataSource === "api") return;
    setAppOptions(listAppOptions({ workspaceCode }));
  }, [appDataSource, workspaceCode]);

  useEffect(() => {
    if (adminDataSource !== "api") return;
    const controller = new AbortController();
    setLoadingUsers(true);

    const request = workspaceCode
      ? fetchWorkspaceEffectiveUsers(workspaceCode, { current: 1, size: 100 }, controller.signal)
      : Promise.resolve({ current: 1, size: 100, total: 0, records: [] });

    request
      .then((userPage) => {
        setEffectiveUsers(userPage.records);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Workspace users request failed.", error);
        setEffectiveUsers([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoadingUsers(false);
      });

    return () => controller.abort();
  }, [adminDataSource, workspaceCode]);

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
    effectiveUsers,
    loading: loadingUsers || loadingApps,
  };
}

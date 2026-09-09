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
  onUserPageChange: (page: number) => void;
  userPage: number;
  userPageCount: number;
}

const USER_PAGE_SIZE = 100;

export function useTaskWorkspaceOptions(workspaceCode: string): TaskWorkspaceOptions {
  const adminDataSource = getServiceDataSource("admin");
  const appDataSource = getServiceDataSource("app");
  const [loadingUsers, setLoadingUsers] = useState(false);
  const [loadingApps, setLoadingApps] = useState(false);
  const [userPage, setUserPage] = useState(1);
  const [userPageCount, setUserPageCount] = useState(1);
  const [appOptions, setAppOptions] = useState(() => (
    appDataSource === "api" ? [] : listAppOptions({ workspaceCode })
  ));
  const [effectiveUsers, setEffectiveUsers] = useState(() => (
    adminDataSource === "api" ? [] : getWorkspaceEffectiveUsers(workspaceCode).slice(0, USER_PAGE_SIZE)
  ));

  useEffect(() => {
    setUserPage(1);
  }, [workspaceCode]);

  useEffect(() => {
    if (adminDataSource === "api") return;
    const users = getWorkspaceEffectiveUsers(workspaceCode);
    const start = (userPage - 1) * USER_PAGE_SIZE;
    setEffectiveUsers(users.slice(start, start + USER_PAGE_SIZE));
    setUserPageCount(Math.max(1, Math.ceil(users.length / USER_PAGE_SIZE)));
  }, [adminDataSource, workspaceCode, userPage]);

  useEffect(() => {
    if (appDataSource === "api") return;
    setAppOptions(listAppOptions({ workspaceCode }));
  }, [appDataSource, workspaceCode]);

  useEffect(() => {
    if (adminDataSource !== "api") return;
    const controller = new AbortController();
    setLoadingUsers(true);

    const request = workspaceCode
      ? fetchWorkspaceEffectiveUsers(workspaceCode, { current: userPage, size: USER_PAGE_SIZE }, controller.signal)
      : Promise.resolve({ current: 1, size: USER_PAGE_SIZE, total: 0, records: [] as WorkspaceEffectiveUserRecord[] });

    request
      .then((page) => {
        setEffectiveUsers(page.records);
        setUserPageCount(Math.max(1, page.total));
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Workspace users request failed.", error);
        setEffectiveUsers([]);
        setUserPageCount(1);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoadingUsers(false);
      });

    return () => controller.abort();
  }, [adminDataSource, workspaceCode, userPage]);

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

  const onUserPageChange = (page: number) => {
    setUserPage(Math.max(1, page));
  };

  return {
    appOptions,
    effectiveUsers,
    loading: loadingUsers || loadingApps,
    onUserPageChange,
    userPage,
    userPageCount,
  };
}

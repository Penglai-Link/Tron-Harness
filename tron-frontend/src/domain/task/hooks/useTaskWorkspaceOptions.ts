import {
  useEffect,
  useMemo,
  useState,
} from "react";

import {
  fetchWorkspaceEffectiveUsers,
} from "@/domain/admin/api";
import {
  getWorkspaceEffectiveUsers,
} from "@/domain/admin/mock";
import type { WorkspaceEffectiveUserRecord } from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface TaskWorkspaceOptions {
  effectiveUsers: WorkspaceEffectiveUserRecord[];
  loading: boolean;
}

export function useTaskWorkspaceOptions(workspaceCode: string): TaskWorkspaceOptions {
  const dataSource = getServiceDataSource("admin");
  const [loading, setLoading] = useState(false);
  const [effectiveUsers, setEffectiveUsers] = useState(() => getWorkspaceEffectiveUsers(workspaceCode));

  useEffect(() => {
    if (dataSource === "api") return;
    setEffectiveUsers(getWorkspaceEffectiveUsers(workspaceCode));
  }, [dataSource, workspaceCode]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);

    const request = workspaceCode
      ? fetchWorkspaceEffectiveUsers(workspaceCode, { current: 1, size: 100 }, controller.signal)
      : Promise.resolve({ current: 1, size: 100, total: 0, records: [] });

    request
      .then((userPage) => {
        setEffectiveUsers(userPage.records);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Falling back to mock workspace users.", error);
        setEffectiveUsers(getWorkspaceEffectiveUsers(workspaceCode));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, workspaceCode]);

  return {
    effectiveUsers,
    loading,
  };
}

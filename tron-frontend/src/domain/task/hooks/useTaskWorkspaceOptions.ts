import {
  useEffect,
  useMemo,
  useState,
} from "react";

import {
  fetchWorkspaceEffectiveUsers,
  fetchWorkspaceOptions,
} from "@/domain/admin/api";
import {
  getWorkspaceEffectiveUsers,
} from "@/domain/admin/mock";
import type { WorkspaceEffectiveUserRecord } from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { getWorkspaceOptionMocks } from "@/shared/constants/workspaceSpaces";
import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";

export interface TaskWorkspaceOptions {
  effectiveUsers: WorkspaceEffectiveUserRecord[];
  loading: boolean;
  workspaces: WorkspaceOption[];
}

export function useTaskWorkspaceOptions(workspaceCode: string): TaskWorkspaceOptions {
  const dataSource = getServiceDataSource("admin");
  const [loading, setLoading] = useState(false);
  const [workspaces, setWorkspaces] = useState(() => getWorkspaceOptionMocks());
  const [effectiveUsers, setEffectiveUsers] = useState(() => getWorkspaceEffectiveUsers(workspaceCode));

  useEffect(() => {
    if (dataSource === "api") return;
    setWorkspaces(getWorkspaceOptionMocks());
    setEffectiveUsers(getWorkspaceEffectiveUsers(workspaceCode));
  }, [dataSource, workspaceCode]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);

    Promise.all([
      fetchWorkspaceOptions({}, controller.signal),
      workspaceCode
        ? fetchWorkspaceEffectiveUsers(workspaceCode, { current: 1, size: 100 }, controller.signal)
        : Promise.resolve({ current: 1, size: 100, total: 0, records: [] }),
    ])
      .then(([workspaceOptions, userPage]) => {
        setWorkspaces(workspaceOptions);
        setEffectiveUsers(userPage.records);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Falling back to mock workspace options.", error);
        setWorkspaces(getWorkspaceOptionMocks());
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
    workspaces,
  };
}

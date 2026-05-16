import {
  useEffect,
  useMemo,
  useState,
} from "react";

import {
  fetchWorkspaceEffectiveUsers,
  fetchWorkspaceTree,
} from "@/domain/admin/api";
import {
  getWorkspaceEffectiveUsers,
  getWorkspaceTree,
} from "@/domain/admin/mock";
import type {
  WorkspaceEffectiveUserRecord,
  WorkspaceTreeNodeRecord,
} from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface TaskWorkspaceOptions {
  effectiveUsers: WorkspaceEffectiveUserRecord[];
  loading: boolean;
  workspaces: WorkspaceTreeNodeRecord[];
}

function flattenWorkspaceTree(records: WorkspaceTreeNodeRecord[]): WorkspaceTreeNodeRecord[] {
  return records.flatMap((record) => [record, ...flattenWorkspaceTree(record.children)]);
}

export function useTaskWorkspaceOptions(workspaceCode: string): TaskWorkspaceOptions {
  const dataSource = getServiceDataSource("admin");
  const [loading, setLoading] = useState(false);
  const [workspaceTree, setWorkspaceTree] = useState(() => getWorkspaceTree());
  const [effectiveUsers, setEffectiveUsers] = useState(() => getWorkspaceEffectiveUsers(workspaceCode));
  const workspaces = useMemo(
    () => flattenWorkspaceTree(workspaceTree).filter((workspace) => workspace.status === "Active"),
    [workspaceTree],
  );

  useEffect(() => {
    if (dataSource === "api") return;
    setWorkspaceTree(getWorkspaceTree());
    setEffectiveUsers(getWorkspaceEffectiveUsers(workspaceCode));
  }, [dataSource, workspaceCode]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);

    Promise.all([
      fetchWorkspaceTree({}, controller.signal),
      workspaceCode
        ? fetchWorkspaceEffectiveUsers(workspaceCode, { current: 1, size: 100 }, controller.signal)
        : Promise.resolve({ current: 1, size: 100, total: 0, records: [] }),
    ])
      .then(([tree, userPage]) => {
        setWorkspaceTree(tree);
        setEffectiveUsers(userPage.records);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Falling back to mock workspace options.", error);
        setWorkspaceTree(getWorkspaceTree());
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

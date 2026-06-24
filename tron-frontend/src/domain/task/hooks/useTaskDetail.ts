import {
  useCallback,
  useEffect,
  useState,
} from "react";

import { fetchTaskDetail } from "@/domain/task/api";
import { getTaskDetail } from "@/domain/task/mock/data";
import type { TaskDetailData } from "@/domain/task/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function emptyTaskDetail(taskCode: string): TaskDetailData {
  return {
    agentCode: "main-agent",
    agentCodes: [],
    appCodes: [],
    archivedAt: null,
    assigneeUserCodes: [],
    createdAt: "",
    description: null,
    executionResult: null,
    id: "",
    modelCode: null,
    parent: null,
    parentTaskCode: null,
    previousStatus: null,
    priority: "Medium",
    progress: 0,
    sourceType: "Internal",
    sourceUrl: null,
    status: "Open",
    subtasks: [],
    taskCode,
    taskType: "Feature",
    title: "",
    updatedAt: "",
    workspaceCode: "",
  };
}

export interface TaskDetailResult {
  loading: boolean;
  refresh: () => void;
  setTask: (task: TaskDetailData) => void;
  task: TaskDetailData;
}

export function useTaskDetail(taskCode = "TSK-SEED-BACKLOG"): TaskDetailResult {
  const dataSource = getServiceDataSource("task");
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [task, setTask] = useState(() => (
    dataSource === "api" ? emptyTaskDetail(taskCode) : getTaskDetail(taskCode)
  ));

  useEffect(() => {
    if (dataSource === "api") return;
    setTask(getTaskDetail(taskCode));
  }, [dataSource, taskCode, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);

    fetchTaskDetail(taskCode, controller.signal)
      .then(setTask)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Task detail request failed.", error);
        setTask(emptyTaskDetail(taskCode));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, reloadKey, taskCode]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    loading,
    refresh,
    setTask,
    task,
  };
}

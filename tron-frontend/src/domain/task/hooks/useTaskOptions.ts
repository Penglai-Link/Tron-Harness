import {
  useEffect,
  useState,
} from "react";

import { fetchTaskOptions } from "@/domain/task/api";
import { getTaskOptions } from "@/domain/task/mock/data";
import type { TaskOptions } from "@/domain/task/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function emptyTaskOptions(): TaskOptions {
  return {
    flow: { flowCode: "", name: "", statuses: [] },
    priorities: [],
    sourceTypes: [],
    statuses: [],
    taskTypes: [],
  };
}

export function useTaskOptions(workspaceCode = ""): TaskOptions {
  const dataSource = getServiceDataSource("task");
  const [options, setOptions] = useState(() => (
    dataSource === "api" ? emptyTaskOptions() : getTaskOptions(workspaceCode)
  ));

  useEffect(() => {
    if (dataSource !== "api") {
      setOptions(getTaskOptions(workspaceCode));
      return;
    }

    if (!workspaceCode) {
      setOptions(emptyTaskOptions());
      return;
    }

    const controller = new AbortController();
    fetchTaskOptions(workspaceCode, controller.signal)
      .then(setOptions)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Task options request failed.", error);
        setOptions(emptyTaskOptions());
      });

    return () => controller.abort();
  }, [dataSource, workspaceCode]);

  return options;
}

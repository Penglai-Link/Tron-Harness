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
    defaultStatuses: {
      bug: "Open",
      featureChild: "Open",
      featureRoot: "Open",
    },
    priorities: [],
    sourceTypes: [],
    statusRules: {
      bug: [],
      featureChild: [],
      featureRoot: [],
    },
    statuses: [],
    taskTypes: [],
  };
}

export function useTaskOptions(): TaskOptions {
  const dataSource = getServiceDataSource("task");
  const [options, setOptions] = useState(() => (
    dataSource === "api" ? emptyTaskOptions() : getTaskOptions()
  ));

  useEffect(() => {
    if (dataSource !== "api") {
      setOptions(getTaskOptions());
      return;
    }

    const controller = new AbortController();
    fetchTaskOptions(controller.signal)
      .then(setOptions)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Task options request failed.", error);
        setOptions(emptyTaskOptions());
      });

    return () => controller.abort();
  }, [dataSource]);

  return options;
}

import {
  useEffect,
  useState,
} from "react";

import { fetchTaskOptions } from "@/domain/task/api";
import { getTaskOptions } from "@/domain/task/mock/data";
import type { TaskOptions } from "@/domain/task/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function useTaskOptions(): TaskOptions {
  const dataSource = getServiceDataSource("task");
  const [options, setOptions] = useState(() => getTaskOptions());

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
        console.warn("[task] Falling back to mock task options.", error);
        setOptions(getTaskOptions());
      });

    return () => controller.abort();
  }, [dataSource]);

  return options;
}

import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";

import {
  fetchTaskKanban,
  type TaskListQuery,
} from "@/domain/task/api";
import { listKanbanColumns } from "@/domain/task/mock/data";
import type { KanbanColumn } from "@/domain/task/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface TaskKanbanResult {
  columns: KanbanColumn[];
  loading: boolean;
  refresh: () => void;
}

export function useTaskKanbanColumns(query: TaskListQuery): TaskKanbanResult {
  const dataSource = getServiceDataSource("task");
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const mockColumns = useMemo(() => listKanbanColumns(query), [query]);
  const [columns, setColumns] = useState(mockColumns);
  const queryKey = JSON.stringify(query);

  useEffect(() => {
    if (dataSource === "api") return;
    setColumns(mockColumns);
  }, [dataSource, mockColumns]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);

    fetchTaskKanban(query, controller.signal)
      .then(setColumns)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[task] Falling back to mock kanban data.", error);
        setColumns(mockColumns);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, mockColumns, queryKey, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    columns,
    loading,
    refresh,
  };
}

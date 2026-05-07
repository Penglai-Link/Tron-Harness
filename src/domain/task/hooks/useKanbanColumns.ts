import { listKanbanColumns } from "@/domain/task/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { taskApiEndpoints } from "@/domain/task/api";

export function useKanbanColumns() {
  return useServiceQuery({
    domain: "task",
    endpoint: taskApiEndpoints.listKanbanColumns,
    mockData: () => listKanbanColumns(),
    deps: [],
  });
}

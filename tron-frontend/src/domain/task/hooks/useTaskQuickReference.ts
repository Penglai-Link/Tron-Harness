import { getTaskQuickReference } from "@/domain/task/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { taskApiEndpoints } from "@/domain/task/api";

export function useTaskQuickReference() {
  return useServiceQuery({
    domain: "task",
    endpoint: taskApiEndpoints.getTaskQuickReference,
    mockData: () => getTaskQuickReference(),
    deps: [],
  });
}

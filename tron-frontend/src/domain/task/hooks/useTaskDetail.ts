import { getTaskDetail } from "@/domain/task/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { taskApiEndpoints } from "@/domain/task/api";

export function useTaskDetail(taskId = "AGT-101") {
  return useServiceQuery({
    domain: "task",
    endpoint: taskApiEndpoints.getTaskDetail,
    mockData: () => getTaskDetail(),
    deps: [taskId],
    params: { taskId },
  });
}

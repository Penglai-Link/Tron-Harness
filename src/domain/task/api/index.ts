import {
  getTaskDetail,
  getTaskQuickReference,
  listKanbanColumns,
} from "@/domain/task/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
export type {
  StatusTone
} from "@/domain/task/constants/StatusTone";
export type {
  QuickReference,
  TaskDetailData,
  KanbanTask,
  KanbanColumn
} from "@/domain/task/model/TaskModel";

export const taskApiEndpoints = {
  getTaskDetail: "GET /api/v1/tasks/{taskId}",
  listKanbanColumns: "GET /api/v1/tasks/kanban-columns",
  getTaskQuickReference: "GET /api/v1/tasks/quick-reference",
} as const;

export function useTaskQuickReference() {
  return useServiceQuery({
    domain: "task",
    endpoint: taskApiEndpoints.getTaskQuickReference,
    mockData: () => getTaskQuickReference(),
    deps: [],
  });
}

export function useTaskDetail(taskId = "AGT-101") {
  return useServiceQuery({
    domain: "task",
    endpoint: taskApiEndpoints.getTaskDetail,
    mockData: () => getTaskDetail(),
    deps: [taskId],
    params: { taskId },
  });
}

export function useKanbanColumns() {
  return useServiceQuery({
    domain: "task",
    endpoint: taskApiEndpoints.listKanbanColumns,
    mockData: () => listKanbanColumns(),
    deps: [],
  });
}

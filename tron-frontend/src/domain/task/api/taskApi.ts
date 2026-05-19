import { requestApi } from "@/shared/api/httpClient";
import type { CreateTaskPayload, KanbanColumn, TaskDetailData, TaskLifecycleResult, TaskListItem, TaskOptions, TaskPage, UpdateTaskPayload, UpdateTaskStatusPayload } from "@/domain/task/model";

export const taskApiEndpoints = { archiveTask: "POST /api/v1/tasks/{taskCode}/archive", createTask: "POST /api/v1/tasks", getTaskDetail: "GET /api/v1/tasks/{taskCode}", getTaskOptions: "GET /api/v1/tasks/options", getTaskQuickReference: "GET /api/v1/tasks/quick-reference", listKanbanColumns: "GET /api/v1/tasks/kanban", listTasks: "GET /api/v1/tasks", restoreTask: "POST /api/v1/tasks/{taskCode}/restore", updateTask: "PATCH /api/v1/tasks/{taskCode}", updateTaskStatus: "PATCH /api/v1/tasks/{taskCode}/status" } as const;

type TaskQueryParams = Record<string, string | number | boolean | null | undefined>;
export interface TaskListQuery { assigneeUserCode?: string; current?: number; includeArchived?: boolean; parentTaskCode?: string; priority?: string; search?: string; size?: number; sourceType?: string; status?: string; taskType?: string; workspaceCode: string; }
function taskQueryParams(query: TaskListQuery): TaskQueryParams { return { ...query }; }
export function fetchTaskOptions(signal?: AbortSignal) { return requestApi<TaskOptions>(taskApiEndpoints.getTaskOptions, { signal }); }
export function fetchTaskKanban(query: TaskListQuery, signal?: AbortSignal) { return requestApi<KanbanColumn[]>(taskApiEndpoints.listKanbanColumns, { queryParams: taskQueryParams(query), signal }); }
export function fetchTaskPage(query: TaskListQuery, signal?: AbortSignal) { return requestApi<TaskPage<TaskListItem>>(taskApiEndpoints.listTasks, { queryParams: taskQueryParams(query), signal }); }
export function fetchTaskDetail(taskCode: string, signal?: AbortSignal) { return requestApi<TaskDetailData>(taskApiEndpoints.getTaskDetail, { pathParams: { taskCode }, signal }); }
export function createTask(payload: CreateTaskPayload) { return requestApi<TaskDetailData>(taskApiEndpoints.createTask, { body: payload }); }
export function updateTask(taskCode: string, payload: UpdateTaskPayload) { return requestApi<TaskDetailData>(taskApiEndpoints.updateTask, { body: payload, pathParams: { taskCode } }); }
export function updateTaskStatus(taskCode: string, payload: UpdateTaskStatusPayload) { return requestApi<TaskDetailData>(taskApiEndpoints.updateTaskStatus, { body: payload, pathParams: { taskCode } }); }
export function archiveTask(taskCode: string) { return requestApi<TaskLifecycleResult>(taskApiEndpoints.archiveTask, { pathParams: { taskCode } }); }
export function restoreTask(taskCode: string) { return requestApi<TaskLifecycleResult>(taskApiEndpoints.restoreTask, { pathParams: { taskCode } }); }

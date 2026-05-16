export type TaskStatus =
  | "Backlog"
  | "Todo"
  | "InProgress"
  | "Testing"
  | "Review"
  | "Reopen"
  | "Completed"
  | "Archived";

export type TaskPriority = "Low" | "Medium" | "High";
export type TaskType = "Feature" | "BugFix" | "Refactor";
export type TaskSourceType = "Internal" | "GitHub" | "GitLab";

export interface TaskOptions { priorities: TaskPriority[]; sourceTypes: TaskSourceType[]; statuses: TaskStatus[]; taskTypes: TaskType[]; }
export interface QuickReference { tips: string[]; variables: string[]; }
export interface TaskSummary { priority: TaskPriority; progress: number; status: TaskStatus; taskCode: string; taskType: TaskType; title: string; }
export interface TaskDetailData { agentCodes: string[]; appCodes: string[]; archivedAt: string | null; assigneeUserCodes: string[]; createdAt: string; description: string | null; id: string; parent: TaskSummary | null; parentTaskCode: string | null; previousStatus: TaskStatus | null; priority: TaskPriority; progress: number; sourceType: TaskSourceType; sourceUrl: string | null; status: TaskStatus; subtasks: TaskSummary[]; taskCode: string; taskType: TaskType; title: string; updatedAt: string; workspaceCode: string; }
export interface TaskListItem { agentCodes: string[]; appCodes: string[]; archivedAt?: string | null; assigneeUserCodes: string[]; parentTaskCode: string | null; priority: TaskPriority; progress: number; sourceType: TaskSourceType; sourceUrl: string | null; status: TaskStatus; subtaskCount: number; taskCode: string; taskType: TaskType; title: string; updatedAt: string; workspaceCode: string; }
export interface KanbanColumn { status: TaskStatus; title: string; tasks: TaskListItem[]; }
export interface TaskPage<T> { current: number; records: T[]; size: number; total: number; }
export interface CreateTaskPayload { agentCodes: string[]; appCodes: string[]; assigneeUserCodes: string[]; description?: string | null; parentTaskCode?: string | null; priority: TaskPriority; sourceType: TaskSourceType; sourceUrl?: string | null; taskType: TaskType; title: string; workspaceCode: string; }
export type UpdateTaskPayload = Partial<Omit<CreateTaskPayload, "parentTaskCode" | "workspaceCode">>;
export interface UpdateTaskStatusPayload { progress?: number; status: TaskStatus; }
export interface TaskLifecycleResult { archivedAt: string | null; previousStatus: TaskStatus | null; status: TaskStatus; taskCode: string; }

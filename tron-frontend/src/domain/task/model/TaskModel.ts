export type TaskStatus =
  | "Open"
  | "Designing"
  | "Red"
  | "Green"
  | "Regressioning"
  | "Review"
  | "InProgress"
  | "Reopen"
  | "Completed"
  | "Archived";

export type TaskPriority = "Low" | "Medium" | "High";
export type TaskType = "Feature" | "Bug";
export type TaskSourceType = "Internal" | "GitHub" | "GitLab";
export type TaskRoleKey = "featureRoot" | "featureChild" | "bug";

export interface TaskStatusRules {
  bug: TaskStatus[];
  featureChild: TaskStatus[];
  featureRoot: TaskStatus[];
}

export interface TaskDefaultStatuses {
  bug: TaskStatus;
  featureChild: TaskStatus;
  featureRoot: TaskStatus;
}

export interface TaskOptions {
  defaultStatuses: TaskDefaultStatuses;
  priorities: TaskPriority[];
  sourceTypes: TaskSourceType[];
  statusRules: TaskStatusRules;
  statuses: TaskStatus[];
  taskTypes: TaskType[];
}

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

export function inferTaskRole(taskType: TaskType, parentTaskCode?: string | null): TaskRoleKey {
  if (taskType === "Bug") return "bug";
  return parentTaskCode ? "featureChild" : "featureRoot";
}

export function statusLabel(value: TaskStatus): string {
  if (value === "InProgress") return "In Progress";
  return value;
}

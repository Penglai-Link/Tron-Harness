import type { TaskListQuery } from "@/domain/task/api";
import {
  inferTaskRole,
  statusLabel,
  type CreateTaskPayload,
  type KanbanColumn,
  type QuickReference,
  type TaskDetailData,
  type TaskLifecycleResult,
  type TaskListItem,
  type TaskOptions,
  type TaskSummary,
  type TaskStatus,
  type UpdateTaskPayload,
  type UpdateTaskStatusPayload,
} from "@/domain/task/model";

const NOW = "2026-05-19T12:00:00Z";
const taskOptions: TaskOptions = {
  defaultStatuses: {
    bug: "Open",
    featureChild: "Open",
    featureRoot: "Open",
  },
  priorities: ["Low", "Medium", "High"],
  sourceTypes: ["Internal", "GitHub", "GitLab"],
  statusRules: {
    bug: ["Open", "InProgress", "Completed", "Reopen", "Archived"],
    featureChild: ["Open", "InProgress", "Completed", "Archived"],
    featureRoot: ["Open", "Designing", "Red", "Green", "Regressioning", "Review", "Completed", "Archived"],
  },
  statuses: ["Open", "Designing", "Red", "Green", "Regressioning", "Review", "InProgress", "Reopen", "Completed", "Archived"],
  taskTypes: ["Feature", "Bug"],
};

let nextTaskId = 1;
let tasks: TaskDetailData[] = [
  {
    agentCodes: [],
    appCodes: [],
    archivedAt: null,
    assigneeUserCodes: ["USR-AUTH-0001"],
    createdAt: NOW,
    description: "Capture the initial superpower intent and task boundary.",
    id: "734500000000000101",
    parent: null,
    parentTaskCode: null,
    previousStatus: null,
    priority: "Medium",
    progress: 0,
    sourceType: "Internal",
    sourceUrl: "",
    status: "Open",
    subtasks: [],
    taskCode: "TSK-SEED-FEATURE",
    taskType: "Feature",
    title: "Model task-management superpower flow",
    updatedAt: NOW,
    workspaceCode: "TRON_CORE",
  },
  {
    agentCodes: ["agent-reviewer"],
    appCodes: ["github"],
    archivedAt: null,
    assigneeUserCodes: ["USR-AUTH-0001"],
    createdAt: NOW,
    description: "Fix a board rendering gap under the new workflow.",
    id: "734500000000000102",
    parent: null,
    parentTaskCode: null,
    previousStatus: null,
    priority: "High",
    progress: 65,
    sourceType: "GitHub",
    sourceUrl: "https://github.com/clouds3n/tron/issues/42",
    status: "InProgress",
    subtasks: [],
    taskCode: "TSK-SEED-BUG",
    taskType: "Bug",
    title: "Fix task board workflow regression",
    updatedAt: NOW,
    workspaceCode: "TRON_CORE",
  },
  {
    agentCodes: [],
    appCodes: [],
    archivedAt: NOW,
    assigneeUserCodes: [],
    createdAt: NOW,
    description: "Archived example under the new flow.",
    id: "734500000000000103",
    parent: null,
    parentTaskCode: null,
    previousStatus: "Review",
    priority: "Low",
    progress: 15,
    sourceType: "Internal",
    sourceUrl: "",
    status: "Archived",
    subtasks: [],
    taskCode: "TSK-SEED-ARCHIVED",
    taskType: "Feature",
    title: "Archived workflow example",
    updatedAt: NOW,
    workspaceCode: "TRON_CORE",
  },
];

export function getTaskOptions() { return taskOptions; }

export function getTaskQuickReference(): QuickReference {
  return {
    tips: [
      "Task status options are backend-owned and depend on task role.",
      "New tasks inherit the workspace selected in the shell header.",
      "Only effective workspace users can be assigned.",
    ],
    variables: ["taskType", "sourceType", "sourceUrl", "assigneeUserCodes"],
  };
}

export function listKanbanColumns(query: TaskListQuery): KanbanColumn[] {
  const records = filterTasks(query);
  const includeArchived = query.includeArchived === true;
  return taskOptions.statuses
    .filter((status) => includeArchived || status !== "Archived")
    .map((status) => ({
      status,
      title: statusLabel(status),
      tasks: records
        .filter((task) => task.status === status)
        .map(toListItem),
    }));
}

export function getTaskDetail(taskCode = "TSK-SEED-FEATURE") {
  return withRelationships(tasks.find((task) => task.taskCode === taskCode) ?? tasks[0]);
}

export function createMockTask(payload: CreateTaskPayload) {
  const taskCode = `TSK-MOCK-${nextTaskId++}`;
  const taskRole = inferTaskRole(payload.taskType, payload.parentTaskCode ?? null);
  const task: TaskDetailData = {
    agentCodes: uniqueCodes(payload.agentCodes),
    appCodes: uniqueCodes(payload.appCodes),
    archivedAt: null,
    assigneeUserCodes: uniqueCodes(payload.assigneeUserCodes),
    createdAt: NOW,
    description: payload.description?.trim() ? payload.description : null,
    id: String(734500000000100000 + nextTaskId),
    parent: null,
    parentTaskCode: payload.parentTaskCode ?? null,
    previousStatus: null,
    priority: payload.priority,
    progress: 0,
    sourceType: payload.sourceType,
    sourceUrl: payload.sourceUrl?.trim() ?? "",
    status: taskOptions.defaultStatuses[taskRole],
    subtasks: [],
    taskCode,
    taskType: payload.taskType,
    title: payload.title,
    updatedAt: NOW,
    workspaceCode: payload.workspaceCode,
  };
  tasks = [task, ...tasks];
  return getTaskDetail(taskCode);
}

export function updateMockTask(taskCode: string, payload: UpdateTaskPayload) {
  const current = getTaskDetail(taskCode);
  const nextTaskType = payload.taskType ?? current.taskType;
  const nextRole = inferTaskRole(nextTaskType, current.parentTaskCode);
  if (!taskOptions.statusRules[nextRole].includes(current.status)) {
    throw new Error("task status does not match the inferred task role");
  }
  tasks = tasks.map((task) => task.taskCode !== taskCode || task.status === "Archived" ? task : {
    ...task,
    ...payload,
    agentCodes: payload.agentCodes ? uniqueCodes(payload.agentCodes) : task.agentCodes,
    appCodes: payload.appCodes ? uniqueCodes(payload.appCodes) : task.appCodes,
    assigneeUserCodes: payload.assigneeUserCodes ? uniqueCodes(payload.assigneeUserCodes) : task.assigneeUserCodes,
    description: payload.description === undefined ? task.description : payload.description,
    sourceUrl: payload.sourceUrl === undefined ? task.sourceUrl : payload.sourceUrl,
    updatedAt: NOW,
  });
  return getTaskDetail(taskCode);
}

export function updateMockTaskStatus(taskCode: string, payload: UpdateTaskStatusPayload) {
  const current = getTaskDetail(taskCode);
  const taskRole = inferTaskRole(current.taskType, current.parentTaskCode);
  if (!taskOptions.statusRules[taskRole].includes(payload.status)) {
    throw new Error("task status does not match the inferred task role");
  }
  tasks = tasks.map((task) => task.taskCode !== taskCode || task.status === "Archived" ? task : {
    ...task,
    progress: payload.status === "Completed" ? 100 : payload.progress ?? task.progress,
    status: payload.status,
    updatedAt: NOW,
  });
  return getTaskDetail(taskCode);
}

export function archiveMockTask(taskCode: string): TaskLifecycleResult {
  const hasActiveSubtasks = tasks.some((task) => task.parentTaskCode === taskCode && task.status !== "Archived");
  if (hasActiveSubtasks) throw new Error("archive active subtasks before archiving the parent task");
  tasks = tasks.map((task) => task.taskCode === taskCode && task.status !== "Archived" ? {
    ...task,
    archivedAt: NOW,
    previousStatus: task.status,
    status: "Archived",
    updatedAt: NOW,
  } : task);
  const task = getTaskDetail(taskCode);
  return { archivedAt: task.archivedAt, previousStatus: task.previousStatus, status: task.status, taskCode };
}

export function restoreMockTask(taskCode: string): TaskLifecycleResult {
  tasks = tasks.map((task) => {
    if (task.taskCode !== taskCode || task.status !== "Archived") return task;
    const taskRole = inferTaskRole(task.taskType, task.parentTaskCode);
    const nextStatus = task.previousStatus && taskOptions.statusRules[taskRole].includes(task.previousStatus)
      ? task.previousStatus
      : taskOptions.defaultStatuses[taskRole];
    return {
      ...task,
      archivedAt: null,
      previousStatus: null,
      status: nextStatus,
      updatedAt: NOW,
    };
  });
  const task = getTaskDetail(taskCode);
  return { archivedAt: task.archivedAt, previousStatus: task.previousStatus, status: task.status, taskCode };
}

function filterTasks(query: TaskListQuery) {
  const includeArchived = query.includeArchived === true;
  const search = query.search?.trim().toLowerCase();
  return tasks.filter((task) => {
    if (query.workspaceCode && task.workspaceCode !== query.workspaceCode) return false;
    if (!includeArchived && task.status === "Archived") return false;
    if (query.status && task.status !== query.status) return false;
    if (query.priority && task.priority !== query.priority) return false;
    if (query.taskType && task.taskType !== query.taskType) return false;
    if (query.sourceType && task.sourceType !== query.sourceType) return false;
    if (query.assigneeUserCode && !task.assigneeUserCodes.includes(query.assigneeUserCode)) return false;
    if (query.parentTaskCode === "__ROOT__" && task.parentTaskCode) return false;
    if (query.parentTaskCode && query.parentTaskCode !== "__ROOT__" && task.parentTaskCode !== query.parentTaskCode) return false;
    if (!search) return true;
    return [task.taskCode, task.title, task.description ?? "", task.workspaceCode, task.sourceUrl ?? ""].join(" ").toLowerCase().includes(search);
  });
}

function effectiveProgress(task: TaskDetailData): number {
  const activeSubtasks = tasks.filter((candidate) => candidate.parentTaskCode === task.taskCode && candidate.status !== "Archived");
  if (!activeSubtasks.length) return task.progress;
  const total = activeSubtasks.reduce((sum, candidate) => sum + effectiveProgress(candidate), 0);
  return Math.round(total / activeSubtasks.length);
}

function toListItem(task: TaskDetailData): TaskListItem {
  return {
    agentCodes: task.agentCodes,
    appCodes: task.appCodes,
    archivedAt: task.archivedAt,
    assigneeUserCodes: task.assigneeUserCodes,
    parentTaskCode: task.parentTaskCode,
    priority: task.priority,
    progress: effectiveProgress(task),
    sourceType: task.sourceType,
    sourceUrl: task.sourceUrl,
    status: task.status,
    subtaskCount: tasks.filter((candidate) => candidate.parentTaskCode === task.taskCode && candidate.status !== "Archived").length,
    taskCode: task.taskCode,
    taskType: task.taskType,
    title: task.title,
    updatedAt: task.updatedAt,
    workspaceCode: task.workspaceCode,
  };
}

function summary(task: TaskDetailData): TaskSummary {
  return {
    priority: task.priority,
    progress: effectiveProgress(task),
    status: task.status,
    taskCode: task.taskCode,
    taskType: task.taskType,
    title: task.title,
  };
}

function withRelationships(task: TaskDetailData): TaskDetailData {
  const parent = task.parentTaskCode ? tasks.find((candidate) => candidate.taskCode === task.parentTaskCode) : undefined;
  const subtasks = tasks.filter((candidate) => candidate.parentTaskCode === task.taskCode && candidate.status !== "Archived");
  return {
    ...task,
    progress: effectiveProgress(task),
    parent: parent ? summary(parent) : null,
    subtasks: subtasks.map(summary),
  };
}

function uniqueCodes(codes: string[]) {
  return Array.from(new Set(codes.map((code) => code.trim()).filter(Boolean)));
}

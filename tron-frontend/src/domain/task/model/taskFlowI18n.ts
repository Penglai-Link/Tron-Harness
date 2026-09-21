import type { TaskFlow, TaskFlowStatus } from "./TaskModel";

export type TaskFlowTranslator = (key: string, defaultValue: string) => string;

const DEFAULT_FLOW_CODE = "DEFAULT";
const DEFAULT_FLOW_NAME = "默认流程";
const DEFAULT_FLOW_STATUSES: Record<string, { defaultValue: string; sourceName: string }> = {
  CLOSED: { defaultValue: "Closed", sourceName: "关闭" },
  DESIGNING: { defaultValue: "Designing", sourceName: "设计中" },
  GREEN: { defaultValue: "Verifying", sourceName: "验证中" },
  IN_PROGRESS: { defaultValue: "In Progress", sourceName: "进行中" },
  NOT_STARTED: { defaultValue: "Not Started", sourceName: "未开始" },
  RED: { defaultValue: "Developing", sourceName: "开发中" },
  REGRESSIONING: { defaultValue: "Regression", sourceName: "回归中" },
  REOPEN: { defaultValue: "Reopened", sourceName: "重新打开" },
  REVIEW: { defaultValue: "Review", sourceName: "评审中" },
};

const FLOW_ERROR_KEYS: Record<string, { defaultValue: string; key: string }> = {
  TASK_FLOW_CODE_CONFLICT: { defaultValue: "Task flow code already exists", key: "errors.codeConflict" },
  TASK_FLOW_IN_USE: { defaultValue: "Task flow is in use", key: "errors.inUse" },
  TASK_FLOW_NOT_FOUND: { defaultValue: "Task flow was not found", key: "errors.notFound" },
  TASK_FLOW_SWITCH_REQUIRES_CLOSED_TASKS: { defaultValue: "Every task must be closed before switching the workflow", key: "errors.switchRequiresClosedTasks" },
  TASK_REQUEST_INVALID: { defaultValue: "Task flow request is invalid", key: "errors.invalid" },
  TASK_STATUS_FLOW_INVALID: { defaultValue: "Task status is unavailable in the current workflow", key: "errors.statusNotInFlow" },
};

const MOCK_FLOW_ERROR_KEYS: Record<string, string> = {
  "任务流不存在": "errors.notFound",
  "任务流正在使用中": "errors.inUse",
  "任务流编码已存在": "errors.codeConflict",
  "任务流编码或名称无效": "errors.invalid",
  "任务状态不属于当前工作流": "errors.statusNotInFlow",
  "所有任务关闭后才能切换工作流": "errors.switchRequiresClosedTasks",
  "状态编码或名称无效": "errors.invalid",
  "默认任务流不能删除": "errors.inUse",
  "task flow was not found": "errors.notFound",
};

export function taskFlowDisplayName(flow: Pick<TaskFlow, "flowCode" | "name">, translate: TaskFlowTranslator) {
  if (flow.flowCode === DEFAULT_FLOW_CODE && flow.name === DEFAULT_FLOW_NAME) {
    return translate("defaultFlow.name", "Default flow");
  }
  return flow.name;
}

export function taskFlowStatusDisplayName(
  status: Pick<TaskFlowStatus, "name" | "statusCode">,
  translate: TaskFlowTranslator,
  flowCode?: string,
) {
  if (status.statusCode === "NOT_STARTED" || status.statusCode === "CLOSED") {
    return translate(`statuses.${status.statusCode}`, DEFAULT_FLOW_STATUSES[status.statusCode].defaultValue);
  }

  const defaultStatus = DEFAULT_FLOW_STATUSES[status.statusCode];
  if (flowCode === DEFAULT_FLOW_CODE && defaultStatus?.sourceName === status.name) {
    return translate(`defaultFlow.statuses.${status.statusCode}`, defaultStatus.defaultValue);
  }
  return status.name;
}

export function taskFlowErrorMessage(
  error: unknown,
  translate: TaskFlowTranslator,
  fallbackKey: string,
  fallback: string,
) {
  if (typeof error === "object" && error !== null && "code" in error && typeof error.code === "string") {
    const known = FLOW_ERROR_KEYS[error.code];
    if (known) return translate(known.key, known.defaultValue);
  }

  const message = error instanceof Error ? error.message : "";
  const key = MOCK_FLOW_ERROR_KEYS[message];
  return key ? translate(key, fallback) : message || translate(fallbackKey, fallback);
}

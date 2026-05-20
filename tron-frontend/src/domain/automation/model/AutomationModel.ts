export type AutomationStatus = "Active" | "Paused" | "Archived";
export type AutomationOperationalStatus = Exclude<AutomationStatus, "Archived">;
export type AutomationExecutionStatus =
  | "Pending"
  | "Running"
  | "Success"
  | "Failed"
  | "Canceled"
  | "Skipped";
export type AutomationTriggerSource = "Scheduler" | "Manual";

export interface AutomationOptions {
  defaultStatus: AutomationStatus;
  executionStatuses: AutomationExecutionStatus[];
  statuses: AutomationStatus[];
  timezones: string[];
}

export interface AutomationListItem {
  archivedAt: string | null;
  automationCode: string;
  cronExpression: string;
  description: string | null;
  lastExecutionCode: string | null;
  lastExecutionStatus: AutomationExecutionStatus | null;
  lastExecutionSummary: string | null;
  lastFinishedAt: string | null;
  lastStartedAt: string | null;
  name: string;
  nextRunAt: string | null;
  status: AutomationStatus;
  timezone: string;
  updatedAt: string;
  workspaceCode: string;
}

export interface AutomationDetailData {
  archivedAt: string | null;
  automationCode: string;
  createdAt: string;
  cronExpression: string;
  description: string | null;
  id: string;
  lastExecutionCode: string | null;
  lastExecutionStatus: AutomationExecutionStatus | null;
  lastExecutionSummary: string | null;
  lastFinishedAt: string | null;
  lastStartedAt: string | null;
  name: string;
  nextRunAt: string | null;
  previousStatus: AutomationStatus | null;
  prompt: string;
  schedulerExpression: string;
  status: AutomationStatus;
  timezone: string;
  updatedAt: string;
  workspaceCode: string;
}

export interface AutomationExecutionListItem {
  automationCode: string;
  createdAt: string;
  executionCode: string;
  finishedAt: string | null;
  scheduledAt: string | null;
  startedAt: string;
  status: AutomationExecutionStatus;
  summary: string | null;
  triggerSource: AutomationTriggerSource;
  workspaceCode: string;
}

export interface AutomationPage<T> {
  current: number;
  records: T[];
  size: number;
  total: number;
}

export interface CreateAutomationPayload {
  cronExpression: string;
  description?: string | null;
  name: string;
  prompt: string;
  status?: AutomationOperationalStatus;
  workspaceCode: string;
}

export interface UpdateAutomationPayload {
  cronExpression?: string;
  description?: string | null;
  name?: string;
  prompt?: string;
}

export interface UpdateAutomationStatusPayload {
  status: AutomationOperationalStatus;
}

export interface AutomationLifecycleResult {
  archivedAt: string | null;
  automationCode: string;
  previousStatus: AutomationStatus | null;
  status: AutomationStatus;
}

export interface AutomationListQuery {
  current?: number;
  includeArchived?: boolean;
  search?: string;
  size?: number;
  status?: AutomationStatus;
  workspaceCode: string;
}

export interface AutomationExecutionListQuery {
  current?: number;
  size?: number;
}

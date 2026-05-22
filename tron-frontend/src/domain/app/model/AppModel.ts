import type { BadgeProps } from "@/shared/ui/Badge";

export type AppRuntimeRole = "Frontend" | "Backend";
export type AppStatus = "Active" | "Archived";

export interface AppListItem {
  appCode: string;
  workspaceCode: string;
  name: string;
  description: string | null;
  localDirectory: string;
  runtimeRole: AppRuntimeRole;
  repositoryUrl: string | null;
  defaultBranch: string | null;
  tags: string[];
  status: AppStatus;
  updatedAt: string;
  archivedAt: string | null;
}

export interface AppDetailData extends AppListItem {
  id: string;
  createdAt: string;
}

export interface AppOption {
  appCode: string;
  name: string;
  description: string | null;
  runtimeRole: AppRuntimeRole;
}

export interface AppLifecycleResult {
  appCode: string;
  status: AppStatus;
  archivedAt: string | null;
}

export interface AppPage<T> {
  current: number;
  size: number;
  total: number;
  records: T[];
}

export interface AppListQuery {
  current?: number;
  includeArchived?: boolean;
  runtimeRole?: AppRuntimeRole;
  search?: string;
  size?: number;
  status?: AppStatus;
  workspaceCode: string;
}

export interface AppOptionQuery {
  runtimeRole?: AppRuntimeRole;
  search?: string;
  workspaceCode: string;
}

export interface CreateAppPayload {
  defaultBranch?: string | null;
  description?: string | null;
  localDirectory: string;
  name: string;
  repositoryUrl?: string | null;
  runtimeRole: AppRuntimeRole;
  tags: string[];
}

export type UpdateAppPayload = CreateAppPayload;

export function appStatusTone(status: AppStatus): BadgeProps["tone"] {
  return status === "Archived" ? "warning" : "success";
}

export function appRuntimeRoleTone(runtimeRole: AppRuntimeRole): BadgeProps["tone"] {
  return runtimeRole === "Frontend" ? "accent" : "neutral";
}

export function formatAppTimestamp(value: string | null) {
  if (!value) return "Not archived";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

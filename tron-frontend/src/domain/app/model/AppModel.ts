import type { BadgeProps } from "@/components/ui/badge";

export type AppRuntimeRole = "Frontend" | "Backend" | "Mono";
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

export function appStatusTone(status: AppStatus): BadgeProps["variant"] {
  return status === "Archived" ? "warning" : "success";
}

export function appRuntimeRoleTone(runtimeRole: AppRuntimeRole): BadgeProps["variant"] {
  if (runtimeRole === "Frontend") return "info";
  if (runtimeRole === "Mono") return "success";
  return "outline";
}

export function formatAppTimestamp(value: string | null, locale = "en-US", emptyLabel = "Not archived") {
  if (!value) return emptyLabel;
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

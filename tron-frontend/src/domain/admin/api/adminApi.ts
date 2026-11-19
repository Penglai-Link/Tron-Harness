import type {
  AuditLogRecord,
  WorkspaceDetailRecord,
  WorkspaceStatus,
  WorkspaceTreeNodeRecord,
} from "@/domain/admin/model";
import type { WorkspaceOption, WorkspaceSwitcherData } from "@/shared/types/WorkspaceSpace";
import { requestApi } from "@/shared/api/httpClient";

export const adminApiEndpoints = {
  listAuditLogs: "GET /api/v1/admin/audit-logs",
  listPermissionDomains: "GET /api/v1/admin/permission-domains",
  listWorkspaces: "GET /api/v1/admin/workspaces",
  listWorkspaceOptions: "GET /api/v1/admin/workspaces/options",
  getWorkspaceSwitcher: "GET /api/v1/admin/workspaces/switcher",
  createWorkspace: "POST /api/v1/admin/workspaces",
  getWorkspace: "GET /api/v1/admin/workspaces/{workspaceCode}",
  updateWorkspace: "PATCH /api/v1/admin/workspaces/{workspaceCode}",
  moveWorkspace: "POST /api/v1/admin/workspaces/{workspaceCode}/move",
  archiveWorkspace: "POST /api/v1/admin/workspaces/{workspaceCode}/archive",
  restoreWorkspace: "POST /api/v1/admin/workspaces/{workspaceCode}/restore",
  deleteWorkspace: "DELETE /api/v1/admin/workspaces/{workspaceCode}",
} as const;

type AdminEndpointParams = Record<string, string | number | boolean | null | undefined>;

interface AdminRequestOptions {
  body?: unknown;
  pathParams?: AdminEndpointParams;
  queryParams?: AdminEndpointParams;
  signal?: AbortSignal;
}

export interface WorkspaceListQuery {
  includeArchived?: boolean;
  search?: string;
  status?: WorkspaceStatus;
}

export interface WorkspaceLifecycleResult {
  affectedWorkspaceCount?: number;
  archivedAt?: string | null;
  status?: WorkspaceStatus;
  workspaceCode?: string;
}

export interface CreateWorkspacePayload {
  name: string;
  description?: string | null;
  parentWorkspaceCode?: string | null;
}

export interface UpdateWorkspacePayload {
  name: string;
  description?: string | null;
}

export interface WorkspaceDeletionResult {
  workspaceCode?: string;
  affectedWorkspaceCount?: number;
  deletedSessionCount?: number;
}

export interface MoveWorkspacePayload {
  parentWorkspaceCode?: string | null;
}

export async function requestAdminApi<T>(endpoint: string, options: AdminRequestOptions = {}): Promise<T> {
  return requestApi<T>(endpoint, {
    body: options.body as Record<string, unknown> | unknown[] | undefined,
    pathParams: options.pathParams,
    queryParams: options.queryParams,
    signal: options.signal,
  });
}

export function fetchWorkspaceTree(query: WorkspaceListQuery = {}, signal?: AbortSignal) {
  return requestAdminApi<WorkspaceTreeNodeRecord[]>(adminApiEndpoints.listWorkspaces, {
    queryParams: { includeArchived: query.includeArchived, search: query.search ?? "", status: query.status },
    signal,
  });
}

export function fetchWorkspaceOptions(query: WorkspaceListQuery = {}, signal?: AbortSignal) {
  return requestAdminApi<WorkspaceOption[]>(adminApiEndpoints.listWorkspaceOptions, {
    queryParams: { includeArchived: query.includeArchived, search: query.search ?? "", status: query.status },
    signal,
  });
}

export function fetchWorkspaceSwitcher(signal?: AbortSignal) {
  return requestAdminApi<WorkspaceSwitcherData>(adminApiEndpoints.getWorkspaceSwitcher, { signal });
}

export function createWorkspace(payload: CreateWorkspacePayload) {
  return requestAdminApi<WorkspaceDetailRecord>(adminApiEndpoints.createWorkspace, { body: payload });
}

export function fetchWorkspaceDetail(workspaceCode: string, signal?: AbortSignal) {
  return requestAdminApi<WorkspaceDetailRecord>(adminApiEndpoints.getWorkspace, {
    pathParams: { workspaceCode },
    signal,
  });
}

export function updateWorkspace(workspaceCode: string, payload: UpdateWorkspacePayload) {
  return requestAdminApi<WorkspaceDetailRecord>(adminApiEndpoints.updateWorkspace, {
    body: payload,
    pathParams: { workspaceCode },
  });
}

export function moveWorkspace(workspaceCode: string, payload: MoveWorkspacePayload) {
  return requestAdminApi<WorkspaceDetailRecord>(adminApiEndpoints.moveWorkspace, {
    body: payload,
    pathParams: { workspaceCode },
  });
}

export function archiveWorkspace(workspaceCode: string) {
  return requestAdminApi<WorkspaceLifecycleResult>(adminApiEndpoints.archiveWorkspace, {
    pathParams: { workspaceCode },
  });
}

export function restoreWorkspace(workspaceCode: string) {
  return requestAdminApi<WorkspaceLifecycleResult>(adminApiEndpoints.restoreWorkspace, {
    pathParams: { workspaceCode },
  });
}

export function deleteWorkspace(workspaceCode: string) {
  return requestAdminApi<WorkspaceDeletionResult>(adminApiEndpoints.deleteWorkspace, {
    pathParams: { workspaceCode },
  });
}

export type { AuditLogRecord };

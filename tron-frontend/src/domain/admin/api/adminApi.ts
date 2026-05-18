import type {
  RoleRecord,
  UserRecord,
  UserPhone,
  UserRoleCode,
  UserStatistics,
  UserStatus,
  WorkspaceDetailRecord,
  WorkspaceEffectiveUserRecord,
  WorkspaceRoleAccessRecord,
  WorkspaceStatistics,
  WorkspaceTreeNodeRecord,
  WorkspaceUserAccessRecord,
  WorkspaceStatus,
} from "@/domain/admin/model";
import type {
  WorkspaceOption,
  WorkspaceSwitcherData,
} from "@/shared/types/WorkspaceSpace";
import { requestApi } from "@/shared/api/httpClient";

export const adminApiEndpoints = {
  listAuditLogs: "GET /api/v1/admin/audit-logs",
  listPermissionDomains: "GET /api/v1/admin/permission-domains",
  listRoles: "GET /api/v1/admin/roles",
  replaceRoles: "PUT /api/v1/admin/roles",
  listUsers: "GET /api/v1/admin/users",
  getTotalUsersStatistic: "GET /api/v1/admin/users/statistics/total-users",
  getActiveUsersStatistic: "GET /api/v1/admin/users/statistics/active-users",
  getRoleAssignmentsStatistic: "GET /api/v1/admin/users/statistics/role-assignments",
  getSpaceAssignmentsStatistic: "GET /api/v1/admin/users/statistics/space-assignments",
  createUser: "POST /api/v1/admin/users",
  getUser: "GET /api/v1/admin/users/{userCode}",
  updateUserProfile: "PATCH /api/v1/admin/users/{userCode}",
  archiveUser: "POST /api/v1/admin/users/{userCode}/archive",
  restoreUser: "POST /api/v1/admin/users/{userCode}/restore",
  replaceUserRoles: "PUT /api/v1/admin/users/{userCode}/roles",
  replaceUserSpaces: "PUT /api/v1/admin/users/{userCode}/spaces",
  resetUserPassword: "POST /api/v1/admin/users/{userCode}/password/reset",
  listWorkspaces: "GET /api/v1/admin/workspaces",
  listWorkspaceOptions: "GET /api/v1/admin/workspaces/options",
  getWorkspaceSwitcher: "GET /api/v1/admin/workspaces/switcher",
  getTotalWorkspacesStatistic: "GET /api/v1/admin/workspaces/statistics/total-workspaces",
  getActiveWorkspacesStatistic: "GET /api/v1/admin/workspaces/statistics/active-workspaces",
  getDirectUserGrantsStatistic: "GET /api/v1/admin/workspaces/statistics/direct-user-grants",
  getRoleGrantsStatistic: "GET /api/v1/admin/workspaces/statistics/role-grants",
  createWorkspace: "POST /api/v1/admin/workspaces",
  getWorkspace: "GET /api/v1/admin/workspaces/{workspaceCode}",
  updateWorkspace: "PATCH /api/v1/admin/workspaces/{workspaceCode}",
  moveWorkspace: "POST /api/v1/admin/workspaces/{workspaceCode}/move",
  archiveWorkspace: "POST /api/v1/admin/workspaces/{workspaceCode}/archive",
  restoreWorkspace: "POST /api/v1/admin/workspaces/{workspaceCode}/restore",
  listWorkspaceUsers: "GET /api/v1/admin/workspaces/{workspaceCode}/access/users",
  replaceWorkspaceUsers: "PUT /api/v1/admin/workspaces/{workspaceCode}/access/users",
  listWorkspaceRoles: "GET /api/v1/admin/workspaces/{workspaceCode}/access/roles",
  replaceWorkspaceRoles: "PUT /api/v1/admin/workspaces/{workspaceCode}/access/roles",
  listWorkspaceEffectiveUsers: "GET /api/v1/admin/workspaces/{workspaceCode}/access/effective-users",
} as const;

type AdminEndpointParams = Record<string, string | number | boolean | null | undefined>;

interface AdminRequestOptions {
  body?: unknown;
  pathParams?: AdminEndpointParams;
  queryParams?: AdminEndpointParams;
  signal?: AbortSignal;
}

export interface AdminPage<T> {
  current: number;
  size: number;
  total: number;
  records: T[];
}

export interface UserListQuery {
  current: number;
  search?: string;
  size: number;
  status?: UserStatus | "" | "All";
}

interface UserStatisticCount {
  count: number;
}

export interface CreateUserPayload {
  displayName: string;
  email: string;
  firstName: string;
  lastName: string;
  locale: string;
  phone?: UserPhone | null;
  roleCodes: UserRoleCode[];
  spaceCodes: string[];
  timezone: string;
  userCode?: string;
  username: string;
}

export interface UpdateUserProfilePayload {
  displayName: string;
  email: string;
  firstName: string;
  lastName: string;
  locale: string;
  phone?: UserPhone | null;
  timezone: string;
}

export interface ResetPasswordResult {
  passwordResetRequired?: boolean;
  temporaryPassword?: string;
  userCode?: string;
}

export interface ReplaceRolesResult {
  roleAssignments?: Array<{ roleCode?: string }>;
  roleCodes?: UserRoleCode[];
  updatedAt?: string;
  userCode?: string;
}

export interface ReplaceSpacesResult {
  spaceAssignments?: Array<{ spaceCode?: string }>;
  spaceCodes?: string[];
  updatedAt?: string;
  userCode?: string;
}

export interface ReplaceRolesCatalogPayload {
  roles: RoleRecord[];
}

export interface UserLifecycleResult {
  archivedAt?: string | null;
  status?: UserStatus;
  updatedAt?: string;
  userCode?: string;
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

export interface MoveWorkspacePayload {
  parentWorkspaceCode?: string | null;
}

export interface WorkspaceListQuery {
  includeArchived?: boolean;
  search?: string;
  status?: WorkspaceStatus;
}

export interface WorkspaceAccessQuery {
  current?: number;
  size?: number;
  search?: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function normalizeAdminPage<T>(payload: unknown, fallback: Pick<AdminPage<T>, "current" | "size">): AdminPage<T> {
  if (isRecord(payload) && Array.isArray(payload.records)) {
    return {
      current: Number(payload.current ?? fallback.current) || fallback.current,
      size: Number(payload.size ?? fallback.size) || fallback.size,
      total: Number(payload.total ?? payload.records.length) || 0,
      records: payload.records as T[],
    };
  }

  if (Array.isArray(payload)) {
    return {
      current: fallback.current,
      size: fallback.size,
      total: payload.length,
      records: payload as T[],
    };
  }

  return {
    current: fallback.current,
    size: fallback.size,
    total: 0,
    records: [],
  };
}

export async function requestAdminApi<T>(endpoint: string, options: AdminRequestOptions = {}): Promise<T> {
  return requestApi<T>(endpoint, {
    body: options.body as Record<string, unknown> | unknown[] | undefined,
    pathParams: options.pathParams,
    queryParams: options.queryParams,
    signal: options.signal,
  });
}

export async function fetchAdminUserPage(query: UserListQuery, signal?: AbortSignal): Promise<AdminPage<UserRecord>> {
  const statusQueryParams: AdminEndpointParams = query.status === "All"
    ? { includeArchived: true }
    : { status: query.status || "Active" };
  const payload = await requestAdminApi<unknown>(adminApiEndpoints.listUsers, {
    queryParams: {
      current: query.current,
      search: query.search ?? "",
      size: query.size,
      ...statusQueryParams,
    },
    signal,
  });

  return normalizeAdminPage<UserRecord>(payload, {
    current: query.current,
    size: query.size,
  });
}

async function fetchAdminUserStatisticCount(endpoint: string, signal?: AbortSignal) {
  const payload = await requestAdminApi<UserStatisticCount>(endpoint, { signal });
  return Number(payload.count) || 0;
}

export async function fetchAdminUserStatistics(signal?: AbortSignal): Promise<UserStatistics> {
  const [
    totalUsers,
    activeUsers,
    roleAssignments,
    spaceAssignments,
  ] = await Promise.all([
    fetchAdminUserStatisticCount(adminApiEndpoints.getTotalUsersStatistic, signal),
    fetchAdminUserStatisticCount(adminApiEndpoints.getActiveUsersStatistic, signal),
    fetchAdminUserStatisticCount(adminApiEndpoints.getRoleAssignmentsStatistic, signal),
    fetchAdminUserStatisticCount(adminApiEndpoints.getSpaceAssignmentsStatistic, signal),
  ]);

  return {
    activeUsers,
    roleAssignments,
    spaceAssignments,
    totalUsers,
  };
}

async function fetchWorkspaceStatisticCount(endpoint: string, signal?: AbortSignal) {
  const payload = await requestAdminApi<UserStatisticCount>(endpoint, { signal });
  return Number(payload.count) || 0;
}

export async function fetchWorkspaceTree(query: WorkspaceListQuery = {}, signal?: AbortSignal) {
  return requestAdminApi<WorkspaceTreeNodeRecord[]>(adminApiEndpoints.listWorkspaces, {
    queryParams: {
      includeArchived: query.includeArchived,
      search: query.search ?? "",
      status: query.status,
    },
    signal,
  });
}

export async function fetchWorkspaceOptions(query: WorkspaceListQuery = {}, signal?: AbortSignal) {
  return requestAdminApi<WorkspaceOption[]>(adminApiEndpoints.listWorkspaceOptions, {
    queryParams: {
      includeArchived: query.includeArchived,
      search: query.search ?? "",
      status: query.status,
    },
    signal,
  });
}

export async function fetchWorkspaceSwitcher(signal?: AbortSignal) {
  return requestAdminApi<WorkspaceSwitcherData>(adminApiEndpoints.getWorkspaceSwitcher, {
    signal,
  });
}

export async function fetchWorkspaceStatistics(signal?: AbortSignal): Promise<WorkspaceStatistics> {
  const [
    totalWorkspaces,
    activeWorkspaces,
    directUserGrants,
    roleGrants,
  ] = await Promise.all([
    fetchWorkspaceStatisticCount(adminApiEndpoints.getTotalWorkspacesStatistic, signal),
    fetchWorkspaceStatisticCount(adminApiEndpoints.getActiveWorkspacesStatistic, signal),
    fetchWorkspaceStatisticCount(adminApiEndpoints.getDirectUserGrantsStatistic, signal),
    fetchWorkspaceStatisticCount(adminApiEndpoints.getRoleGrantsStatistic, signal),
  ]);

  return {
    totalWorkspaces,
    activeWorkspaces,
    directUserGrants,
    roleGrants,
  };
}

export function createWorkspace(payload: CreateWorkspacePayload) {
  return requestAdminApi<WorkspaceDetailRecord>(adminApiEndpoints.createWorkspace, {
    body: payload,
  });
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

export function fetchWorkspaceUserAccess(
  workspaceCode: string,
  query: WorkspaceAccessQuery = {},
  signal?: AbortSignal,
) {
  return requestAdminApi<AdminPage<WorkspaceUserAccessRecord>>(adminApiEndpoints.listWorkspaceUsers, {
    pathParams: { workspaceCode },
    queryParams: {
      current: query.current ?? 1,
      size: query.size ?? 100,
      search: query.search ?? "",
    },
    signal,
  });
}

export function replaceWorkspaceUserAccess(
  workspaceCode: string,
  users: Array<{ userCode: string; isOwner: boolean }>,
) {
  return requestAdminApi<AdminPage<WorkspaceUserAccessRecord>>(adminApiEndpoints.replaceWorkspaceUsers, {
    body: { users },
    pathParams: { workspaceCode },
  });
}

export function fetchWorkspaceRoleAccess(workspaceCode: string, signal?: AbortSignal) {
  return requestAdminApi<WorkspaceRoleAccessRecord[]>(adminApiEndpoints.listWorkspaceRoles, {
    pathParams: { workspaceCode },
    signal,
  });
}

export function replaceWorkspaceRoleAccess(
  workspaceCode: string,
  roles: Array<{ roleCode: UserRoleCode }>,
) {
  return requestAdminApi<WorkspaceRoleAccessRecord[]>(adminApiEndpoints.replaceWorkspaceRoles, {
    body: { roles },
    pathParams: { workspaceCode },
  });
}

export function fetchWorkspaceEffectiveUsers(
  workspaceCode: string,
  query: WorkspaceAccessQuery = {},
  signal?: AbortSignal,
) {
  return requestAdminApi<AdminPage<WorkspaceEffectiveUserRecord>>(adminApiEndpoints.listWorkspaceEffectiveUsers, {
    pathParams: { workspaceCode },
    queryParams: {
      current: query.current ?? 1,
      size: query.size ?? 100,
      search: query.search ?? "",
    },
    signal,
  });
}

export function createAdminUser(payload: CreateUserPayload) {
  return requestAdminApi<UserRecord>(adminApiEndpoints.createUser, { body: payload });
}

export function fetchAdminUser(userCode: string, signal?: AbortSignal) {
  return requestAdminApi<UserRecord>(adminApiEndpoints.getUser, {
    pathParams: { userCode },
    signal,
  });
}

export function updateAdminUserProfile(userCode: string, payload: UpdateUserProfilePayload) {
  return requestAdminApi<UserRecord>(adminApiEndpoints.updateUserProfile, {
    body: payload,
    pathParams: { userCode },
  });
}

export function replaceAdminUserRoles(userCode: string, roleCodes: readonly string[]) {
  return requestAdminApi<ReplaceRolesResult>(adminApiEndpoints.replaceUserRoles, {
    body: { roleCodes },
    pathParams: { userCode },
  });
}

export function replaceAdminRoles(payload: ReplaceRolesCatalogPayload) {
  return requestAdminApi<RoleRecord[]>(adminApiEndpoints.replaceRoles, {
    body: payload,
  });
}

export function replaceAdminUserSpaces(userCode: string, spaceCodes: readonly string[]) {
  return requestAdminApi<ReplaceSpacesResult>(adminApiEndpoints.replaceUserSpaces, {
    body: { spaceCodes },
    pathParams: { userCode },
  });
}

export function resetAdminUserPassword(userCode: string) {
  return requestAdminApi<ResetPasswordResult>(adminApiEndpoints.resetUserPassword, {
    pathParams: { userCode },
  });
}

export function archiveAdminUser(userCode: string) {
  return requestAdminApi<UserLifecycleResult>(adminApiEndpoints.archiveUser, {
    pathParams: { userCode },
  });
}

export function restoreAdminUser(userCode: string) {
  return requestAdminApi<UserLifecycleResult>(adminApiEndpoints.restoreUser, {
    pathParams: { userCode },
  });
}

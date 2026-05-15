import type {
  UserRecord,
  UserPhone,
  UserRoleCode,
  UserStatistics,
  UserStatus,
} from "@/domain/admin/model";
import { requestApi } from "@/shared/api/httpClient";

export const adminApiEndpoints = {
  listAuditLogs: "GET /api/v1/admin/audit-logs",
  listPermissionDomains: "GET /api/v1/admin/permission-domains",
  listRoles: "GET /api/v1/admin/roles",
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

export interface UserLifecycleResult {
  archivedAt?: string | null;
  status?: UserStatus;
  updatedAt?: string;
  userCode?: string;
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

import {
  getAuditLogs,
  listPermissionDomains,
  listRoles,
  listUsers,
} from "@/domain/admin/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";

export type {
  AuditLogRecord,
  RoleRecord,
  UserRecord
} from "@/domain/admin/model/AdminModel";

export const adminApiEndpoints = {
  listAuditLogs: "GET /api/v1/admin/audit-logs",
  listPermissionDomains: "GET /api/v1/admin/permission-domains",
  listRoles: "GET /api/v1/admin/roles",
  listUsers: "GET /api/v1/admin/users",
} as const;

export function useAuditLogs() {
  return useServiceQuery({
    domain: "admin",
    endpoint: adminApiEndpoints.listAuditLogs,
    mockData: () => getAuditLogs(),
    deps: [],
  });
}

export function usePermissionDomains() {
  return useServiceQuery({
    domain: "admin",
    endpoint: adminApiEndpoints.listPermissionDomains,
    mockData: () => listPermissionDomains(),
    deps: [],
  });
}

export function useRoles() {
  return useServiceQuery({
    domain: "admin",
    endpoint: adminApiEndpoints.listRoles,
    mockData: () => listRoles(),
    deps: [],
  });
}

export function useUsers() {
  return useServiceQuery({
    domain: "admin",
    endpoint: adminApiEndpoints.listUsers,
    mockData: () => listUsers(),
    deps: [],
  });
}

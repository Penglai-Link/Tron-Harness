export const adminApiEndpoints = {
  listAuditLogs: "GET /api/v1/admin/audit-logs",
  listPermissionDomains: "GET /api/v1/admin/permission-domains",
  listRoles: "GET /api/v1/admin/roles",
  listUsers: "GET /api/v1/admin/users",
} as const;

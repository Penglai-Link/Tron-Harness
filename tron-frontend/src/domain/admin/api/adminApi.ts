export const adminApiEndpoints = {
  listAuditLogs: "GET /api/v1/admin/audit-logs",
  listPermissionDomains: "GET /api/v1/admin/permission-domains",
  listRoles: "GET /api/v1/admin/roles",
  listUsers: "GET /api/v1/admin/users",
  getUser: "GET /api/v1/admin/users/{userCode}",
  archiveUser: "POST /api/v1/admin/users/{userCode}/archive",
  restoreUser: "POST /api/v1/admin/users/{userCode}/restore",
  replaceUserRoles: "PUT /api/v1/admin/users/{userCode}/roles",
  replaceUserSpaces: "PUT /api/v1/admin/users/{userCode}/spaces",
  resetUserPassword: "POST /api/v1/admin/users/{userCode}/password/reset",
} as const;

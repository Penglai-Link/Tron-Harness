export const adminManagementTabs = [
  { label: "User Directory", value: "users" },
  { label: "Role Matrix", value: "roles" },
  { label: "Audit Logs", value: "audit" },
] as const;

export const adminUserRoleFilters = ["All", "Admin", "Editor", "Viewer"] as const;

export const adminUserRoleOptions = ["Admin", "Editor", "Viewer"] as const;

export const adminAuditStatusOptions = [
  { label: "All status", value: "all" },
  { label: "Success", value: "Success" },
  { label: "Warning", value: "Warning" },
  { label: "Failed", value: "Failed" },
] as const;

export const adminPermissionCodes = ["R", "W", "D", "A"] as const;

export const adminPermissionLegend = ["R Read", "W Write", "D Delete", "A Admin"] as const;

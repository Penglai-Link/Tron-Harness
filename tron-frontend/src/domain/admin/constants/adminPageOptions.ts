import type {
  UserRoleCode,
  UserStatus,
} from "@/domain/admin/model";

export const adminUserStatusOptions: UserStatus[] = ["Active", "Archived"];

export const adminUserRoleOptions: UserRoleCode[] = [
  "ADMIN",
  "WORKSPACE_ADMIN",
  "DEVELOPER",
  "VIEWER",
];

export const adminUserRoleFilters = ["All", ...adminUserRoleOptions] as const;

export const adminUserRoleLabels: Record<UserRoleCode, string> = {
  ADMIN: "Admin",
  WORKSPACE_ADMIN: "Workspace Admin",
  DEVELOPER: "Developer",
  VIEWER: "Viewer",
};

export function getAdminUserRoleLabel(roleCode: string) {
  return adminUserRoleLabels[roleCode as UserRoleCode] ?? roleCode;
}

export const adminAuditStatusOptions = [
  { label: "All status", value: "all" },
  { label: "Success", value: "Success" },
  { label: "Warning", value: "Warning" },
  { label: "Failed", value: "Failed" },
] as const;

export const adminPermissionCodes = ["R", "W", "D", "A"] as const;

export const adminPermissionLegend = ["R Read", "W Write", "D Delete", "A Admin"] as const;

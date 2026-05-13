// This file defines the data models for the admin domain, including audit logs, roles, and users.

/**
 * Represents a single audit log entry, capturing details about actions performed within the system.
 */
export interface AuditLogRecord {
  /**
   * The unique identifier for the audit log entry
   */
  id: string;
  /**
   * The timestamp when the action occurred
   */
  timestamp: string;
  /**
   * The user or system entity that performed the action
   */
  actor: string;
  /**
   * The action that was performed
   */
  action: string;
  /**
   * The resource that the action was performed on
   */
  resourceId: string;
  /**
   * The status of the action
   */
  status: "Success" | "Warning" | "Failed";
  /**
   * The IP address from which the action was performed
   */
  ipAddress: string;
}

/**
 * Represents a role within the system, including its permissions and description.
 */
export interface RoleRecord {
  id: string;
  name: string;
  description: string;
  permissions: Record<string, string[]>;
}

export type UserStatus = "Active" | "Archived";

export type UserRoleCode =
  | "ADMIN"
  | "WORKSPACE_ADMIN"
  | "DEVELOPER"
  | "VIEWER";

export interface UserPhone {
  countryCode: string;
  number: string;
}

/**
 * Represents the User Management API boundary. Role and space values are external stable codes only.
 */
export interface UserRecord {
  id: string;
  userCode: string;
  username: string;
  firstName: string;
  lastName: string;
  displayName: string;
  email: string;
  phone: UserPhone | null;
  avatarUrl: string | null;
  locale: string;
  timezone: string;
  roleCodes: UserRoleCode[];
  spaceCodes: string[];
  status: UserStatus;
  passwordResetRequired: boolean;
  lastActiveAt: string | null;
  createdAt: string;
  updatedAt: string;
  archivedAt: string | null;
}

export interface UserStatistics {
  activeUsers: number;
  roleAssignments: number;
  spaceAssignments: number;
  totalUsers: number;
}

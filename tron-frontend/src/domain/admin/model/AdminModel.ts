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

/**
 * Represents a user within the system, including role assignments, workspace memberships, status, and last active time.
 */
export interface UserRecord {
  id: string;
  name: string;
  email: string;
  roles: string[];
  workspaces: string[];
  status: string;
  lastActive: string;
}

import { permissionDomains } from "@/domain/admin/constants";
import type {
  AuditLogRecord,
  RoleRecord,
  UserRecord,
  UserStatistics,
} from "@/domain/admin/model/AdminModel";

const auditLogs: AuditLogRecord[] = [
  { id: "log-9131", timestamp: "2026-05-01 14:42:18", actor: "jdoe@tron.com", action: "Role updated", resourceId: "role.workspace.admin", status: "Success", ipAddress: "10.24.8.12" },
  { id: "log-9130", timestamp: "2026-05-01 14:39:02", actor: "asmith@tron.com", action: "Inheritance override", resourceId: "model.tron-opus-v2", status: "Warning", ipAddress: "10.24.9.77" },
  { id: "log-9129", timestamp: "2026-05-01 14:34:55", actor: "system_agent", action: "Secret rotated", resourceId: "vault.openai.primary", status: "Success", ipAddress: "127.0.0.1" },
  { id: "log-9128", timestamp: "2026-05-01 14:31:21", actor: "jdoe@tron.com", action: "Export requested", resourceId: "audit.batch.2451", status: "Success", ipAddress: "10.24.8.12" },
];

export function getAuditLogs() {
  return auditLogs;
}


const roles: RoleRecord[] = [
  {
    id: "admin",
    code: "ADMIN",
    name: "Admin",
    description: "Full System Access",
    kind: "System",
    managedBy: "Platform",
    permissions: Object.fromEntries(permissionDomains.map((domain) => [domain, ["R", "W", "D", "A"]])),
    updatedAt: "2026-05-15T00:00:00Z",
  },
  {
    id: "workspace-admin",
    code: "WORKSPACE_ADMIN",
    name: "Workspace Admin",
    description: "Workspace administration",
    kind: "System",
    managedBy: "Workspace",
    permissions: {
      Workspace: ["R", "W"],
      Models: ["R", "W"],
      Tasks: ["R", "W"],
      Memory: ["R"],
      Skills: ["R", "W"],
      MCP: ["R"],
    },
    updatedAt: "2026-05-15T00:00:00Z",
  },
  {
    id: "developer",
    code: "DEVELOPER",
    name: "Developer",
    description: "Build and operate applications",
    kind: "System",
    managedBy: "Platform",
    permissions: {
      Workspace: ["R"],
      Models: ["R", "W"],
      Tasks: ["R", "W"],
      Memory: ["R"],
      Skills: ["R", "W"],
      MCP: ["R"],
    },
    updatedAt: "2026-05-15T00:00:00Z",
  },
  {
    id: "viewer",
    code: "VIEWER",
    name: "Viewer",
    description: "Read-only Access",
    kind: "System",
    managedBy: "Platform",
    permissions: Object.fromEntries(permissionDomains.map((domain) => [domain, ["R"]])),
    updatedAt: "2026-05-15T00:00:00Z",
  },
];

const users: UserRecord[] = [
  {
    id: "734102998871252992",
    userCode: "USER_ALEX_JOHNSON",
    username: "alex.johnson",
    firstName: "Alex",
    lastName: "Johnson",
    displayName: "Alex Johnson",
    email: "alex@tron.ai",
    phone: { countryCode: "+1", number: "5550100" },
    avatarUrl: "https://cdn.tron.local/avatar/alex.png",
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["ADMIN", "DEVELOPER"],
    spaceCodes: ["ENTERPRISE", "PLATFORM"],
    status: "Active",
    passwordResetRequired: false,
    lastActiveAt: "2026-05-11T08:30:00Z",
    createdAt: "2026-05-11T08:00:00Z",
    updatedAt: "2026-05-11T08:00:00Z",
    archivedAt: null,
  },
  {
    id: "734103002743185408",
    userCode: "USER_MAYA_SINGH",
    username: "maya.singh",
    firstName: "Maya",
    lastName: "Singh",
    displayName: "Maya Singh",
    email: "maya@tron.ai",
    phone: { countryCode: "+1", number: "5550101" },
    avatarUrl: null,
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["DEVELOPER"],
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
    passwordResetRequired: true,
    lastActiveAt: "2026-05-11T08:14:00Z",
    createdAt: "2026-05-11T08:40:00Z",
    updatedAt: "2026-05-11T08:40:00Z",
    archivedAt: null,
  },
  {
    id: "9007199254740993",
    userCode: "USER_CHEN_WEI",
    username: "chen.wei",
    firstName: "Chen",
    lastName: "Wei",
    displayName: "Chen Wei",
    email: "chen@tron.ai",
    phone: null,
    avatarUrl: null,
    locale: "en-US",
    timezone: "Asia/Shanghai",
    roleCodes: ["VIEWER"],
    spaceCodes: ["ENTERPRISE", "DATA"],
    status: "Active",
    passwordResetRequired: false,
    lastActiveAt: null,
    createdAt: "2026-05-10T03:20:00Z",
    updatedAt: "2026-05-10T03:20:00Z",
    archivedAt: null,
  },
  {
    id: "734103112812822528",
    userCode: "USER_RINA_PATEL",
    username: "rina.patel",
    firstName: "Rina",
    lastName: "Patel",
    displayName: "Rina Patel",
    email: "rina@tron.ai",
    phone: { countryCode: "+1", number: "5550102" },
    avatarUrl: null,
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["WORKSPACE_ADMIN", "VIEWER"],
    spaceCodes: ["SUPPORT", "PLATFORM"],
    status: "Archived",
    passwordResetRequired: false,
    lastActiveAt: "2026-04-28T17:15:00Z",
    createdAt: "2026-04-12T10:00:00Z",
    updatedAt: "2026-05-08T19:00:00Z",
    archivedAt: "2026-05-08T19:00:00Z",
  },
];

export function listPermissionDomains() {
  return [...permissionDomains];
}

export function listRoles() {
  return roles;
}

export function listUsers() {
  return users;
}

export function getUserStatistics(): UserStatistics {
  return {
    activeUsers: users.filter((user) => user.status === "Active").length,
    roleAssignments: users.reduce((total, user) => total + user.roleCodes.length, 0),
    spaceAssignments: users.reduce((total, user) => total + user.spaceCodes.length, 0),
    totalUsers: users.length,
  };
}

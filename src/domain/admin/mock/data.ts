import { permissionDomains } from "@/domain/admin/constants";
import type {
  AuditLogRecord,
  RoleRecord,
  UserRecord,
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
    name: "Admin",
    description: "Full System Access",
    permissions: Object.fromEntries(permissionDomains.map((domain) => [domain, ["R", "W", "D", "A"]])),
  },
  {
    id: "editor",
    name: "Editor",
    description: "Content Management",
    permissions: {
      Workspace: ["R", "W"],
      Models: ["R", "W"],
      Tasks: ["R", "W"],
      Memory: ["R"],
      Skills: ["R", "W"],
      MCP: ["R"],
    },
  },
  {
    id: "viewer",
    name: "Viewer",
    description: "Read-only Access",
    permissions: Object.fromEntries(permissionDomains.map((domain) => [domain, ["R"]])),
  },
];

const users: UserRecord[] = [
  { id: "usr-alex", name: "Alex Johnson", email: "alex@tron.ai", role: "Admin", workspace: "Enterprise", status: "Active", lastActive: "2m ago" },
  { id: "usr-maya", name: "Maya Singh", email: "maya@tron.ai", role: "Editor", workspace: "Platform", status: "Active", lastActive: "14m ago" },
  { id: "usr-chen", name: "Chen Wei", email: "chen@tron.ai", role: "Viewer", workspace: "Data", status: "Invited", lastActive: "Pending" },
  { id: "usr-rina", name: "Rina Patel", email: "rina@tron.ai", role: "Editor", workspace: "Support", status: "Suspended", lastActive: "Apr 28" },
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

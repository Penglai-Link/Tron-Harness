import { permissionDomains } from "@/domain/admin/constants";
import type {
  AuditLogRecord,
  RoleRecord,
  UserRecord,
  UserRoleCode,
  UserStatistics,
  WorkspaceDetailRecord,
  WorkspaceEffectiveUserRecord,
  WorkspaceLineageItem,
  WorkspaceRoleAccessRecord,
  WorkspaceStatistics,
  WorkspaceTreeNodeRecord,
  WorkspaceUserAccessRecord,
  WorkspaceStatus,
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

const MOCK_CREATED_AT = "2026-05-16T00:00:00Z";
let nextWorkspaceId = 734400000000000005n;
const workspaceTimestamps = new Map<string, { archivedAt: string | null; createdAt: string; updatedAt: string }>([
  ["ENTERPRISE", { createdAt: MOCK_CREATED_AT, updatedAt: MOCK_CREATED_AT, archivedAt: null }],
  ["PLATFORM", { createdAt: MOCK_CREATED_AT, updatedAt: MOCK_CREATED_AT, archivedAt: null }],
  ["TRON_CORE", { createdAt: MOCK_CREATED_AT, updatedAt: MOCK_CREATED_AT, archivedAt: null }],
  ["COMPLIANCE", { createdAt: MOCK_CREATED_AT, updatedAt: MOCK_CREATED_AT, archivedAt: null }],
]);

let workspaceNodes: WorkspaceTreeNodeRecord[] = [
  {
    id: "734400000000000001",
    workspaceCode: "ENTERPRISE",
    name: "Enterprise",
    description: "Global governance workspace",
    parentWorkspaceCode: null,
    depth: 1,
    status: "Active",
    childCount: 1,
    directUserCount: 0,
    roleGrantCount: 0,
    effectiveUserCount: 0,
    children: [
      {
        id: "734400000000000002",
        workspaceCode: "PLATFORM",
        name: "Platform",
        description: "Core platform runtime workspace",
        parentWorkspaceCode: "ENTERPRISE",
        depth: 2,
        status: "Active",
        childCount: 1,
        directUserCount: 0,
        roleGrantCount: 1,
        effectiveUserCount: 1,
        children: [
          {
            id: "734400000000000003",
            workspaceCode: "TRON_CORE",
            name: "Tron Core",
            description: "Main Tron console workspace",
            parentWorkspaceCode: "PLATFORM",
            depth: 3,
            status: "Active",
            childCount: 0,
            directUserCount: 1,
            roleGrantCount: 0,
            effectiveUserCount: 1,
            children: [],
          },
        ],
      },
    ],
  },
  {
    id: "734400000000000004",
    workspaceCode: "COMPLIANCE",
    name: "Compliance",
    description: "Compliance evidence workspace",
    parentWorkspaceCode: null,
    depth: 1,
    status: "Active",
    childCount: 0,
    directUserCount: 0,
    roleGrantCount: 0,
    effectiveUserCount: 0,
    children: [],
  },
];

let workspaceDirectUserAccess: Record<string, WorkspaceUserAccessRecord[]> = {
  TRON_CORE: [
    {
      userCode: "USER_ALEX_JOHNSON",
      displayName: "Alex Johnson",
      email: "alex@tron.ai",
      isOwner: true,
      grantedAt: "2026-05-16T00:00:00Z",
    },
  ],
};

let workspaceRoleAccess: Record<string, WorkspaceRoleAccessRecord[]> = {
  PLATFORM: [
    {
      roleCode: "WORKSPACE_ADMIN",
      roleName: "Workspace Admin",
      matchedUserCount: 1,
      grantedAt: "2026-05-16T00:00:00Z",
    },
  ],
};

function timestampNow() {
  return new Date().toISOString();
}

function cloneWorkspaceNode(record: WorkspaceTreeNodeRecord): WorkspaceTreeNodeRecord {
  return {
    ...record,
    children: record.children.map(cloneWorkspaceNode),
  };
}

function cloneWorkspaceTree(records: WorkspaceTreeNodeRecord[]) {
  return records.map(cloneWorkspaceNode);
}

function cloneWorkspaceUserAccess(records: WorkspaceUserAccessRecord[]) {
  return records.map((record) => ({ ...record }));
}

function cloneWorkspaceRoleAccess(records: WorkspaceRoleAccessRecord[]) {
  return records.map((record) => ({ ...record }));
}

function flattenWorkspaceTree(records: WorkspaceTreeNodeRecord[]): WorkspaceTreeNodeRecord[] {
  return records.flatMap((item) => [item, ...flattenWorkspaceTree(item.children)]);
}

function findWorkspaceNode(workspaceCode: string, records: WorkspaceTreeNodeRecord[] = workspaceNodes) {
  return flattenWorkspaceTree(records).find((item) => item.workspaceCode === workspaceCode) ?? null;
}

function buildLineage(workspaceCode: string, records: WorkspaceTreeNodeRecord[] = workspaceNodes): WorkspaceLineageItem[] {
  const flat = flattenWorkspaceTree(records);
  const lineage: WorkspaceLineageItem[] = [];
  let current = flat.find((item) => item.workspaceCode === workspaceCode) ?? null;
  while (current) {
    lineage.push({
      workspaceCode: current.workspaceCode,
      name: current.name,
      depth: current.depth,
    });
    current = current.parentWorkspaceCode
      ? flat.find((item) => item.workspaceCode === current?.parentWorkspaceCode) ?? null
      : null;
  }
  return lineage.reverse();
}

function getWorkspaceTimestamp(workspaceCode: string) {
  const existing = workspaceTimestamps.get(workspaceCode);
  if (existing) return existing;
  const next = {
    createdAt: MOCK_CREATED_AT,
    updatedAt: MOCK_CREATED_AT,
    archivedAt: null,
  };
  workspaceTimestamps.set(workspaceCode, next);
  return next;
}

function workspaceRoleLabel(roleCode: UserRoleCode) {
  return roles.find((role) => role.code === roleCode)?.name ?? roleCode;
}

function matchedUserCountForRole(roleCode: UserRoleCode) {
  return users.filter((user) => user.status === "Active" && user.roleCodes.includes(roleCode)).length;
}

function computeWorkspaceEffectiveUsers(workspaceCode: string): WorkspaceEffectiveUserRecord[] {
  const direct = workspaceDirectUserAccess[workspaceCode] ?? [];
  const grantedRoles = workspaceRoleAccess[workspaceCode] ?? [];
  const grantedRoleCodes = new Set(grantedRoles.map((role) => role.roleCode));
  const directMap = new Map(direct.map((entry) => [entry.userCode, entry]));

  return users
    .filter((user) => user.status === "Active")
    .map((user) => {
      const directEntry = directMap.get(user.userCode);
      const matchedRoleCodes = user.roleCodes.filter((roleCode) => grantedRoleCodes.has(roleCode));
      if (!directEntry && !matchedRoleCodes.length) return null;
      return {
        userCode: user.userCode,
        displayName: user.displayName,
        email: user.email,
        source: directEntry && matchedRoleCodes.length
          ? "DirectAndRole"
          : directEntry
            ? "Direct"
            : "Role",
        matchedRoleCodes,
        isOwner: directEntry?.isOwner ?? false,
      } satisfies WorkspaceEffectiveUserRecord;
    })
    .filter((item): item is WorkspaceEffectiveUserRecord => item !== null);
}

function normalizeWorkspaceRoleAccess() {
  workspaceRoleAccess = Object.fromEntries(
    Object.entries(workspaceRoleAccess)
      .filter(([, records]) => records.length > 0)
      .map(([workspaceCode, records]) => [
        workspaceCode,
        records.map((record) => ({
          ...record,
          roleName: workspaceRoleLabel(record.roleCode),
          matchedUserCount: matchedUserCountForRole(record.roleCode),
        })),
      ]),
  );
}

function recalculateWorkspaceTree(records: WorkspaceTreeNodeRecord[], parentWorkspaceCode: string | null, depth: number): WorkspaceTreeNodeRecord[] {
  return records.map((record) => {
    const children = recalculateWorkspaceTree(record.children, record.workspaceCode, depth + 1);
    const directUserCount = (workspaceDirectUserAccess[record.workspaceCode] ?? []).length;
    const roleGrantCount = (workspaceRoleAccess[record.workspaceCode] ?? []).length;
    const effectiveUserCount = computeWorkspaceEffectiveUsers(record.workspaceCode).length;

    return {
      ...record,
      parentWorkspaceCode,
      depth,
      childCount: children.length,
      directUserCount,
      roleGrantCount,
      effectiveUserCount,
      children,
    };
  });
}

function recalculateWorkspaceState() {
  normalizeWorkspaceRoleAccess();
  workspaceNodes = recalculateWorkspaceTree(workspaceNodes, null, 1);
}

function subtreeSpan(workspaceCode: string) {
  const node = findWorkspaceNode(workspaceCode);
  if (!node) return 0;

  const visit = (record: WorkspaceTreeNodeRecord): number => (
    record.children.length ? 1 + Math.max(...record.children.map(visit)) : 0
  );

  return visit(node);
}

function collectDescendantCodes(record: WorkspaceTreeNodeRecord): Set<string> {
  const codes = new Set<string>([record.workspaceCode]);
  record.children.forEach((child) => {
    collectDescendantCodes(child).forEach((value) => codes.add(value));
  });
  return codes;
}

function removeWorkspaceNode(records: WorkspaceTreeNodeRecord[], workspaceCode: string): {
  nextRecords: WorkspaceTreeNodeRecord[];
  removedNode: WorkspaceTreeNodeRecord | null;
} {
  let removedNode: WorkspaceTreeNodeRecord | null = null;
  const nextRecords = records
    .map((record) => {
      if (record.workspaceCode === workspaceCode) {
        removedNode = record;
        return null;
      }

      if (!record.children.length) return record;

      const nested = removeWorkspaceNode(record.children, workspaceCode);
      if (nested.removedNode) {
        removedNode = nested.removedNode;
        return {
          ...record,
          children: nested.nextRecords,
        };
      }

      return record;
    })
    .filter((record): record is WorkspaceTreeNodeRecord => record !== null);

  return { nextRecords, removedNode };
}

function insertWorkspaceNode(
  records: WorkspaceTreeNodeRecord[],
  parentWorkspaceCode: string | null,
  node: WorkspaceTreeNodeRecord,
): WorkspaceTreeNodeRecord[] {
  if (!parentWorkspaceCode) return [...records, node];

  return records.map((record) => {
    if (record.workspaceCode === parentWorkspaceCode) {
      return {
        ...record,
        children: [...record.children, node],
      };
    }

    return record.children.length
      ? {
        ...record,
        children: insertWorkspaceNode(record.children, parentWorkspaceCode, node),
      }
      : record;
  });
}

function updateWorkspaceNode(
  records: WorkspaceTreeNodeRecord[],
  workspaceCode: string,
  updater: (record: WorkspaceTreeNodeRecord) => WorkspaceTreeNodeRecord,
): WorkspaceTreeNodeRecord[] {
  return records.map((record) => {
    if (record.workspaceCode === workspaceCode) return updater(record);
    if (!record.children.length) return record;
    return {
      ...record,
      children: updateWorkspaceNode(record.children, workspaceCode, updater),
    };
  });
}

function updateWorkspaceStatusSubtree(record: WorkspaceTreeNodeRecord, status: WorkspaceStatus, archivedAt: string | null): WorkspaceTreeNodeRecord {
  const timestamp = getWorkspaceTimestamp(record.workspaceCode);
  timestamp.updatedAt = timestampNow();
  timestamp.archivedAt = archivedAt;

  return {
    ...record,
    status,
    children: record.children.map((child) => updateWorkspaceStatusSubtree(child, status, archivedAt)),
  };
}

function ensureActiveWorkspaceParent(parentWorkspaceCode: string | null) {
  if (!parentWorkspaceCode) return;
  const parent = findWorkspaceNode(parentWorkspaceCode);
  if (!parent) throw new Error(`Workspace ${parentWorkspaceCode} was not found.`);
  if (parent.status !== "Active") throw new Error("Parent workspace must be active.");
}

function createWorkspaceTreeNode(
  workspaceCode: string,
  name: string,
  description: string | null,
  parentWorkspaceCode: string | null,
) {
  const createdAt = timestampNow();
  workspaceTimestamps.set(workspaceCode, {
    createdAt,
    updatedAt: createdAt,
    archivedAt: null,
  });

  return {
    id: String(nextWorkspaceId++),
    workspaceCode,
    name,
    description,
    parentWorkspaceCode,
    depth: 1,
    status: "Active" satisfies WorkspaceStatus,
    childCount: 0,
    directUserCount: 0,
    roleGrantCount: 0,
    effectiveUserCount: 0,
    children: [],
  } satisfies WorkspaceTreeNodeRecord;
}

export function getWorkspaceTree() {
  recalculateWorkspaceState();
  return cloneWorkspaceTree(workspaceNodes);
}

export function getWorkspaceStatistics(): WorkspaceStatistics {
  recalculateWorkspaceState();
  const flat = flattenWorkspaceTree(workspaceNodes);
  return {
    totalWorkspaces: flat.length,
    activeWorkspaces: flat.filter((item) => item.status === "Active").length,
    directUserGrants: Object.values(workspaceDirectUserAccess).reduce((total, items) => total + items.length, 0),
    roleGrants: Object.values(workspaceRoleAccess).reduce((total, items) => total + items.length, 0),
  };
}

export function getWorkspaceDetail(workspaceCode: string): WorkspaceDetailRecord | null {
  recalculateWorkspaceState();
  const node = findWorkspaceNode(workspaceCode);
  if (!node) return null;
  const timestamps = getWorkspaceTimestamp(workspaceCode);
  return {
    id: node.id,
    workspaceCode: node.workspaceCode,
    name: node.name,
    description: node.description,
    parentWorkspaceCode: node.parentWorkspaceCode,
    depth: node.depth,
    status: node.status,
    lineage: buildLineage(workspaceCode),
    children: node.children,
    directUserCount: node.directUserCount,
    roleGrantCount: node.roleGrantCount,
    effectiveUserCount: node.effectiveUserCount,
    createdAt: timestamps.createdAt,
    updatedAt: timestamps.updatedAt,
    archivedAt: timestamps.archivedAt,
  };
}

export function getWorkspaceUserAccess(workspaceCode: string): WorkspaceUserAccessRecord[] {
  return cloneWorkspaceUserAccess(workspaceDirectUserAccess[workspaceCode] ?? []);
}

export function getWorkspaceRoleAccess(workspaceCode: string): WorkspaceRoleAccessRecord[] {
  recalculateWorkspaceState();
  return cloneWorkspaceRoleAccess(workspaceRoleAccess[workspaceCode] ?? []);
}

export function getWorkspaceEffectiveUsers(workspaceCode: string): WorkspaceEffectiveUserRecord[] {
  recalculateWorkspaceState();
  return computeWorkspaceEffectiveUsers(workspaceCode).map((record) => ({ ...record, matchedRoleCodes: [...record.matchedRoleCodes] }));
}

export function createWorkspaceRecord(payload: {
  workspaceCode: string;
  name: string;
  description?: string | null;
  parentWorkspaceCode?: string | null;
}) {
  const workspaceCode = payload.workspaceCode.trim().toUpperCase();
  const name = payload.name.trim();
  const description = payload.description?.trim() || null;
  const parentWorkspaceCode = payload.parentWorkspaceCode ?? null;

  if (!workspaceCode) throw new Error("Workspace code is required.");
  if (!name) throw new Error("Workspace name is required.");
  if (findWorkspaceNode(workspaceCode)) throw new Error(`Workspace ${workspaceCode} already exists.`);
  ensureActiveWorkspaceParent(parentWorkspaceCode);

  const parent = parentWorkspaceCode ? findWorkspaceNode(parentWorkspaceCode) : null;
  const depth = (parent?.depth ?? 0) + 1;
  if (depth > 3) throw new Error("Workspace hierarchy cannot exceed 3 levels.");

  workspaceNodes = insertWorkspaceNode(
    workspaceNodes,
    parentWorkspaceCode,
    createWorkspaceTreeNode(workspaceCode, name, description, parentWorkspaceCode),
  );
  recalculateWorkspaceState();

  const detail = getWorkspaceDetail(workspaceCode);
  if (!detail) throw new Error(`Workspace ${workspaceCode} could not be created.`);
  return detail;
}

export function updateWorkspaceRecord(
  workspaceCode: string,
  payload: {
    name: string;
    description?: string | null;
  },
) {
  const target = findWorkspaceNode(workspaceCode);
  if (!target) throw new Error(`Workspace ${workspaceCode} was not found.`);
  if (target.status !== "Active") throw new Error("Archived workspace cannot be modified.");

  const name = payload.name.trim();
  if (!name) throw new Error("Workspace name is required.");
  const description = payload.description?.trim() || null;
  const timestamp = getWorkspaceTimestamp(workspaceCode);
  timestamp.updatedAt = timestampNow();

  workspaceNodes = updateWorkspaceNode(workspaceNodes, workspaceCode, (record) => ({
    ...record,
    name,
    description,
  }));
  recalculateWorkspaceState();

  const detail = getWorkspaceDetail(workspaceCode);
  if (!detail) throw new Error(`Workspace ${workspaceCode} could not be updated.`);
  return detail;
}

export function moveWorkspaceRecord(workspaceCode: string, parentWorkspaceCode?: string | null) {
  const nextParentWorkspaceCode = parentWorkspaceCode ?? null;
  const target = findWorkspaceNode(workspaceCode);
  if (!target) throw new Error(`Workspace ${workspaceCode} was not found.`);
  if (target.status !== "Active") throw new Error("Archived workspace cannot be modified.");
  ensureActiveWorkspaceParent(nextParentWorkspaceCode);

  if (nextParentWorkspaceCode === workspaceCode) {
    throw new Error("Workspace cannot become its own parent.");
  }

  const targetDescendants = collectDescendantCodes(target);
  if (nextParentWorkspaceCode && targetDescendants.has(nextParentWorkspaceCode)) {
    throw new Error("Workspace cannot move under its own descendant.");
  }

  const parent = nextParentWorkspaceCode ? findWorkspaceNode(nextParentWorkspaceCode) : null;
  const nextDepth = (parent?.depth ?? 0) + 1;
  if (nextDepth + subtreeSpan(workspaceCode) > 3) {
    throw new Error("Workspace hierarchy cannot exceed 3 levels.");
  }

  const removal = removeWorkspaceNode(workspaceNodes, workspaceCode);
  if (!removal.removedNode) throw new Error(`Workspace ${workspaceCode} was not found.`);

  const timestamp = getWorkspaceTimestamp(workspaceCode);
  timestamp.updatedAt = timestampNow();
  workspaceNodes = insertWorkspaceNode(
    removal.nextRecords,
    nextParentWorkspaceCode,
    {
      ...removal.removedNode,
      parentWorkspaceCode: nextParentWorkspaceCode,
    },
  );
  recalculateWorkspaceState();

  const detail = getWorkspaceDetail(workspaceCode);
  if (!detail) throw new Error(`Workspace ${workspaceCode} could not be moved.`);
  return detail;
}

export function archiveWorkspaceRecord(workspaceCode: string) {
  const target = findWorkspaceNode(workspaceCode);
  if (!target) throw new Error(`Workspace ${workspaceCode} was not found.`);
  const archivedAt = timestampNow();
  workspaceNodes = updateWorkspaceNode(workspaceNodes, workspaceCode, (record) => updateWorkspaceStatusSubtree(record, "Archived", archivedAt));
  recalculateWorkspaceState();

  return {
    workspaceCode,
    status: "Archived" satisfies WorkspaceStatus,
    archivedAt,
    affectedWorkspaceCount: flattenWorkspaceTree([findWorkspaceNode(workspaceCode)!]).length,
  };
}

export function restoreWorkspaceRecord(workspaceCode: string) {
  const target = findWorkspaceNode(workspaceCode);
  if (!target) throw new Error(`Workspace ${workspaceCode} was not found.`);
  if (target.parentWorkspaceCode) {
    const parent = findWorkspaceNode(target.parentWorkspaceCode);
    if (parent?.status === "Archived") {
      throw new Error("Restore the parent workspace before restoring this node.");
    }
  }

  workspaceNodes = updateWorkspaceNode(workspaceNodes, workspaceCode, (record) => updateWorkspaceStatusSubtree(record, "Active", null));
  recalculateWorkspaceState();

  return {
    workspaceCode,
    status: "Active" satisfies WorkspaceStatus,
    archivedAt: null,
    affectedWorkspaceCount: flattenWorkspaceTree([findWorkspaceNode(workspaceCode)!]).length,
  };
}

export function replaceWorkspaceUserAccessRecords(
  workspaceCode: string,
  entries: Array<{ userCode: string; isOwner: boolean }>,
) {
  if (!findWorkspaceNode(workspaceCode)) throw new Error(`Workspace ${workspaceCode} was not found.`);

  const seen = new Set<string>();
  const nextRecords = entries.map((entry) => {
    const userCode = entry.userCode.trim();
    if (!userCode) throw new Error("User code is required.");
    if (seen.has(userCode)) throw new Error(`Duplicate direct user grant: ${userCode}.`);
    seen.add(userCode);

    const user = users.find((candidate) => candidate.userCode === userCode && candidate.status === "Active");
    if (!user) throw new Error(`User ${userCode} is not available for workspace access.`);

    const existing = (workspaceDirectUserAccess[workspaceCode] ?? []).find((record) => record.userCode === userCode);
    return {
      userCode,
      displayName: user.displayName,
      email: user.email,
      isOwner: entry.isOwner,
      grantedAt: existing?.grantedAt ?? timestampNow(),
    } satisfies WorkspaceUserAccessRecord;
  });

  if (nextRecords.length) {
    workspaceDirectUserAccess = {
      ...workspaceDirectUserAccess,
      [workspaceCode]: nextRecords,
    };
  } else {
    const { [workspaceCode]: _, ...rest } = workspaceDirectUserAccess;
    workspaceDirectUserAccess = rest;
  }

  getWorkspaceTimestamp(workspaceCode).updatedAt = timestampNow();
  recalculateWorkspaceState();
  return getWorkspaceUserAccess(workspaceCode);
}

export function replaceWorkspaceRoleAccessRecords(
  workspaceCode: string,
  entries: Array<{ roleCode: UserRoleCode }>,
) {
  if (!findWorkspaceNode(workspaceCode)) throw new Error(`Workspace ${workspaceCode} was not found.`);

  const seen = new Set<string>();
  const nextRecords = entries.map((entry) => {
    const roleCode = entry.roleCode.trim() as UserRoleCode;
    if (!roleCode) throw new Error("Role code is required.");
    if (seen.has(roleCode)) throw new Error(`Duplicate workspace role grant: ${roleCode}.`);
    seen.add(roleCode);

    const role = roles.find((candidate) => candidate.code === roleCode);
    if (!role) throw new Error(`Role ${roleCode} is not available for workspace access.`);

    const existing = (workspaceRoleAccess[workspaceCode] ?? []).find((record) => record.roleCode === roleCode);
    return {
      roleCode,
      roleName: role.name,
      matchedUserCount: matchedUserCountForRole(roleCode),
      grantedAt: existing?.grantedAt ?? timestampNow(),
    } satisfies WorkspaceRoleAccessRecord;
  });

  if (nextRecords.length) {
    workspaceRoleAccess = {
      ...workspaceRoleAccess,
      [workspaceCode]: nextRecords,
    };
  } else {
    const { [workspaceCode]: _, ...rest } = workspaceRoleAccess;
    workspaceRoleAccess = rest;
  }

  getWorkspaceTimestamp(workspaceCode).updatedAt = timestampNow();
  recalculateWorkspaceState();
  return getWorkspaceRoleAccess(workspaceCode);
}

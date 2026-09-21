import { permissionDomains } from "@/domain/admin/constants";
import type {
  AuditLogRecord,
  WorkspaceDetailRecord,
  WorkspaceLineageItem,
  WorkspaceTreeNodeRecord,
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

export function listPermissionDomains() {
  return [...permissionDomains];
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
    children: [],
  },
];

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

function flattenWorkspaceTree(records: WorkspaceTreeNodeRecord[]): WorkspaceTreeNodeRecord[] {
  return records.flatMap((item) => [item, ...flattenWorkspaceTree(item.children)]);
}

function findWorkspaceNode(workspaceCode: string, records: WorkspaceTreeNodeRecord[] = workspaceNodes) {
  return flattenWorkspaceTree(records).find((item) => item.workspaceCode === workspaceCode) ?? null;
}

function buildWorkspacePath(workspaceCode: string, records: WorkspaceTreeNodeRecord[] = workspaceNodes): WorkspaceLineageItem[] {
  const flat = flattenWorkspaceTree(records);
  const path: WorkspaceLineageItem[] = [];
  let current = flat.find((item) => item.workspaceCode === workspaceCode) ?? null;
  while (current) {
    path.push({
      workspaceCode: current.workspaceCode,
      name: current.name,
      depth: current.depth,
    });
    current = current.parentWorkspaceCode
      ? flat.find((item) => item.workspaceCode === current?.parentWorkspaceCode) ?? null
      : null;
  }
  return path.reverse();
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

function recalculateWorkspaceTree(records: WorkspaceTreeNodeRecord[], parentWorkspaceCode: string | null, depth: number): WorkspaceTreeNodeRecord[] {
  return records.map((record) => {
    const children = recalculateWorkspaceTree(record.children, record.workspaceCode, depth + 1);

    return {
      ...record,
      parentWorkspaceCode,
      depth,
      childCount: children.length,
      children,
    };
  });
}

function recalculateWorkspaceState() {
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
    children: [],
  } satisfies WorkspaceTreeNodeRecord;
}

export function getWorkspaceTree() {
  recalculateWorkspaceState();
  return cloneWorkspaceTree(workspaceNodes);
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
    path: buildWorkspacePath(workspaceCode),
    children: node.children,
    createdAt: timestamps.createdAt,
    updatedAt: timestamps.updatedAt,
    archivedAt: timestamps.archivedAt,
  };
}

export function createWorkspaceRecord(payload: {
  name: string;
  description?: string | null;
  parentWorkspaceCode?: string | null;
}) {
  const workspaceCode = generateWorkspaceCode();
  const name = payload.name.trim();
  const description = payload.description?.trim() || null;
  const parentWorkspaceCode = payload.parentWorkspaceCode ?? null;

  if (!name) throw new Error("Workspace name is required.");
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

function generateWorkspaceCode() {
  for (let attempt = 0; attempt < 16; attempt += 1) {
    const workspaceCode = `WS-${randomWorkspaceCodeSuffix()}`;
    if (!findWorkspaceNode(workspaceCode)) return workspaceCode;
  }

  throw new Error("Workspace code could not be generated.");
}

function randomWorkspaceCodeSuffix() {
  const buffer = new Uint32Array(2);
  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(buffer);
  } else {
    buffer[0] = Math.floor(Math.random() * 0xffffffff);
    buffer[1] = Math.floor(Math.random() * 0xffffffff);
  }

  return Array.from(buffer, (value) => value.toString(16).padStart(8, "0")).join("").toUpperCase();
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

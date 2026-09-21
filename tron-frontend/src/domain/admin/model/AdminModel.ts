export interface AuditLogRecord {
  id: string;
  timestamp: string;
  actor: string;
  action: string;
  resourceId: string;
  status: "Success" | "Warning" | "Failed";
  ipAddress: string;
}

export type WorkspaceStatus = "Active" | "Archived";

export interface WorkspaceTreeNodeRecord {
  id: string;
  workspaceCode: string;
  name: string;
  description: string | null;
  parentWorkspaceCode: string | null;
  depth: number;
  status: WorkspaceStatus;
  childCount: number;
  children: WorkspaceTreeNodeRecord[];
}

export interface WorkspaceLineageItem {
  workspaceCode: string;
  name: string;
  depth: number;
}

export interface WorkspaceDetailRecord {
  id: string;
  workspaceCode: string;
  name: string;
  description: string | null;
  parentWorkspaceCode: string | null;
  depth: number;
  status: WorkspaceStatus;
  path: WorkspaceLineageItem[];
  children: WorkspaceTreeNodeRecord[];
  createdAt: string;
  updatedAt: string;
  archivedAt: string | null;
}

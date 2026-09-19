export interface ProjectRecord {
  projectCode: string;
  workspaceCode: string;
  name: string;
  appCodes: string[];
  primaryAppCode?: string | null;
  updatedAt: string;
}

export interface SaveProjectInput {
  projectCode?: string | null;
  workspaceCode: string;
  name: string;
  appCodes: string[];
  primaryAppCode?: string | null;
}

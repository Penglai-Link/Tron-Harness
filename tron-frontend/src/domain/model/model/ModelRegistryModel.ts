export interface ModelCostLine {
  label: string;
  value: string;
}

export type ModelVendor = "OpenAI" | "Anthropic" | "Google";
export type WorkspaceModelStatus = "Available" | "Removed";

export interface WorkspaceModelRecord {
  modelCode: string;
  name: string;
  purpose: string;
  vendor: ModelVendor;
  context: string;
  maxOutput: string;
  trainingData: string;
  speed: string;
  price: string;
  status: WorkspaceModelStatus;
  sourceWorkspaceCode: string;
  sourceWorkspaceName: string;
  sourceLabel: string;
  removedAt: string | null;
}

export interface WorkspaceModelMutationResult {
  workspaceCode: string;
  modelCode: string;
  status: WorkspaceModelStatus;
  removedAt: string | null;
}

export interface WorkspaceModelQuery {
  search?: string;
  status?: WorkspaceModelStatus;
  vendor?: ModelVendor;
}

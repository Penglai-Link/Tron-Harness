export type PromptCategory = "SYSTEM" | "PERSONA" | "TOOL" | "TEMPLATE";
export type PromptScopeKind = "PLATFORM" | "WORKSPACE";
export type PromptSourceKind = "PLATFORM" | "LOCAL" | "INHERITED" | "OVERRIDE";
export type PromptVariableType = "STRING";

export interface PromptVariable {
  name: string;
  type: PromptVariableType;
}

export interface PromptWorkspacePathItem {
  workspaceCode: string;
  name: string;
}

export interface PromptRecord {
  promptCode: string;
  scopeKind: PromptScopeKind;
  workspaceCode: string | null;
  displayName: string;
  category: PromptCategory;
  promptBody: string;
  description: string | null;
  enabled: boolean;
  sourceKind: PromptSourceKind;
  sourceLabel: string;
  sourceWorkspaceCode: string | null;
  sourceWorkspaceName: string | null;
  sourceWorkspacePath: PromptWorkspacePathItem[];
  allowInheritance: boolean | null;
  variables: PromptVariable[];
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface PromptPage<T> {
  current: number;
  size: number;
  total: number;
  records: T[];
}

export interface PromptPageQuery {
  current: number;
  size: number;
  workspaceCode?: string;
  search?: string;
  category?: PromptCategory;
  sourceKind?: PromptSourceKind;
  enabled?: boolean;
}

export interface PromptCreateFields {
  displayName: string;
  category: PromptCategory;
  promptBody: string;
  description: string;
  enabled: boolean;
}

export interface WorkspacePromptCreatePayload extends PromptCreateFields {
  workspaceCode: string;
  allowInheritance: boolean;
}

export type PlatformPromptCreatePayload = PromptCreateFields;

export interface PromptUpdateFields {
  promptCode: string;
  expectedVersion: number;
  displayName: string;
  promptBody: string;
  description: string;
  enabled: boolean;
}

export interface WorkspacePromptUpdatePayload extends PromptUpdateFields {
  allowInheritance?: boolean;
  workspaceCode: string;
}

export type PlatformPromptUpdatePayload = PromptUpdateFields;

export interface PromptDetailPayload {
  promptCode: string;
  workspaceCode?: string;
}

export interface PromptDuplicatePayload extends PromptDetailPayload {
  targetCategory: PromptCategory;
}

export interface PromptAnalyzeResult {
  variables: PromptVariable[];
  invalidVariables: string[];
}

export interface PromptRenderPayload {
  promptBody: string;
  variables: Record<string, string>;
}

export interface PromptRenderResult {
  renderedPrompt: string;
}

export interface PromptExportResult {
  fileName: string;
  content: string;
}

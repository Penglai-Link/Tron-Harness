export type ModelScopeKind = "PLATFORM" | "WORKSPACE";
export type ModelSourceKind = "PLATFORM" | "LOCAL" | "INHERITED" | "OVERRIDE";
export type ModelProtocol = "OPENAI_COMPATIBLE" | "RESPONSES" | "RERANK";
export type ModelType = "TEXT" | "EMBEDDING" | "RERANK" | "IMAGE";
export type ModelVerifyStatus = "SUCCEEDED" | "FAILED";
export type ModelCapability =
  | "MULTIMODAL_ALL"
  | "TEXT_GENERATION"
  | "DEEP_THINKING"
  | "VISION_UNDERSTANDING"
  | "IMAGE_GENERATION"
  | "VIDEO_GENERATION"
  | "THREED_GENERATION"
  | "SPEECH_RECOGNITION"
  | "SPEECH_SYNTHESIS"
  | "MULTIMODAL_VECTOR"
  | "TEXT_VECTOR"
  | "REALTIME_MULTIMODAL"
  | "REALTIME_SPEECH_SYNTHESIS"
  | "REALTIME_SPEECH_RECOGNITION"
  | "REALTIME_SPEECH_TRANSLATION"
  | "RERANK";

export interface ModelWorkspacePathItem {
  workspaceCode: string;
  name: string;
}

export interface ModelRecord {
  scopeKind: ModelScopeKind;
  workspaceCode: string | null;
  modelCode: string;
  displayName: string;
  vendor: string;
  protocol: ModelProtocol;
  modelType: ModelType;
  modelCapabilities: ModelCapability[];
  baseUrl: string;
  apiPath: string | null;
  apiKeyRef: string;
  modelName: string;
  enabled: boolean;
  description: string | null;
  thinkingEnabled: boolean;
  structuredOutputEnabled: boolean;
  temperature: number | null;
  maxTokens: number | null;
  contextWindowTokens: number | null;
  topP: number | null;
  maxRetries: number | null;
  lastVerifiedAt: string | null;
  lastVerifyStatus: ModelVerifyStatus | null;
  lastVerifyMessage: string | null;
  sourceKind: ModelSourceKind;
  sourceModelCode: string | null;
  sourceLabel: string;
  sourceWorkspaceCode: string | null;
  sourceWorkspaceName: string | null;
  sourceWorkspacePath: ModelWorkspacePathItem[];
  allowInheritance: boolean | null;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface WorkspaceDefaultModelRecord {
  modelType: ModelType;
  modelCode: string | null;
  displayName: string | null;
  sourceLabel: string | null;
}

export interface WorkspaceModelCatalog {
  workspaceCode: string;
  defaults: WorkspaceDefaultModelRecord[];
  records: ModelRecord[];
}

export interface ModelSelectOption {
  modelCode: string;
  displayName: string;
  modelName: string;
  sourceKind: ModelSourceKind;
  sourceLabel: string;
  vendor: string;
  protocol: ModelProtocol;
  modelType: ModelType;
  enabled: boolean;
}

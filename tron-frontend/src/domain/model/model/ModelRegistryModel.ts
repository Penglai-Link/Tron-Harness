export type ModelScopeKind = "PLATFORM" | "WORKSPACE";
export type ModelSourceKind = "PLATFORM" | "LOCAL" | "INHERITED" | "OVERRIDE";
export type ModelProtocol =
  | "OPENAI_COMPATIBLE"
  | "OPENAI_COMPATIBLE_EMBEDDING"
  | "RESPONSES"
  | "RERANK";
export type ModelType = "TEXT" | "EMBEDDING" | "RERANK" | "IMAGE";
export type ModelVerifyStatus = "SUCCEEDED" | "FAILED";
export type ModelProviderCode = "PersonAi";
export type ModelSelectionKind = "Fixed" | "ProviderRoute";
export type ProviderRouteCode = "ChatAuto" | "EmbeddingAuto" | "LlmFallback";
export type ProviderCatalogModelStatus = "Available" | "Unavailable";
export type ProviderCatalogModelType =
  | "Chat"
  | "Embedding"
  | "Image"
  | "Video"
  | "Speech"
  | "AigcTextDetection"
  | "AigcImageDetection";
export type ProviderModelQuality = "High" | "Middle" | "Low";
export type ReasoningEffort =
  | "None"
  | "Minimal"
  | "Low"
  | "Medium"
  | "High"
  | "XHigh"
  | "Max"
  | "Ultra";
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
  supportedReasoningEfforts: ReasoningEffort[];
  defaultReasoningEffort: ReasoningEffort | null;
  structuredOutputEnabled: boolean;
  responsesStatefulEnabled: boolean;
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
  optionCode?: string;
  modelCode: string;
  displayName: string;
  modelName: string;
  sourceKind: ModelSourceKind;
  sourceLabel: string;
  vendor: string;
  protocol: ModelProtocol;
  modelType: ModelType;
  enabled: boolean;
  /**
   * The server owns the selection contract. Existing mock records intentionally
   * omit it and are normalized to a legacy Fixed selection by callers.
   */
  modelSelection?: ModelSelection;
  providerCode?: ModelProviderCode | null;
  accessPlatform?: string | null;
  upstreamVendorName?: string | null;
  remoteStatus?: ProviderCatalogModelStatus | null;
  contextWindowTokens?: number | null;
  qualityType?: ProviderModelQuality | null;
  inputCoefficient?: number | null;
  cacheHitCoefficient?: number | null;
  outputCoefficient?: number | null;
  supportedReasoningEfforts?: ReasoningEffort[];
  defaultReasoningEffort?: ReasoningEffort | null;
  isDefault?: boolean;
}

export interface ModelSelection {
  kind: ModelSelectionKind;
  providerCode?: ModelProviderCode | null;
  modelCode?: string | null;
  qualityType?: ProviderModelQuality | null;
  routeCode?: ProviderRouteCode | null;
}

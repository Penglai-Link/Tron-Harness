export type ModelSourceKind = "Provider";
export type ModelProtocol =
  | "OPENAI_COMPATIBLE"
  | "OPENAI_COMPATIBLE_EMBEDDING"
  | "RESPONSES"
  | "ANTHROPIC"
  | "GEMINI"
  | "RERANK";
export type ModelType = "TEXT" | "TEXT_EMBEDDING" | "MULTIMODAL_EMBEDDING" | "TEXT_RERANK" | "MULTIMODAL_RERANK" | "IMAGE" | "VIDEO" | "SPEECH_SYNTHESIS" | "SPEECH_RECOGNITION" | "REALTIME_SPEECH";
export type ModelProviderCode = "PersonAi";
export type ModelSelectionKind = "Fixed" | "ProviderRoute" | "Auto";
export type ProviderRouteCode = "ChatAuto" | "EmbeddingAuto" | "MultimodalEmbeddingAuto" | "RerankAuto";
export type ProviderCatalogModelStatus = "Available" | "Unavailable";
export type ProviderCatalogModelType =
  | "Chat"
  | "Embedding"
  | "Rerank"
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

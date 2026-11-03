import { requestApi } from "@/shared/api/httpClient";

export type ConfiguredProviderProtocol =
  | "OpenAiChatCompletions"
  | "OpenAiResponses"
  | "AnthropicMessages"
  | "GoogleGemini";

export type ProviderInputCapability = "Text" | "Image" | "Audio" | "Video";

export type ConfiguredProviderModelType = "Text" | "TextEmbedding" | "MultimodalEmbedding" | "TextRerank" | "MultimodalRerank" | "Image" | "Video" | "SpeechSynthesis" | "SpeechRecognition" | "RealtimeSpeech";

export interface ConfiguredProviderModel {
  modelType: ConfiguredProviderModelType;
  dimension?: number | null;
  modelCode: string | null;
  logicalModelCode?: string | null;
  modelId: string;
  name: string;
  contextWindowTokens: number | null;
  maxOutputTokens: number | null;
  reasoning: boolean;
  supportsDeveloperRole: boolean;
  inputCapabilities: ProviderInputCapability[];
  enabled: boolean;
}

export interface ConfiguredProviderUpdate {
  providerType?: string | null;
  displayName: string;
  protocol: ConfiguredProviderProtocol;
  baseUrl: string;
  apiKey: string | null;
  requestHeaders: Record<string, string>;
  enabled: boolean;
  models: ConfiguredProviderModel[];
}

export type ConfiguredProviderCreate = ConfiguredProviderUpdate;

export interface ConfiguredProvider extends Omit<ConfiguredProviderUpdate, "apiKey"> {
  providerId: string;
  hasApiKey: boolean;
  apiKeySummary: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface ProviderPreset {
  id: string;
  name: string;
  protocol: ConfiguredProviderProtocol | null;
  baseUrl: string | null;
  envKey: string | null;
  guessed: boolean;
  needsBaseUrl: boolean;
  supported: boolean;
  unsupportedReason: string | null;
  modelCount: number;
}

export interface ProviderPresetModel {
  modelId: string;
  name: string;
  contextWindowTokens: number | null;
  reasoning: boolean;
  inputCapabilities: ("text" | "image" | "audio" | "video")[];
  toolUse: boolean;
}

export function fetchConfiguredProvider(providerId: string, signal?: AbortSignal) {
  return requestApi<ConfiguredProvider>("GET /api/v1/model-providers/configured/{providerId}", {
    pathParams: { providerId }, signal,
  });
}

export function createConfiguredProvider(body: ConfiguredProviderCreate) {
  return requestApi<ConfiguredProvider>("POST /api/v1/model-providers/configured", { body });
}

export function updateConfiguredProvider(providerId: string, body: ConfiguredProviderUpdate) {
  return requestApi<ConfiguredProvider>("PUT /api/v1/model-providers/configured/{providerId}", {
    pathParams: { providerId }, body,
  });
}

export function deleteConfiguredProvider(providerId: string) {
  return requestApi<Record<string, never>>("DELETE /api/v1/model-providers/configured/{providerId}", {
    pathParams: { providerId },
  });
}

export function fetchProviderPresets(signal?: AbortSignal) {
  return requestApi<ProviderPreset[]>("GET /api/v1/model-providers/presets", { signal });
}

export function fetchProviderPresetModels(presetId: string, signal?: AbortSignal) {
  return requestApi<ProviderPresetModel[]>("GET /api/v1/model-providers/presets/{presetId}/models", {
    pathParams: { presetId }, signal,
  });
}

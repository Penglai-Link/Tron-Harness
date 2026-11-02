import type {
  ConfiguredProvider,
  ConfiguredProviderCreate,
  ConfiguredProviderModel,
  ConfiguredProviderModelType,
  ConfiguredProviderProtocol,
  ProviderInputCapability,
  ProviderPreset,
  ProviderPresetModel,
} from "@/domain/model/api/configuredProviderApi";

export const configuredProviderProtocols: ConfiguredProviderProtocol[] = [
  "OpenAiChatCompletions", "OpenAiResponses", "AnthropicMessages", "GoogleGemini",
];

export const configuredProviderModelTypes: ConfiguredProviderModelType[] = [
  "Text", "TextEmbedding", "MultimodalEmbedding", "TextRerank", "MultimodalRerank", "Image", "Video", "SpeechSynthesis", "SpeechRecognition", "RealtimeSpeech",
];
export function isMultimodalModel(type: ConfiguredProviderModelType) { return type === "MultimodalEmbedding" || type === "MultimodalRerank"; }
export function isMediaModel(type: ConfiguredProviderModelType) {
  return !["Text", "TextEmbedding", "MultimodalEmbedding", "TextRerank", "MultimodalRerank"].includes(type);
}
export function supportsModelType(type: ConfiguredProviderModelType, protocol: ConfiguredProviderProtocol) {
  if (type === "Text") return true;
  if (isMediaModel(type)) return protocol !== "AnthropicMessages";
  return ["OpenAiChatCompletions", "OpenAiResponses"].includes(protocol);
}

export interface ProviderModelDraft extends Omit<ConfiguredProviderModel, "contextWindowTokens" | "maxOutputTokens" | "dimension"> {
  rowId: string;
  dimension: string;
  contextWindowTokens: string;
  maxOutputTokens: string;
}

export interface ProviderHeaderDraft { rowId: string; name: string; value: string }

export interface ConfiguredProviderDraft {
  providerType: string | null;
  displayName: string;
  protocol: ConfiguredProviderProtocol;
  baseUrl: string;
  apiKey: string;
  hasApiKey: boolean;
  apiKeySummary: string | null;
  enabled: boolean;
  headers: ProviderHeaderDraft[];
  models: ProviderModelDraft[];
}

let draftSequence = 0;
export function draftRowId() { return `provider-draft-${++draftSequence}`; }

export function emptyProviderModel(): ProviderModelDraft {
  return {
    rowId: draftRowId(), dimension: "", modelType: "Text", modelCode: null, modelId: "", name: "", contextWindowTokens: "",
    maxOutputTokens: "", reasoning: false, supportsDeveloperRole: false,
    inputCapabilities: ["Text"], enabled: true,
  };
}

export function emptyProviderDraft(): ConfiguredProviderDraft {
  return {
    providerType: "custom", displayName: "", protocol: "OpenAiChatCompletions", baseUrl: "",
    apiKey: "", hasApiKey: false, apiKeySummary: null, enabled: true, headers: [], models: [emptyProviderModel()],
  };
}

export function configuredProviderToDraft(record: ConfiguredProvider): ConfiguredProviderDraft {
  const { providerId: _providerId, ...connection } = record;
  return {
    ...connection, providerType: record.providerType ?? null, apiKey: "",
    headers: Object.entries(record.requestHeaders).map(([name, value]) => ({ rowId: draftRowId(), name, value })),
    models: record.models.map((model) => ({
      ...model, dimension: model.dimension?.toString() ?? "", modelType: model.modelType ?? "Text", rowId: draftRowId(), contextWindowTokens: model.contextWindowTokens?.toString() ?? "",
      maxOutputTokens: model.maxOutputTokens?.toString() ?? "",
    })),
  };
}

export function duplicateProviderDraft(record: ConfiguredProvider): ConfiguredProviderDraft {
  const draft = configuredProviderToDraft(record);
  return {
    ...draft, apiKey: "", hasApiKey: false, apiKeySummary: null,
    models: draft.models.map(model => ({ ...model, modelCode: null })),
  };
}

export function presetToDraft(preset: ProviderPreset): ConfiguredProviderDraft | null {
  if (!preset.supported || !preset.protocol) return null;
  return {
    ...emptyProviderDraft(),
    providerType: preset.id, displayName: preset.name, protocol: preset.protocol, baseUrl: preset.baseUrl ?? "", models: [],
  };
}

export function presetModelToDraft(model: ProviderPresetModel, _protocol: ConfiguredProviderProtocol): ProviderModelDraft {
  const allowed = ["text", "image"];
  const capabilityNames = { text: "Text", image: "Image", audio: "Audio", video: "Video" } as const;
  const inputs: ProviderInputCapability[] = model.inputCapabilities
    .filter((value) => allowed.includes(value)).map((value) => capabilityNames[value]);
  return {
    ...emptyProviderModel(), modelId: model.modelId, name: model.name,
    contextWindowTokens: model.contextWindowTokens?.toString() ?? "", reasoning: model.reasoning,
    inputCapabilities: [...new Set<ProviderInputCapability>(["Text", ...inputs])],
  };
}

const forbiddenHeaders = new Set([
  "authorization", "proxy-authorization", "x-api-key", "api-key", "x-goog-api-key", "cookie", "set-cookie",
  "host", "content-length", "connection", "transfer-encoding", "upgrade", "trailer", "te",
]);

export type ProviderDraftError = "displayName" | "baseUrl" | "modelsRequired" | "modelId" | "modelName" | "duplicateModel" | "tokenLimit" | "outputLimit" | "headers" | "inputCapabilities" | "modelType" | "dimension";

function tokenLimit(value: string): number | null {
  return value.trim() === "" ? null : Number(value);
}

export function validateProviderDraft(draft: ConfiguredProviderDraft): ProviderDraftError | null {
  if (!draft.displayName.trim()) return "displayName";
  try {
    const url = new URL(draft.baseUrl.trim());
    if (!["http:", "https:"].includes(url.protocol) || url.username || url.password || url.search || url.hash) return "baseUrl";
  } catch { return "baseUrl"; }
  if (!draft.models.length) return "modelsRequired";
  const ids = new Set<string>();
  for (const model of draft.models) {
    if (!supportsModelType(model.modelType, draft.protocol)) return "modelType";
    if (["TextEmbedding", "MultimodalEmbedding"].includes(model.modelType) && (!/^\d+$/.test(model.dimension) || !Number.isSafeInteger(Number(model.dimension)) || Number(model.dimension) < 1 || Number(model.dimension) > 4294967295)) return "dimension";
    if (!model.modelId.trim()) return "modelId";
    if (!model.name.trim()) return "modelName";
    if (ids.has(model.modelId.trim())) return "duplicateModel";
    ids.add(model.modelId.trim());
    const context = tokenLimit(model.contextWindowTokens), output = tokenLimit(model.maxOutputTokens);
    if ([context, output].some((value) => value !== null && (!Number.isSafeInteger(value) || value <= 0 || value > 4_294_967_295))) return "tokenLimit";
    if (context !== null && output !== null && output > context) return "outputLimit";
    if (["TextEmbedding", "TextRerank"].includes(model.modelType) && (model.inputCapabilities.length !== 1 || model.inputCapabilities[0] !== "Text")) return "inputCapabilities";
    if (isMultimodalModel(model.modelType) && !model.inputCapabilities.some(value => value !== "Text")) return "inputCapabilities";
    if (!model.inputCapabilities.length || (!isMediaModel(model.modelType) && !isMultimodalModel(model.modelType) && (!model.inputCapabilities.includes("Text") || model.inputCapabilities.some((value) => value === "Audio" || value === "Video")))) return "inputCapabilities";
  }
  const headerNames = new Set<string>();
  for (const header of draft.headers) {
    const name = header.name.trim().toLowerCase();
    if (!/^[!#$%&'*+.^_`|~0-9a-z-]+$/.test(name) || forbiddenHeaders.has(name) || headerNames.has(name) || /[\r\n\0]/.test(header.value)) return "headers";
    headerNames.add(name);
  }
  return null;
}

export function providerDraftToPayload(draft: ConfiguredProviderDraft): ConfiguredProviderCreate {
  return {
    providerType: draft.providerType, displayName: draft.displayName.trim(), protocol: draft.protocol,
    baseUrl: draft.baseUrl.trim(), apiKey: draft.apiKey.trim() || null, enabled: draft.enabled,
    requestHeaders: Object.fromEntries(draft.headers.map((header) => [header.name.trim(), header.value])),
    models: draft.models.map(({ rowId: _rowId, dimension, contextWindowTokens, maxOutputTokens, ...model }) => ({
      dimension: ["TextEmbedding", "MultimodalEmbedding"].includes(model.modelType) ? tokenLimit(dimension) : null,
      ...model, modelId: model.modelId.trim(), name: model.name.trim(),
      contextWindowTokens: tokenLimit(contextWindowTokens), maxOutputTokens: tokenLimit(maxOutputTokens),
    })),
  };
}

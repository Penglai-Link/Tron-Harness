import type {
  ConfiguredProvider,
  ConfiguredProviderCreate,
  ConfiguredProviderModel,
  ConfiguredProviderProtocol,
  ProviderInputCapability,
  ProviderPreset,
  ProviderPresetModel,
} from "@/domain/model/api/configuredProviderApi";

export const configuredProviderProtocols: ConfiguredProviderProtocol[] = [
  "OpenAiChatCompletions", "OpenAiResponses", "AnthropicMessages", "GoogleGemini",
];

export interface ProviderModelDraft extends Omit<ConfiguredProviderModel, "contextWindowTokens" | "maxOutputTokens"> {
  rowId: string;
  contextWindowTokens: string;
  maxOutputTokens: string;
}

export interface ProviderHeaderDraft { rowId: string; name: string; value: string }

export interface ConfiguredProviderDraft {
  providerId: string;
  displayName: string;
  protocol: ConfiguredProviderProtocol;
  baseUrl: string;
  apiKey: string;
  hasApiKey: boolean;
  enabled: boolean;
  headers: ProviderHeaderDraft[];
  models: ProviderModelDraft[];
}

let draftSequence = 0;
export function draftRowId() { return `provider-draft-${++draftSequence}`; }

export function emptyProviderModel(): ProviderModelDraft {
  return {
    rowId: draftRowId(), modelCode: null, modelId: "", name: "", contextWindowTokens: "",
    maxOutputTokens: "", reasoning: false, supportsDeveloperRole: false,
    inputCapabilities: ["Text"], enabled: true,
  };
}

export function emptyProviderDraft(): ConfiguredProviderDraft {
  return {
    providerId: "", displayName: "", protocol: "OpenAiChatCompletions", baseUrl: "",
    apiKey: "", hasApiKey: false, enabled: true, headers: [], models: [emptyProviderModel()],
  };
}

export function configuredProviderToDraft(record: ConfiguredProvider): ConfiguredProviderDraft {
  return {
    ...record, apiKey: "",
    headers: Object.entries(record.requestHeaders).map(([name, value]) => ({ rowId: draftRowId(), name, value })),
    models: record.models.map((model) => ({
      ...model, rowId: draftRowId(), contextWindowTokens: model.contextWindowTokens?.toString() ?? "",
      maxOutputTokens: model.maxOutputTokens?.toString() ?? "",
    })),
  };
}

export function presetToDraft(preset: ProviderPreset): ConfiguredProviderDraft | null {
  if (!preset.supported || !preset.protocol) return null;
  return {
    ...emptyProviderDraft(), providerId: preset.id.toLowerCase().replace(/[^a-z0-9._-]/g, "-"),
    displayName: preset.name, protocol: preset.protocol, baseUrl: preset.baseUrl ?? "", models: [],
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

export type ProviderDraftError = "providerId" | "displayName" | "baseUrl" | "modelsRequired" | "modelId" | "modelName" | "duplicateModel" | "tokenLimit" | "outputLimit" | "headers" | "inputCapabilities";

function tokenLimit(value: string): number | null {
  return value.trim() === "" ? null : Number(value);
}

export function validateProviderDraft(draft: ConfiguredProviderDraft): ProviderDraftError | null {
  if (!/^[a-z0-9][a-z0-9._-]{0,79}$/.test(draft.providerId) || draft.providerId === "personai") return "providerId";
  if (!draft.displayName.trim()) return "displayName";
  try {
    const url = new URL(draft.baseUrl.trim());
    if (!["http:", "https:"].includes(url.protocol) || url.username || url.password || url.search || url.hash) return "baseUrl";
  } catch { return "baseUrl"; }
  if (!draft.models.length) return "modelsRequired";
  const ids = new Set<string>();
  for (const model of draft.models) {
    if (!model.modelId.trim()) return "modelId";
    if (!model.name.trim()) return "modelName";
    if (ids.has(model.modelId.trim())) return "duplicateModel";
    ids.add(model.modelId.trim());
    const context = tokenLimit(model.contextWindowTokens), output = tokenLimit(model.maxOutputTokens);
    if ([context, output].some((value) => value !== null && (!Number.isSafeInteger(value) || value <= 0 || value > 4_294_967_295))) return "tokenLimit";
    if (context !== null && output !== null && output > context) return "outputLimit";
    if (!model.inputCapabilities.includes("Text") || model.inputCapabilities.some((value) => value === "Audio" || value === "Video")) return "inputCapabilities";
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
    providerId: draft.providerId, displayName: draft.displayName.trim(), protocol: draft.protocol,
    baseUrl: draft.baseUrl.trim(), apiKey: draft.apiKey.trim() || null, enabled: draft.enabled,
    requestHeaders: Object.fromEntries(draft.headers.map((header) => [header.name.trim(), header.value])),
    models: draft.models.map(({ rowId: _rowId, contextWindowTokens, maxOutputTokens, ...model }) => ({
      ...model, modelId: model.modelId.trim(), name: model.name.trim(),
      contextWindowTokens: tokenLimit(contextWindowTokens), maxOutputTokens: tokenLimit(maxOutputTokens),
    })),
  };
}

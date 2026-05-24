import type {
  ModelCapability,
  ModelProtocol,
  ModelSourceKind,
  ModelType,
  ModelVerifyStatus,
} from "@/domain/model/model";

export interface ModelCapabilityOption {
  label: string;
  modelType: ModelType;
  value: ModelCapability;
}

export interface ModelCapabilityGroup {
  label: string;
  options: ModelCapabilityOption[];
}

export const modelTypeSequence: ModelType[] = ["TEXT", "EMBEDDING", "RERANK", "IMAGE"];

const modelCapabilityGroups: ModelCapabilityGroup[] = [
  {
    label: "Multimodal",
    options: [
      { label: "All Modalities", modelType: "TEXT", value: "MULTIMODAL_ALL" },
    ],
  },
  {
    label: "Text",
    options: [
      { label: "Text Generation", modelType: "TEXT", value: "TEXT_GENERATION" },
      { label: "Deep Thinking", modelType: "TEXT", value: "DEEP_THINKING" },
    ],
  },
  {
    label: "Vision",
    options: [
      { label: "Vision Understanding", modelType: "IMAGE", value: "VISION_UNDERSTANDING" },
      { label: "Image Generation", modelType: "IMAGE", value: "IMAGE_GENERATION" },
      { label: "Video Generation", modelType: "IMAGE", value: "VIDEO_GENERATION" },
      { label: "3D Generation", modelType: "IMAGE", value: "THREED_GENERATION" },
    ],
  },
  {
    label: "Speech",
    options: [
      { label: "Speech Recognition", modelType: "TEXT", value: "SPEECH_RECOGNITION" },
      { label: "Speech Synthesis", modelType: "TEXT", value: "SPEECH_SYNTHESIS" },
    ],
  },
  {
    label: "Vectors",
    options: [
      { label: "Multimodal Vectors", modelType: "EMBEDDING", value: "MULTIMODAL_VECTOR" },
      { label: "Text Vectors", modelType: "EMBEDDING", value: "TEXT_VECTOR" },
    ],
  },
  {
    label: "Realtime",
    options: [
      { label: "Realtime All Modalities", modelType: "TEXT", value: "REALTIME_MULTIMODAL" },
      { label: "Realtime Speech Synthesis", modelType: "TEXT", value: "REALTIME_SPEECH_SYNTHESIS" },
      { label: "Realtime Speech Recognition", modelType: "TEXT", value: "REALTIME_SPEECH_RECOGNITION" },
      { label: "Realtime Speech Translation", modelType: "TEXT", value: "REALTIME_SPEECH_TRANSLATION" },
    ],
  },
  {
    label: "Ranking",
    options: [
      { label: "Rerank", modelType: "RERANK", value: "RERANK" },
    ],
  },
];

const defaultModelCapabilitiesByType: Record<ModelType, ModelCapability[]> = {
  TEXT: ["TEXT_GENERATION"],
  EMBEDDING: ["TEXT_VECTOR"],
  RERANK: ["RERANK"],
  IMAGE: ["IMAGE_GENERATION"],
};

const modelTypeLabelByType: Record<ModelType, string> = {
  TEXT: "Text",
  EMBEDDING: "Embedding",
  RERANK: "Rerank",
  IMAGE: "Image",
};

const modelCapabilityOptionMap = new Map(
  modelCapabilityGroups.flatMap((group) => group.options.map((option) => [option.value, option] as const)),
);

const validModelTypesByProtocol: Record<ModelProtocol, ModelType[]> = {
  OPENAI_COMPATIBLE: ["TEXT", "EMBEDDING"],
  RESPONSES: ["TEXT", "IMAGE"],
  RERANK: ["RERANK"],
};

export function formatModelProtocol(value: ModelProtocol, modelType?: ModelType) {
  switch (value) {
    case "OPENAI_COMPATIBLE":
      return modelType === "EMBEDDING"
        ? "OpenAI Compatible - Embedding"
        : "OpenAI Compatible - Chat";
    case "RESPONSES":
      return "OpenAI Compatible - Response";
    case "RERANK":
      return "OpenAI Compatible - Rerank";
    default:
      return value;
  }
}

export function formatModelType(value: ModelType) {
  return modelTypeLabelByType[value] ?? value;
}

export function formatModelCapability(value: ModelCapability) {
  return modelCapabilityOptionMap.get(value)?.label ?? value;
}

export function formatModelCapabilities(
  values: ModelCapability[] | null | undefined,
  fallbackModelType: ModelType,
) {
  const normalized = normalizeModelCapabilities(values, fallbackModelType);
  return normalized.map((value) => formatModelCapability(value)).join(", ");
}

export function getModelProtocolOptions(modelType: ModelType): Array<{ label: string; value: ModelProtocol }> {
  return [
    { label: formatModelProtocol("OPENAI_COMPATIBLE", modelType), value: "OPENAI_COMPATIBLE" },
    { label: formatModelProtocol("RESPONSES", modelType), value: "RESPONSES" },
    { label: formatModelProtocol("RERANK", modelType), value: "RERANK" },
  ];
}

export function formatModelVerifyStatus(value: ModelVerifyStatus | null) {
  if (value === "SUCCEEDED") return "Verified";
  if (value === "FAILED") return "Failed";
  return "Never";
}

export function getModelVerifyTone(value: ModelVerifyStatus | null) {
  if (value === "SUCCEEDED") return "success" as const;
  if (value === "FAILED") return "error" as const;
  return "neutral" as const;
}

export function getModelSourceTone(sourceKind: ModelSourceKind) {
  switch (sourceKind) {
    case "LOCAL":
      return "accent" as const;
    case "INHERITED":
      return "neutral" as const;
    case "OVERRIDE":
      return "warning" as const;
    default:
      return "neutral" as const;
  }
}

export function getValidModelTypesForProtocol(protocol: ModelProtocol) {
  return validModelTypesByProtocol[protocol];
}

export function getDefaultModelCapabilitiesForType(modelType: ModelType): ModelCapability[] {
  return [...(defaultModelCapabilitiesByType[modelType] ?? defaultModelCapabilitiesByType.TEXT)];
}

export function getModelTypeFromCapability(capability: ModelCapability): ModelType {
  return modelCapabilityOptionMap.get(capability)?.modelType ?? "TEXT";
}

export function normalizeModelCapabilities(
  values: ModelCapability[] | null | undefined,
  fallbackModelType: ModelType,
): ModelCapability[] {
  const source = Array.isArray(values) ? values : [];
  const seeded = source.length ? source : getDefaultModelCapabilitiesForType(fallbackModelType);
  const normalized: ModelCapability[] = [];
  seeded.forEach((value) => {
    if (!normalized.includes(value)) normalized.push(value);
  });
  return normalized.length ? normalized : getDefaultModelCapabilitiesForType(fallbackModelType);
}

export function getPrimaryModelTypeFromCapabilities(
  protocol: ModelProtocol,
  capabilities: ModelCapability[] | null | undefined,
  fallbackModelType: ModelType,
): ModelType {
  const normalized = normalizeModelCapabilities(capabilities, fallbackModelType);
  const selectedTypes = new Set(normalized.map((value) => getModelTypeFromCapability(value)));
  if (selectedTypes.has(fallbackModelType)) {
    return fallbackModelType;
  }
  const validModelTypes = getValidModelTypesForProtocol(protocol);
  return validModelTypes.find((value) => selectedTypes.has(value)) ?? fallbackModelType;
}

export function getModelCapabilityGroups(protocol: ModelProtocol): ModelCapabilityGroup[] {
  const validModelTypes = new Set(validModelTypesByProtocol[protocol]);
  return modelCapabilityGroups
    .map((group) => ({
      ...group,
      options: group.options.filter((option) => validModelTypes.has(option.modelType)),
    }))
    .filter((group) => group.options.length > 0);
}

export function getDefaultApiPathForProtocol(protocol: ModelProtocol, modelType: ModelType = "TEXT") {
  switch (protocol) {
    case "OPENAI_COMPATIBLE":
      if (modelType === "EMBEDDING") return "/embeddings";
      return "/chat/completions";
    case "RERANK":
      return "/rerank";
    default:
      return "/responses";
  }
}

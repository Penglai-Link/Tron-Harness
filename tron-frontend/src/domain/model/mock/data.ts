import type {
  ModelMutationPayload,
  PlatformDefaultMutationResult,
  WorkspaceDefaultMutationResult,
} from "@/domain/model/api";
import type {
  ModelCapability,
  ModelProtocol,
  ModelRecord,
  ReasoningEffort,
  ModelSelectOption,
  ModelSourceKind,
  ModelType,
  ModelVerifyStatus,
  WorkspaceDefaultModelRecord,
  WorkspaceModelCatalog,
} from "@/domain/model/model";
import {
  getDefaultModelCapabilitiesForType,
  getPrimaryModelTypeFromCapabilities,
  normalizeModelCapabilities,
  normalizeReasoningEfforts,
} from "@/domain/model/constants/modelPageOptions";

const now = "2026-05-20T12:00:00Z";
const MASKED_API_KEY_VALUE = "*****";
const modelTypeSequence: ModelType[] = ["TEXT", "EMBEDDING", "RERANK", "IMAGE"];

type WorkspaceLineageEntry = {
  label: string;
  name: string;
  workspaceCode: string;
};

type BaseModel = {
  apiKeyRef: string;
  apiPath: string | null;
  baseUrl: string;
  createdAt: string;
  description: string | null;
  displayName: string;
  enabled: boolean;
  lastVerifiedAt: string | null;
  lastVerifyMessage: string | null;
  lastVerifyStatus: ModelVerifyStatus | null;
  contextWindowTokens: number | null;
  maxRetries: number | null;
  maxTokens: number | null;
  modelCapabilities: ModelCapability[];
  modelCode: string;
  modelName: string;
  modelType: ModelType;
  protocol: ModelProtocol;
  structuredOutputEnabled: boolean;
  responsesStatefulEnabled: boolean;
  temperature: number | null;
  thinkingEnabled: boolean;
  supportedReasoningEfforts: ReasoningEffort[];
  defaultReasoningEffort: ReasoningEffort | null;
  topP: number | null;
  updatedAt: string;
  vendor: string;
};

type PlatformModel = BaseModel;

type WorkspaceLocalModel = BaseModel & {
  allowInheritance: boolean;
  workspaceCode: string;
};

type WorkspaceOverrideModel = BaseModel & {
  slotModelCode: string;
  workspaceCode: string;
};

type WorkspaceStatusPatch = {
  createdAt: string;
  enabled: boolean;
  modelCode: string;
  slotModelCode: string;
  updatedAt: string;
  workspaceCode: string;
};

type EffectiveProvider = {
  enabled: boolean;
  model: BaseModel;
  workspaceCode: string | null;
  workspaceName: string | null;
  workspacePath: WorkspaceLineageEntry[];
};

type DefinitionPointer =
  | { kind: "platform"; modelCode: string }
  | { kind: "local"; modelCode: string }
  | { kind: "override"; modelCode: string };

type OwnedPointer =
  | { kind: "local"; modelCode: string }
  | { kind: "override"; modelCode: string }
  | { kind: "statusPatch"; modelCode: string }
  | null;

type ResolvedWorkspaceRecord = {
  currentOwned: OwnedPointer;
  definition: DefinitionPointer;
  upstreamEnabled: boolean | null;
  view: ModelRecord;
};

const workspaceLineages: Record<string, WorkspaceLineageEntry[]> = {
  ENTERPRISE: [
    { workspaceCode: "ENTERPRISE", name: "Enterprise", label: "Enterprise" },
  ],
  PLATFORM: [
    { workspaceCode: "ENTERPRISE", name: "Enterprise", label: "Enterprise" },
    { workspaceCode: "PLATFORM", name: "Platform", label: "Enterprise / Platform" },
  ],
  TRON_CORE: [
    { workspaceCode: "ENTERPRISE", name: "Enterprise", label: "Enterprise" },
    { workspaceCode: "PLATFORM", name: "Platform", label: "Enterprise / Platform" },
    { workspaceCode: "TRON_CORE", name: "Tron Core", label: "Enterprise / Platform / Tron Core" },
  ],
  COMPLIANCE: [
    { workspaceCode: "COMPLIANCE", name: "Compliance", label: "Compliance" },
  ],
};

let nextMockModelSequence = 1000;

let platformModels = buildInitialPlatformModels();
let workspaceLocalModels = buildInitialWorkspaceLocalModels();
let workspaceOverrides: WorkspaceOverrideModel[] = [];
let workspaceStatusPatches: WorkspaceStatusPatch[] = [];
let platformDefaults = buildInitialPlatformDefaults();
let workspaceDefaults = buildInitialWorkspaceDefaults();

function createSeedBaseModel(
  modelCode: string,
  displayName: string,
  vendor: string,
  protocol: ModelProtocol,
  modelType: ModelType,
  baseUrl: string,
  apiPath: string | null,
  apiKeyRef: string,
  modelName: string,
): BaseModel {
  return {
    apiKeyRef,
    apiPath,
    baseUrl,
    createdAt: now,
    description: `${displayName} model.`,
    displayName,
    enabled: true,
    lastVerifiedAt: null,
    lastVerifyMessage: null,
    lastVerifyStatus: null,
    contextWindowTokens: modelType === "TEXT" ? 128000 : null,
    maxRetries: 1,
    maxTokens: modelType === "TEXT" ? 4096 : null,
    modelCapabilities: getDefaultModelCapabilitiesForType(modelType),
    modelCode,
    modelName,
    modelType,
    protocol,
    structuredOutputEnabled: protocol === "RESPONSES",
    responsesStatefulEnabled: false,
    temperature: modelType === "TEXT" ? 0.2 : null,
    thinkingEnabled: modelType === "TEXT",
    supportedReasoningEfforts: protocol === "RESPONSES" && modelType === "TEXT"
      ? ["Low", "Medium", "High"]
      : [],
    defaultReasoningEffort: protocol === "RESPONSES" && modelType === "TEXT"
      ? "Medium"
      : null,
    topP: modelType === "TEXT" ? 1 : null,
    updatedAt: now,
    vendor,
  };
}

function buildInitialPlatformModels(): PlatformModel[] {
  return [
    createSeedBaseModel("MODEL-734600000000000001", "GPT-4o Main", "OpenAI", "RESPONSES", "TEXT", "https://api.openai.com/v1", "/responses", "sk-platform-openai-gpt4o-demo", "gpt-4o"),
    createSeedBaseModel("MODEL-734600000000000002", "Embedding Shared", "OpenAI", "OPENAI_COMPATIBLE_EMBEDDING", "EMBEDDING", "https://api.openai.com/v1", "/embeddings", "sk-platform-openai-embedding-demo", "text-embedding-3-small"),
    createSeedBaseModel("MODEL-734600000000000003", "Rerank Default", "Cohere", "RERANK", "RERANK", "https://api.cohere.ai/v1", "/rerank", "sk-platform-cohere-rerank-demo", "rerank-v3.5"),
  ];
}

function buildInitialWorkspaceLocalModels(): WorkspaceLocalModel[] {
  return [
    {
      ...createSeedBaseModel("MODEL-734600000000000005", "Private Image Lab", "OpenAI", "RESPONSES", "IMAGE", "https://api.openai.com/v1", "/responses", "sk-workspace-platform-image-demo", "gpt-image-1"),
      allowInheritance: false,
      workspaceCode: "PLATFORM",
    },
    {
      ...createSeedBaseModel("MODEL-734600000000000007", "Core Fast", "OpenAI-Compatible", "OPENAI_COMPATIBLE", "TEXT", "https://router.tron.ai/v1", "/chat/completions", "sk-workspace-tron-core-fast-demo", "gpt-4.1-mini"),
      allowInheritance: true,
      workspaceCode: "TRON_CORE",
    },
  ];
}

function buildInitialPlatformDefaults(): Partial<Record<ModelType, string>> {
  return {
    TEXT: "MODEL-734600000000000001",
    EMBEDDING: "MODEL-734600000000000002",
    RERANK: "MODEL-734600000000000003",
  };
}

function buildInitialWorkspaceDefaults(): Record<string, Partial<Record<ModelType, string>>> {
  return {
    COMPLIANCE: {},
    ENTERPRISE: {},
    PLATFORM: {},
    TRON_CORE: {},
  };
}

function currentWorkspaceEntry(workspaceCode: string) {
  const lineage = workspaceLineages[workspaceCode];
  if (!lineage?.length) {
    throw new Error(`Workspace ${workspaceCode} was not found.`);
  }
  return lineage[lineage.length - 1];
}

function workspaceName(workspaceCode: string) {
  return currentWorkspaceEntry(workspaceCode).name;
}

function generateModelCode() {
  nextMockModelSequence += 1;
  return `MODEL-MOCK-${nextMockModelSequence}`;
}

function cloneBaseModel<T extends BaseModel>(model: T): T {
  return {
    ...model,
    modelCapabilities: [...model.modelCapabilities],
    supportedReasoningEfforts: [...model.supportedReasoningEfforts],
  };
}

function buildSourceLabel(sourceKind: ModelSourceKind) {
  switch (sourceKind) {
    case "LOCAL":
      return "Local";
    case "INHERITED":
      return "Inherited";
    case "OVERRIDE":
      return "Override";
    default:
      return "Platform";
  }
}

function buildPlatformView(model: PlatformModel): ModelRecord {
  const defaults = resolveEffectivePlatformDefaults();
  return {
    scopeKind: "PLATFORM",
    workspaceCode: null,
    modelCode: model.modelCode,
    displayName: model.displayName,
    vendor: model.vendor,
    protocol: model.protocol,
    modelType: model.modelType,
    modelCapabilities: [...model.modelCapabilities],
    baseUrl: model.baseUrl,
    apiPath: model.apiPath,
    apiKeyRef: maskApiKeyValue(model.apiKeyRef),
    modelName: model.modelName,
    enabled: model.enabled,
    description: model.description,
    thinkingEnabled: model.thinkingEnabled,
    supportedReasoningEfforts: [...model.supportedReasoningEfforts],
    defaultReasoningEffort: model.defaultReasoningEffort,
    structuredOutputEnabled: model.structuredOutputEnabled,
    responsesStatefulEnabled: model.responsesStatefulEnabled,
    temperature: model.temperature,
    maxTokens: model.maxTokens,
    contextWindowTokens: model.contextWindowTokens,
    topP: model.topP,
    maxRetries: model.maxRetries,
    lastVerifiedAt: model.lastVerifiedAt,
    lastVerifyStatus: model.lastVerifyStatus,
    lastVerifyMessage: model.lastVerifyMessage,
    sourceKind: "PLATFORM",
    sourceModelCode: null,
    sourceLabel: "Platform",
    sourceWorkspaceCode: null,
    sourceWorkspaceName: null,
    sourceWorkspacePath: [],
    allowInheritance: null,
    isDefault: defaults[model.modelType] === model.modelCode && model.enabled,
    createdAt: model.createdAt,
    updatedAt: model.updatedAt,
  };
}

function buildWorkspaceView(
  workspaceCode: string,
  slotModelCode: string,
  configModel: BaseModel,
  sourceKind: Exclude<ModelSourceKind, "PLATFORM">,
  inheritedFrom: EffectiveProvider | null,
  enabled: boolean,
  allowInheritance: boolean | null,
  timestamps: { createdAt: string; updatedAt: string },
): ModelRecord {
  return {
    scopeKind: "WORKSPACE",
    workspaceCode,
    modelCode: slotModelCode,
    displayName: configModel.displayName,
    vendor: configModel.vendor,
    protocol: configModel.protocol,
    modelType: configModel.modelType,
    modelCapabilities: [...configModel.modelCapabilities],
    baseUrl: configModel.baseUrl,
    apiPath: configModel.apiPath,
    apiKeyRef: maskApiKeyValue(configModel.apiKeyRef),
    modelName: configModel.modelName,
    enabled,
    description: configModel.description,
    thinkingEnabled: configModel.thinkingEnabled,
    supportedReasoningEfforts: [...configModel.supportedReasoningEfforts],
    defaultReasoningEffort: configModel.defaultReasoningEffort,
    structuredOutputEnabled: configModel.structuredOutputEnabled,
    responsesStatefulEnabled: configModel.responsesStatefulEnabled,
    temperature: configModel.temperature,
    maxTokens: configModel.maxTokens,
    contextWindowTokens: configModel.contextWindowTokens,
    topP: configModel.topP,
    maxRetries: configModel.maxRetries,
    lastVerifiedAt: configModel.lastVerifiedAt,
    lastVerifyStatus: configModel.lastVerifyStatus,
    lastVerifyMessage: configModel.lastVerifyMessage,
    sourceKind,
    sourceModelCode: sourceKind === "LOCAL" ? null : slotModelCode,
    sourceLabel: buildSourceLabel(sourceKind),
    sourceWorkspaceCode: inheritedFrom?.workspaceCode ?? null,
    sourceWorkspaceName: inheritedFrom?.workspaceName ?? null,
    sourceWorkspacePath: inheritedFrom?.workspacePath.map((entry) => ({
      workspaceCode: entry.workspaceCode,
      name: entry.name,
    })) ?? [],
    allowInheritance,
    isDefault: false,
    createdAt: timestamps.createdAt,
    updatedAt: timestamps.updatedAt,
  };
}

function maskApiKeyValue(value: string) {
  return value.trim() ? MASKED_API_KEY_VALUE : "";
}

function resolveEffectivePlatformDefaults() {
  return modelTypeSequence.reduce<Partial<Record<ModelType, string>>>((defaults, modelType) => {
    const explicit = platformDefaults[modelType];
    const explicitModel = explicit
      ? platformModels.find((model) => model.modelCode === explicit && model.enabled && model.modelType === modelType)
      : null;
    if (explicitModel) {
      defaults[modelType] = explicitModel.modelCode;
      return defaults;
    }

    const fallback = platformModels.find((model) => model.enabled && model.modelType === modelType);
    if (fallback) {
      defaults[modelType] = fallback.modelCode;
    }
    return defaults;
  }, {});
}

function platformProvider(model: BaseModel): EffectiveProvider {
  return {
    enabled: model.enabled,
    model: cloneBaseModel(model),
    workspaceCode: null,
    workspaceName: null,
    workspacePath: [],
  };
}

function workspaceProvider(model: BaseModel, workspaceCode: string): EffectiveProvider {
  const lineage = workspaceLineages[workspaceCode] ?? [];
  return {
    enabled: model.enabled,
    model: cloneBaseModel(model),
    workspaceCode,
    workspaceName: workspaceName(workspaceCode),
    workspacePath: [...lineage],
  };
}

function statusPatchProvider(
  definitionModel: BaseModel,
  workspaceCode: string,
  enabled: boolean,
): EffectiveProvider {
  return {
    enabled,
    model: cloneBaseModel(definitionModel),
    workspaceCode,
    workspaceName: workspaceName(workspaceCode),
    workspacePath: [...(workspaceLineages[workspaceCode] ?? [])],
  };
}

function findWorkspaceOverride(workspaceCode: string, slotModelCode: string) {
  return workspaceOverrides.find((record) => (
    record.workspaceCode === workspaceCode
    && record.slotModelCode === slotModelCode
  ));
}

function findWorkspaceStatusPatch(workspaceCode: string, slotModelCode: string) {
  return workspaceStatusPatches.find((record) => (
    record.workspaceCode === workspaceCode
    && record.slotModelCode === slotModelCode
  ));
}

function buildResolvedWorkspaceRecord(
  workspaceCode: string,
  root:
    | { kind: "platform"; model: PlatformModel }
    | { kind: "local"; model: WorkspaceLocalModel },
): ResolvedWorkspaceRecord {
  const slotModelCode = root.model.modelCode;

  if (root.kind === "local" && root.model.workspaceCode === workspaceCode) {
    return {
      currentOwned: { kind: "local", modelCode: root.model.modelCode },
      definition: { kind: "local", modelCode: root.model.modelCode },
      upstreamEnabled: null,
      view: buildWorkspaceView(
        workspaceCode,
        slotModelCode,
        root.model,
        "LOCAL",
        null,
        root.model.enabled,
        root.model.allowInheritance,
        root.model,
      ),
    };
  }

  const lineage = workspaceLineages[workspaceCode] ?? [];
  const currentLineageIndex = lineage.findIndex((entry) => entry.workspaceCode === workspaceCode);
  const ownerWorkspaceCode = root.kind === "local" ? root.model.workspaceCode : null;
  const ownerIndex = ownerWorkspaceCode
    ? lineage.findIndex((entry) => entry.workspaceCode === ownerWorkspaceCode)
    : -1;
  const ancestorEntries = root.kind === "platform"
    ? lineage.slice(0, currentLineageIndex)
    : lineage.slice(ownerIndex + 1, currentLineageIndex);

  let definitionProvider = root.kind === "platform"
    ? platformProvider(root.model)
    : workspaceProvider(root.model, root.model.workspaceCode);
  let inheritedProvider = definitionProvider;

  ancestorEntries.forEach((ancestor) => {
    const ancestorOverride = findWorkspaceOverride(ancestor.workspaceCode, slotModelCode);
    if (ancestorOverride) {
      definitionProvider = workspaceProvider(ancestorOverride, ancestor.workspaceCode);
      inheritedProvider = definitionProvider;
      return;
    }
    const ancestorPatch = findWorkspaceStatusPatch(ancestor.workspaceCode, slotModelCode);
    if (ancestorPatch) {
      inheritedProvider = statusPatchProvider(
        definitionProvider.model,
        ancestor.workspaceCode,
        ancestorPatch.enabled,
      );
    }
  });

  const currentOverride = findWorkspaceOverride(workspaceCode, slotModelCode);
  if (currentOverride) {
    return {
      currentOwned: { kind: "override", modelCode: currentOverride.modelCode },
      definition: { kind: "override", modelCode: currentOverride.modelCode },
      upstreamEnabled: inheritedProvider.enabled,
      view: buildWorkspaceView(
        workspaceCode,
        slotModelCode,
        currentOverride,
        "OVERRIDE",
        inheritedProvider,
        currentOverride.enabled,
        null,
        currentOverride,
      ),
    };
  }

  const currentPatch = findWorkspaceStatusPatch(workspaceCode, slotModelCode);
  if (currentPatch) {
    return {
      currentOwned: { kind: "statusPatch", modelCode: currentPatch.modelCode },
      definition: resolveDefinitionPointer(definitionProvider.model.modelCode),
      upstreamEnabled: inheritedProvider.enabled,
      view: buildWorkspaceView(
        workspaceCode,
        slotModelCode,
        definitionProvider.model,
        "INHERITED",
        inheritedProvider,
        currentPatch.enabled,
        null,
        currentPatch,
      ),
    };
  }

  return {
    currentOwned: null,
    definition: resolveDefinitionPointer(definitionProvider.model.modelCode),
    upstreamEnabled: inheritedProvider.enabled,
    view: buildWorkspaceView(
      workspaceCode,
      slotModelCode,
      definitionProvider.model,
      "INHERITED",
      inheritedProvider,
      inheritedProvider.enabled,
      null,
      definitionProvider.model,
    ),
  };
}

function resolveDefinitionPointer(modelCode: string): DefinitionPointer {
  if (platformModels.some((model) => model.modelCode === modelCode)) {
    return { kind: "platform", modelCode };
  }
  if (workspaceOverrides.some((model) => model.modelCode === modelCode)) {
    return { kind: "override", modelCode };
  }
  return { kind: "local", modelCode };
}

function sortModelRecords(records: ModelRecord[]) {
  return [...records].sort((left, right) => (
    left.modelType.localeCompare(right.modelType)
    || left.displayName.localeCompare(right.displayName)
    || left.sourceLabel.localeCompare(right.sourceLabel)
    || left.modelCode.localeCompare(right.modelCode)
  ));
}

function resolveWorkspaceCatalogState(workspaceCode: string) {
  currentWorkspaceEntry(workspaceCode);
  const lineageCodes = new Set((workspaceLineages[workspaceCode] ?? []).map((entry) => entry.workspaceCode));
  const roots: Array<
    | { kind: "platform"; model: PlatformModel }
    | { kind: "local"; model: WorkspaceLocalModel }
  > = [
    ...platformModels.map((model) => ({ kind: "platform" as const, model })),
    ...workspaceLocalModels
      .filter((model) => (
        model.workspaceCode === workspaceCode
        || (lineageCodes.has(model.workspaceCode) && model.allowInheritance)
      ))
      .map((model) => ({ kind: "local" as const, model })),
  ];

  const resolvedRecords = roots.map((root) => buildResolvedWorkspaceRecord(workspaceCode, root));
  const sortedViews = sortModelRecords(resolvedRecords.map((record) => record.view));
  const sortedRecords = sortedViews.map((view) => (
    resolvedRecords.find((record) => record.view.modelCode === view.modelCode)!
  ));

  const recordMap = new Map(sortedRecords.map((record) => [record.view.modelCode, record.view]));
  const platformDefaultMap = resolveEffectivePlatformDefaults();
  const resolutionOrder = [
    workspaceCode,
    ...(workspaceLineages[workspaceCode] ?? [])
      .map((entry) => entry.workspaceCode)
      .reverse()
      .filter((code) => code !== workspaceCode),
  ];

  const defaults = modelTypeSequence.map((modelType) => {
    for (const candidateWorkspaceCode of resolutionOrder) {
      const assignedCode = workspaceDefaults[candidateWorkspaceCode]?.[modelType];
      if (!assignedCode) continue;
      const matching = recordMap.get(assignedCode);
      if (matching && matching.enabled && matching.modelType === modelType) {
        return {
          modelType,
          modelCode: matching.modelCode,
          displayName: matching.displayName,
          sourceLabel: matching.sourceLabel,
        } satisfies WorkspaceDefaultModelRecord;
      }
    }

    const platformDefaultCode = platformDefaultMap[modelType];
    const matching = platformDefaultCode ? recordMap.get(platformDefaultCode) : null;
    if (matching && matching.enabled && matching.modelType === modelType) {
      return {
        modelType,
        modelCode: matching.modelCode,
        displayName: matching.displayName,
        sourceLabel: matching.sourceLabel,
      } satisfies WorkspaceDefaultModelRecord;
    }

    return {
      modelType,
      modelCode: null,
      displayName: null,
      sourceLabel: null,
    } satisfies WorkspaceDefaultModelRecord;
  });

  sortedRecords.forEach((record) => {
    record.view.isDefault = defaults.some((item) => item.modelCode === record.view.modelCode);
  });

  return {
    catalog: {
      workspaceCode,
      defaults,
      records: sortedRecords.map((record) => record.view),
    } satisfies WorkspaceModelCatalog,
    resolvedRecords: sortedRecords,
  };
}

function resolveWorkspaceCatalogInternal(workspaceCode: string): WorkspaceModelCatalog {
  return resolveWorkspaceCatalogState(workspaceCode).catalog;
}

function resolveWorkspaceRecordState(workspaceCode: string, modelCode: string) {
  const state = resolveWorkspaceCatalogState(workspaceCode);
  const record = state.resolvedRecords.find((item) => item.view.modelCode === modelCode);
  if (!record) {
    throw new Error("model was not found");
  }
  return {
    catalog: state.catalog,
    record,
  };
}

function ensurePlatformDisplayNameUnique(displayName: string, excludeModelCode?: string) {
  const normalized = displayName.trim();
  if (platformModels.some((record) => record.displayName === normalized && record.modelCode !== excludeModelCode)) {
    throw new Error("displayName already exists in platform models");
  }
}

function ensureWorkspaceDisplayNameUnique(
  workspaceCode: string,
  displayName: string,
  excludeModelCode?: string,
) {
  const normalized = displayName.trim();
  const records = resolveWorkspaceCatalogInternal(workspaceCode).records;
  if (records.some((record) => record.displayName === normalized && record.modelCode !== excludeModelCode)) {
    throw new Error(`displayName already exists in workspace ${workspaceCode}`);
  }
}

function nextPlatformCopyDisplayName(displayName: string) {
  const baseName = `${displayName.trim()}-Copy`;
  if (!platformModels.some((record) => record.displayName === baseName)) {
    return baseName;
  }

  let copyIndex = 2;
  while (platformModels.some((record) => record.displayName === `${displayName.trim()}-Copy-${copyIndex}`)) {
    copyIndex += 1;
  }
  return `${displayName.trim()}-Copy-${copyIndex}`;
}

function nextWorkspaceCopyDisplayName(workspaceCode: string, displayName: string) {
  const baseName = `${displayName.trim()}-Copy`;
  const visibleNames = resolveWorkspaceCatalogInternal(workspaceCode).records.map((record) => record.displayName);
  if (!visibleNames.includes(baseName)) {
    return baseName;
  }

  let copyIndex = 2;
  while (visibleNames.includes(`${displayName.trim()}-Copy-${copyIndex}`)) {
    copyIndex += 1;
  }
  return `${displayName.trim()}-Copy-${copyIndex}`;
}

function updateBaseModelFromPayload(
  model: BaseModel,
  payload: ModelMutationPayload,
  options: { allowInheritance?: boolean; fallbackApiKeyRef?: string },
) {
  const modelCapabilities = normalizeModelCapabilities(payload.modelCapabilities, payload.modelType);
  model.displayName = payload.displayName.trim();
  model.vendor = payload.vendor.trim();
  model.protocol = payload.protocol;
  model.modelCapabilities = modelCapabilities;
  model.modelType = getPrimaryModelTypeFromCapabilities(
    payload.protocol,
    modelCapabilities,
    payload.modelType,
  );
  model.baseUrl = payload.baseUrl.trim();
  model.apiPath = payload.apiPath?.trim() ? payload.apiPath.trim() : null;
  if (payload.apiKeyRef.trim() && payload.apiKeyRef.trim() !== MASKED_API_KEY_VALUE) {
    model.apiKeyRef = payload.apiKeyRef.trim();
  } else if (options.fallbackApiKeyRef) {
    model.apiKeyRef = options.fallbackApiKeyRef;
  }
  model.modelName = payload.modelName.trim();
  model.enabled = payload.enabled;
  model.description = payload.description?.trim() ? payload.description.trim() : null;
  model.thinkingEnabled = payload.thinkingEnabled;
  model.supportedReasoningEfforts = payload.thinkingEnabled && model.modelType === "TEXT"
    ? normalizeReasoningEfforts(payload.supportedReasoningEfforts)
    : [];
  model.defaultReasoningEffort = payload.defaultReasoningEffort
    && model.supportedReasoningEfforts.includes(payload.defaultReasoningEffort)
    ? payload.defaultReasoningEffort
    : null;
  model.structuredOutputEnabled = payload.structuredOutputEnabled;
  model.responsesStatefulEnabled = payload.protocol === "RESPONSES" ? payload.responsesStatefulEnabled : false;
  model.temperature = payload.temperature ?? null;
  model.maxTokens = payload.maxTokens ?? null;
  model.contextWindowTokens = payload.contextWindowTokens ?? null;
  model.topP = payload.topP ?? null;
  model.maxRetries = payload.maxRetries ?? null;
  model.updatedAt = now;
  if ("allowInheritance" in model) {
    (model as WorkspaceLocalModel).allowInheritance = options.allowInheritance ?? true;
  }
}

function clearWorkspaceDefaultForModelCode(modelCode: string) {
  Object.values(workspaceDefaults).forEach((mapping) => {
    modelTypeSequence.forEach((modelType) => {
      if (mapping[modelType] === modelCode) {
        delete mapping[modelType];
      }
    });
  });
}

function clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode: string, modelCode: string) {
  const mapping = workspaceDefaults[workspaceCode];
  if (!mapping) return;
  modelTypeSequence.forEach((modelType) => {
    if (mapping[modelType] === modelCode) {
      delete mapping[modelType];
    }
  });
}

function clearPlatformDefaultForModelCode(modelCode: string) {
  modelTypeSequence.forEach((modelType) => {
    if (platformDefaults[modelType] === modelCode) {
      delete platformDefaults[modelType];
    }
  });
}

function removeWorkspaceSlotDependents(slotModelCode: string) {
  workspaceOverrides = workspaceOverrides.filter((record) => record.slotModelCode !== slotModelCode);
  workspaceStatusPatches = workspaceStatusPatches.filter((record) => record.slotModelCode !== slotModelCode);
}

function baseModelForDefinition(definition: DefinitionPointer): BaseModel {
  switch (definition.kind) {
    case "platform": {
      const model = platformModels.find((item) => item.modelCode === definition.modelCode);
      if (!model) throw new Error("model was not found");
      return model;
    }
    case "override": {
      const model = workspaceOverrides.find((item) => item.modelCode === definition.modelCode);
      if (!model) throw new Error("model was not found");
      return model;
    }
    case "local": {
      const model = workspaceLocalModels.find((item) => item.modelCode === definition.modelCode);
      if (!model) throw new Error("model was not found");
      return model;
    }
  }
}

function deleteOwnedPointer(pointer: OwnedPointer) {
  if (!pointer) return;
  switch (pointer.kind) {
    case "override":
      workspaceOverrides = workspaceOverrides.filter((item) => item.modelCode !== pointer.modelCode);
      break;
    case "statusPatch":
      workspaceStatusPatches = workspaceStatusPatches.filter((item) => item.modelCode !== pointer.modelCode);
      break;
    case "local":
      workspaceLocalModels = workspaceLocalModels.filter((item) => item.modelCode !== pointer.modelCode);
      break;
  }
}

function upsertWorkspaceStatusPatch(
  workspaceCode: string,
  slotModelCode: string,
  enabled: boolean,
) {
  const existing = findWorkspaceStatusPatch(workspaceCode, slotModelCode);
  if (existing) {
    existing.enabled = enabled;
    existing.updatedAt = now;
    return existing;
  }

  const created: WorkspaceStatusPatch = {
    createdAt: now,
    enabled,
    modelCode: generateModelCode(),
    slotModelCode,
    updatedAt: now,
    workspaceCode,
  };
  workspaceStatusPatches.push(created);
  return created;
}

function createWorkspaceOverrideFromResolved(
  workspaceCode: string,
  slotModelCode: string,
  payload: ModelMutationPayload,
  fallbackApiKeyRef: string,
) {
  const created: WorkspaceOverrideModel = {
    ...createSeedBaseModel(
      generateModelCode(),
      payload.displayName.trim(),
      payload.vendor.trim(),
      payload.protocol,
      payload.modelType,
      payload.baseUrl.trim(),
      payload.apiPath?.trim() ? payload.apiPath.trim() : null,
      fallbackApiKeyRef,
      payload.modelName.trim(),
    ),
    slotModelCode,
    workspaceCode,
  };
  updateBaseModelFromPayload(created, payload, { fallbackApiKeyRef });
  workspaceOverrides.push(created);
  return created;
}

function verifyBaseModel(model: BaseModel) {
  model.lastVerifiedAt = now;
  model.lastVerifyStatus = "SUCCEEDED";
  model.lastVerifyMessage = `Verified via POST ${model.apiPath ?? defaultVerifyPath(model.protocol, model.modelType)}`;
  model.updatedAt = now;
}

export function listPlatformModelsMock() {
  return sortModelRecords(platformModels.map((record) => buildPlatformView(record)));
}

export function getPlatformModelRecord(modelCode: string) {
  const record = platformModels.find((item) => item.modelCode === modelCode);
  if (!record) throw new Error("model was not found");
  return buildPlatformView(record);
}

export function resetModelMockState() {
  nextMockModelSequence = 1000;
  platformModels = buildInitialPlatformModels();
  workspaceLocalModels = buildInitialWorkspaceLocalModels();
  workspaceOverrides = [];
  workspaceStatusPatches = [];
  platformDefaults = buildInitialPlatformDefaults();
  workspaceDefaults = buildInitialWorkspaceDefaults();
}

export function createPlatformModelRecord(payload: ModelMutationPayload) {
  ensurePlatformDisplayNameUnique(payload.displayName);
  const defaultsBeforeCreate = resolveEffectivePlatformDefaults();
  const record = createSeedBaseModel(
    generateModelCode(),
    payload.displayName.trim(),
    payload.vendor.trim(),
    payload.protocol,
    payload.modelType,
    payload.baseUrl.trim(),
    payload.apiPath?.trim() ? payload.apiPath.trim() : null,
    payload.apiKeyRef.trim(),
    payload.modelName.trim(),
  );
  updateBaseModelFromPayload(record, payload, {});
  platformModels.push(record);
  if (record.enabled && !defaultsBeforeCreate[record.modelType]) {
    platformDefaults = {
      ...platformDefaults,
      [record.modelType]: record.modelCode,
    };
  }
  return buildPlatformView(record);
}

export function verifyPlatformModelCreateDraftRecord(payload: ModelMutationPayload) {
  ensurePlatformDisplayNameUnique(payload.displayName);
  const preview = createSeedBaseModel(
    "",
    payload.displayName.trim(),
    payload.vendor.trim(),
    payload.protocol,
    payload.modelType,
    payload.baseUrl.trim(),
    payload.apiPath?.trim() ? payload.apiPath.trim() : null,
    payload.apiKeyRef.trim(),
    payload.modelName.trim(),
  );
  updateBaseModelFromPayload(preview, payload, {});
  verifyBaseModel(preview);
  return buildPlatformView(preview);
}

export function updatePlatformModelRecord(modelCode: string, payload: ModelMutationPayload) {
  const record = platformModels.find((item) => item.modelCode === modelCode);
  if (!record) throw new Error("model was not found");
  ensurePlatformDisplayNameUnique(payload.displayName, modelCode);
  const previousModelType = record.modelType;
  const wasEnabled = record.enabled;
  updateBaseModelFromPayload(record, payload, {});
  if (previousModelType !== record.modelType || (wasEnabled && !record.enabled)) {
    clearWorkspaceDefaultForModelCode(modelCode);
    clearPlatformDefaultForModelCode(modelCode);
  }
  return buildPlatformView(record);
}

export function verifyPlatformModelRecord(modelCode: string) {
  const record = platformModels.find((item) => item.modelCode === modelCode);
  if (!record) throw new Error("model was not found");
  verifyBaseModel(record);
  return buildPlatformView(record);
}

export function verifyPlatformModelDraftRecord(modelCode: string, payload: ModelMutationPayload) {
  const record = platformModels.find((item) => item.modelCode === modelCode);
  if (!record) throw new Error("model was not found");
  const preview = cloneBaseModel(record);
  updateBaseModelFromPayload(preview, payload, {});
  verifyBaseModel(preview);
  return buildPlatformView(preview);
}

export function duplicatePlatformModelRecord(modelCode: string) {
  const source = platformModels.find((item) => item.modelCode === modelCode);
  if (!source) throw new Error("model was not found");
  const duplicate = cloneBaseModel(source);
  duplicate.modelCode = generateModelCode();
  duplicate.displayName = nextPlatformCopyDisplayName(source.displayName);
  duplicate.lastVerifiedAt = null;
  duplicate.lastVerifyStatus = null;
  duplicate.lastVerifyMessage = null;
  duplicate.createdAt = now;
  duplicate.updatedAt = now;
  platformModels.push(duplicate);
  return buildPlatformView(duplicate);
}

export function setPlatformModelEnabled(modelCode: string, enabled: boolean) {
  const record = platformModels.find((item) => item.modelCode === modelCode);
  if (!record) throw new Error("model was not found");
  record.enabled = enabled;
  record.updatedAt = now;
  if (!enabled) {
    clearWorkspaceDefaultForModelCode(modelCode);
    clearPlatformDefaultForModelCode(modelCode);
  }
  return buildPlatformView(record);
}

export function setPlatformDefaultRecord(modelCode: string): PlatformDefaultMutationResult {
  const record = platformModels.find((item) => item.modelCode === modelCode);
  if (!record) throw new Error("model was not found");
  if (!record.enabled) throw new Error("default model must be enabled before it can be assigned");
  platformDefaults = {
    ...platformDefaults,
    [record.modelType]: record.modelCode,
  };
  return {
    modelCode: record.modelCode,
    modelType: record.modelType,
  };
}

export function deletePlatformModelRecord(modelCode: string) {
  const next = platformModels.filter((record) => record.modelCode !== modelCode);
  if (next.length === platformModels.length) throw new Error("model was not found");
  platformModels = next;
  removeWorkspaceSlotDependents(modelCode);
  clearWorkspaceDefaultForModelCode(modelCode);
  clearPlatformDefaultForModelCode(modelCode);
  return { modelCode, workspaceCode: null };
}

export function getWorkspaceModelCatalog(workspaceCode: string) {
  return resolveWorkspaceCatalogInternal(workspaceCode);
}

export function getWorkspaceSelectOptions(workspaceCode: string): ModelSelectOption[] {
  return resolveWorkspaceCatalogInternal(workspaceCode).records.map((record) => ({
    modelCode: record.modelCode,
    displayName: record.displayName,
    modelName: record.modelName,
    sourceKind: record.sourceKind,
    sourceLabel: record.sourceLabel,
    vendor: record.vendor,
    protocol: record.protocol,
    modelType: record.modelType,
    enabled: record.enabled,
    contextWindowTokens: record.contextWindowTokens,
    supportedReasoningEfforts: [...record.supportedReasoningEfforts],
    defaultReasoningEffort: record.defaultReasoningEffort,
  }));
}

export function createWorkspaceModelRecord(
  workspaceCode: string,
  payload: ModelMutationPayload,
) {
  currentWorkspaceEntry(workspaceCode);
  ensureWorkspaceDisplayNameUnique(workspaceCode, payload.displayName);
  const record: WorkspaceLocalModel = {
    ...createSeedBaseModel(
      generateModelCode(),
      payload.displayName.trim(),
      payload.vendor.trim(),
      payload.protocol,
      payload.modelType,
      payload.baseUrl.trim(),
      payload.apiPath?.trim() ? payload.apiPath.trim() : null,
      payload.apiKeyRef.trim(),
      payload.modelName.trim(),
    ),
    allowInheritance: payload.allowInheritance ?? true,
    workspaceCode,
  };
  updateBaseModelFromPayload(record, payload, {
    allowInheritance: payload.allowInheritance ?? true,
  });
  workspaceLocalModels.push(record);
  return resolveWorkspaceRecordState(workspaceCode, record.modelCode).record.view;
}

export function verifyWorkspaceModelCreateDraftRecord(
  workspaceCode: string,
  payload: ModelMutationPayload,
) {
  currentWorkspaceEntry(workspaceCode);
  ensureWorkspaceDisplayNameUnique(workspaceCode, payload.displayName);
  const preview: WorkspaceLocalModel = {
    ...createSeedBaseModel(
      "",
      payload.displayName.trim(),
      payload.vendor.trim(),
      payload.protocol,
      payload.modelType,
      payload.baseUrl.trim(),
      payload.apiPath?.trim() ? payload.apiPath.trim() : null,
      payload.apiKeyRef.trim(),
      payload.modelName.trim(),
    ),
    allowInheritance: payload.allowInheritance ?? true,
    workspaceCode,
  };
  updateBaseModelFromPayload(preview, payload, {
    allowInheritance: payload.allowInheritance ?? true,
  });
  verifyBaseModel(preview);
  return buildWorkspaceView(
    workspaceCode,
    preview.modelCode,
    preview,
    "LOCAL",
    null,
    preview.enabled,
    preview.allowInheritance,
    { createdAt: preview.createdAt, updatedAt: preview.updatedAt },
  );
}

export function duplicateWorkspaceModelRecord(workspaceCode: string, modelCode: string) {
  currentWorkspaceEntry(workspaceCode);
  const { record } = resolveWorkspaceRecordState(workspaceCode, modelCode);
  const source = baseModelForDefinition(record.definition);
  const duplicate: WorkspaceLocalModel = {
    ...cloneBaseModel(source),
    allowInheritance: true,
    modelCode: generateModelCode(),
    displayName: nextWorkspaceCopyDisplayName(workspaceCode, record.view.displayName),
    lastVerifiedAt: null,
    lastVerifyStatus: null,
    lastVerifyMessage: null,
    workspaceCode,
    createdAt: now,
    updatedAt: now,
  };
  workspaceLocalModels.push(duplicate);
  return resolveWorkspaceRecordState(workspaceCode, duplicate.modelCode).record.view;
}

export function updateWorkspaceModelRecord(
  workspaceCode: string,
  modelCode: string,
  payload: ModelMutationPayload,
) {
  const { record } = resolveWorkspaceRecordState(workspaceCode, modelCode);
  ensureWorkspaceDisplayNameUnique(workspaceCode, payload.displayName, modelCode);

  if (record.view.sourceKind === "LOCAL") {
    const local = workspaceLocalModels.find((item) => item.modelCode === modelCode && item.workspaceCode === workspaceCode);
    if (!local) throw new Error("model was not found");
    const previousModelType = local.modelType;
    const wasEnabled = local.enabled;
    updateBaseModelFromPayload(local, payload, {
      allowInheritance: local.allowInheritance,
    });
    if (previousModelType !== local.modelType || (wasEnabled && !local.enabled)) {
      clearWorkspaceDefaultForModelCode(local.modelCode);
    }
    return resolveWorkspaceRecordState(workspaceCode, local.modelCode).record.view;
  }

  const fallbackApiKeyRef = baseModelForDefinition(record.definition).apiKeyRef;
  if (record.currentOwned?.kind === "statusPatch") {
    deleteOwnedPointer(record.currentOwned);
  }

  const existingOverride = record.currentOwned?.kind === "override"
    ? workspaceOverrides.find((item) => item.modelCode === record.currentOwned?.modelCode)
    : findWorkspaceOverride(workspaceCode, modelCode);
  if (existingOverride) {
    const previousModelType = existingOverride.modelType;
    const wasEnabled = existingOverride.enabled;
    updateBaseModelFromPayload(existingOverride, payload, { fallbackApiKeyRef });
    existingOverride.slotModelCode = modelCode;
    if (previousModelType !== existingOverride.modelType || (wasEnabled && !existingOverride.enabled)) {
      clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode, modelCode);
    }
  } else {
    const created = createWorkspaceOverrideFromResolved(workspaceCode, modelCode, payload, fallbackApiKeyRef);
    if (!created.enabled) {
      clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode, modelCode);
    }
  }

  return resolveWorkspaceRecordState(workspaceCode, modelCode).record.view;
}

export function verifyWorkspaceModelRecord(workspaceCode: string, modelCode: string) {
  const { record } = resolveWorkspaceRecordState(workspaceCode, modelCode);
  verifyBaseModel(baseModelForDefinition(record.definition));
  return resolveWorkspaceRecordState(workspaceCode, modelCode).record.view;
}

export function verifyWorkspaceModelDraftRecord(
  workspaceCode: string,
  modelCode: string,
  payload: ModelMutationPayload,
) {
  const { record } = resolveWorkspaceRecordState(workspaceCode, modelCode);
  const preview = cloneBaseModel(baseModelForDefinition(record.definition));
  const fallbackApiKeyRef = baseModelForDefinition(record.definition).apiKeyRef;
  updateBaseModelFromPayload(preview, payload, { fallbackApiKeyRef });
  verifyBaseModel(preview);
  return {
    ...record.view,
    apiKeyRef: MASKED_API_KEY_VALUE,
    apiPath: preview.apiPath,
    baseUrl: preview.baseUrl,
    description: preview.description,
    displayName: preview.displayName,
    lastVerifiedAt: preview.lastVerifiedAt,
    lastVerifyMessage: preview.lastVerifyMessage,
    lastVerifyStatus: preview.lastVerifyStatus,
    maxRetries: preview.maxRetries,
    maxTokens: preview.maxTokens,
    contextWindowTokens: preview.contextWindowTokens,
    modelCapabilities: [...preview.modelCapabilities],
    modelName: preview.modelName,
    modelType: preview.modelType,
    protocol: preview.protocol,
    supportedReasoningEfforts: [...preview.supportedReasoningEfforts],
    defaultReasoningEffort: preview.defaultReasoningEffort,
    structuredOutputEnabled: preview.structuredOutputEnabled,
    responsesStatefulEnabled: preview.responsesStatefulEnabled,
    temperature: preview.temperature,
    thinkingEnabled: preview.thinkingEnabled,
    topP: preview.topP,
    updatedAt: preview.updatedAt,
    vendor: preview.vendor,
  };
}

export function setWorkspaceModelEnabled(
  workspaceCode: string,
  modelCode: string,
  enabled: boolean,
) {
  const { record } = resolveWorkspaceRecordState(workspaceCode, modelCode);

  switch (record.view.sourceKind) {
    case "LOCAL": {
      const local = workspaceLocalModels.find((item) => item.modelCode === modelCode && item.workspaceCode === workspaceCode);
      if (!local) throw new Error("model was not found");
      local.enabled = enabled;
      local.updatedAt = now;
      if (!enabled) {
        clearWorkspaceDefaultForModelCode(modelCode);
      }
      break;
    }
    case "OVERRIDE": {
      const override = workspaceOverrides.find((item) => item.modelCode === record.currentOwned?.modelCode);
      if (!override) throw new Error("model was not found");
      override.enabled = enabled;
      override.updatedAt = now;
      if (!enabled) {
        clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode, modelCode);
      }
      break;
    }
    case "INHERITED": {
      const upstreamEnabled = record.upstreamEnabled ?? record.view.enabled;
      const existingPatch = record.currentOwned?.kind === "statusPatch"
        ? workspaceStatusPatches.find((item) => item.modelCode === record.currentOwned?.modelCode)
        : findWorkspaceStatusPatch(workspaceCode, modelCode);
      if (enabled === upstreamEnabled) {
        if (existingPatch) {
          workspaceStatusPatches = workspaceStatusPatches.filter((item) => item.modelCode !== existingPatch.modelCode);
        }
      } else if (existingPatch) {
        existingPatch.enabled = enabled;
        existingPatch.updatedAt = now;
      } else {
        upsertWorkspaceStatusPatch(workspaceCode, modelCode, enabled);
      }
      if (!enabled) {
        clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode, modelCode);
      }
      break;
    }
    default:
      break;
  }

  return resolveWorkspaceRecordState(workspaceCode, modelCode).record.view;
}

export function setWorkspaceDefaultRecord(
  workspaceCode: string,
  modelCode: string,
): WorkspaceDefaultMutationResult {
  const { record } = resolveWorkspaceRecordState(workspaceCode, modelCode);
  if (!record.view.enabled) {
    throw new Error("default model must be enabled in the current workspace");
  }
  workspaceDefaults[workspaceCode] = {
    ...(workspaceDefaults[workspaceCode] ?? {}),
    [record.view.modelType]: record.view.modelCode,
  };
  return {
    workspaceCode,
    modelCode: record.view.modelCode,
    modelType: record.view.modelType,
  };
}

export function setWorkspaceAllowInheritanceRecord(
  workspaceCode: string,
  modelCode: string,
  allowInheritance: boolean,
) {
  const local = workspaceLocalModels.find((item) => item.workspaceCode === workspaceCode && item.modelCode === modelCode);
  if (!local) throw new Error("model was not found");
  local.allowInheritance = allowInheritance;
  local.updatedAt = now;
  return resolveWorkspaceRecordState(workspaceCode, modelCode).record.view;
}

export function deleteWorkspaceModelRecord(workspaceCode: string, modelCode: string) {
  const { record } = resolveWorkspaceRecordState(workspaceCode, modelCode);
  if (record.view.sourceKind === "INHERITED") {
    throw new Error("inherited models must be overridden or disabled instead of deleted");
  }

  if (record.view.sourceKind === "LOCAL") {
    workspaceLocalModels = workspaceLocalModels.filter((item) => !(item.workspaceCode === workspaceCode && item.modelCode === modelCode));
    removeWorkspaceSlotDependents(modelCode);
    clearWorkspaceDefaultForModelCode(modelCode);
    return {
      workspaceCode,
      modelCode,
    };
  }

  deleteOwnedPointer(record.currentOwned);
  return {
    workspaceCode,
    modelCode,
  };
}

function defaultVerifyPath(protocol: ModelProtocol, modelType: ModelType) {
  if (protocol === "OPENAI_COMPATIBLE" && modelType === "TEXT") return "/chat/completions";
  if (protocol === "OPENAI_COMPATIBLE_EMBEDDING") return "/embeddings";
  if (protocol === "RERANK" && modelType === "RERANK") return "/rerank";
  if (protocol === "ANTHROPIC") return "/messages";
  if (protocol === "GEMINI") return "";
  return "/responses";
}

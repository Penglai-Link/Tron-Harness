import { expect, test, type Page, type Route } from "@playwright/test";
import {
  spawn,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
const now = "2026-05-20T12:00:00Z";
const MASKED_API_KEY_VALUE = "*****";

let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

type ModelScopeKind = "PLATFORM" | "WORKSPACE";
type ModelSourceKind = "PLATFORM" | "LOCAL" | "INHERITED" | "OVERRIDE";
type ModelProtocol = "OPENAI_COMPATIBLE" | "RESPONSES" | "RERANK";
type ModelType = "TEXT" | "EMBEDDING" | "RERANK" | "IMAGE";
type ModelCapability =
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
  lastVerifyStatus: "SUCCEEDED" | "FAILED" | null;
  maxRetries: number | null;
  maxTokens: number | null;
  modelCapabilities: ModelCapability[];
  modelCode: string;
  modelName: string;
  modelType: ModelType;
  protocol: ModelProtocol;
  structuredOutputEnabled: boolean;
  temperature: number | null;
  thinkingEnabled: boolean;
  timeoutMs: number | null;
  topP: number | null;
  updatedAt: string;
  vendor: string;
};

type PlatformModel = BaseModel;
type WorkspaceLocalModel = BaseModel & { allowInheritance: boolean; workspaceCode: string };
type WorkspaceOverrideModel = BaseModel & { slotModelCode: string; workspaceCode: string };
type WorkspaceStatusPatch = {
  createdAt: string;
  enabled: boolean;
  modelCode: string;
  slotModelCode: string;
  updatedAt: string;
  workspaceCode: string;
};

type WorkspaceLineageEntry = {
  label: string;
  name: string;
  workspaceCode: string;
};

type ModelRecord = {
  allowInheritance: boolean | null;
  apiKeyRef: string;
  apiPath: string | null;
  baseUrl: string;
  createdAt: string;
  description: string | null;
  displayName: string;
  enabled: boolean;
  isDefault: boolean;
  lastVerifiedAt: string | null;
  lastVerifyMessage: string | null;
  lastVerifyStatus: "SUCCEEDED" | "FAILED" | null;
  maxRetries: number | null;
  maxTokens: number | null;
  modelCapabilities: ModelCapability[];
  modelCode: string;
  modelName: string;
  modelType: ModelType;
  protocol: ModelProtocol;
  scopeKind: ModelScopeKind;
  sourceKind: ModelSourceKind;
  sourceLabel: string;
  sourceModelCode: string | null;
  sourceWorkspaceCode: string | null;
  sourceWorkspaceName: string | null;
  sourceWorkspacePath: WorkspaceLineageEntry[];
  structuredOutputEnabled: boolean;
  temperature: number | null;
  thinkingEnabled: boolean;
  timeoutMs: number | null;
  topP: number | null;
  updatedAt: string;
  vendor: string;
  workspaceCode: string | null;
};

type WorkspaceDefaultRecord = {
  displayName: string | null;
  modelCode: string | null;
  modelType: ModelType;
  sourceLabel: string | null;
};

type WorkspaceCatalog = {
  defaults: WorkspaceDefaultRecord[];
  records: ModelRecord[];
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

const authenticatedSession = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    roleCodes: ["ADMIN", "WORKSPACE_ADMIN"],
    spaceCodes: ["TRON_CORE", "PLATFORM", "COMPLIANCE"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  },
};

const authenticatedGrant = {
  accessToken: "playwright-access-token",
  tokenType: "Bearer",
  expiresIn: 3600,
  session: authenticatedSession,
};

const workspaceSwitcher = {
  primarySpaces: [
    {
      workspaceCode: "ENTERPRISE",
      name: "Enterprise",
      description: "Global governance workspace",
      childCount: 1,
      status: "Active",
    },
    {
      workspaceCode: "COMPLIANCE",
      name: "Compliance",
      description: "Compliance evidence workspace",
      childCount: 0,
      status: "Active",
    },
  ],
  secondarySpaces: [
    {
      workspaceCode: "ENTERPRISE",
      primaryWorkspaceCode: "ENTERPRISE",
      label: "Enterprise",
      name: "Enterprise",
      description: "Global governance workspace",
      depth: 1,
      childCount: 1,
      effectiveUserCount: 1,
      status: "Active",
    },
    {
      workspaceCode: "PLATFORM",
      primaryWorkspaceCode: "ENTERPRISE",
      label: "Enterprise / Platform",
      name: "Platform",
      description: "Platform runtime workspace",
      depth: 2,
      childCount: 1,
      effectiveUserCount: 1,
      status: "Active",
    },
    {
      workspaceCode: "TRON_CORE",
      primaryWorkspaceCode: "ENTERPRISE",
      label: "Enterprise / Platform / Tron Core",
      name: "Tron Core",
      description: "Main Tron console workspace",
      depth: 3,
      childCount: 0,
      effectiveUserCount: 1,
      status: "Active",
    },
    {
      workspaceCode: "COMPLIANCE",
      primaryWorkspaceCode: "COMPLIANCE",
      label: "Compliance",
      name: "Compliance",
      description: "Compliance evidence workspace",
      depth: 1,
      childCount: 0,
      effectiveUserCount: 1,
      status: "Active",
    },
  ],
} as const;

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

const modelTypeSequence: ModelType[] = ["TEXT", "EMBEDDING", "RERANK", "IMAGE"];

function defaultModelCapabilitiesForType(modelType: ModelType): ModelCapability[] {
  switch (modelType) {
    case "EMBEDDING":
      return ["TEXT_VECTOR"];
    case "RERANK":
      return ["RERANK"];
    case "IMAGE":
      return ["IMAGE_GENERATION"];
    default:
      return ["TEXT_GENERATION"];
  }
}

function getModelTypeFromCapability(value: ModelCapability): ModelType {
  switch (value) {
    case "VISION_UNDERSTANDING":
    case "IMAGE_GENERATION":
    case "VIDEO_GENERATION":
    case "THREED_GENERATION":
      return "IMAGE";
    case "MULTIMODAL_VECTOR":
    case "TEXT_VECTOR":
      return "EMBEDDING";
    case "RERANK":
      return "RERANK";
    default:
      return "TEXT";
  }
}

function normalizeModelCapabilities(values: ModelCapability[], fallbackModelType: ModelType): ModelCapability[] {
  const seeded = values.length ? values : defaultModelCapabilitiesForType(fallbackModelType);
  const normalized: ModelCapability[] = [];
  seeded.forEach((value) => {
    if (!normalized.includes(value)) normalized.push(value);
  });
  return normalized.length ? normalized : defaultModelCapabilitiesForType(fallbackModelType);
}

function getPrimaryModelTypeFromCapabilities(
  protocol: ModelProtocol,
  capabilities: ModelCapability[],
  fallbackModelType: ModelType,
): ModelType {
  const normalized = normalizeModelCapabilities(capabilities, fallbackModelType);
  const selectedTypes = new Set(normalized.map((value) => getModelTypeFromCapability(value)));
  if (selectedTypes.has(fallbackModelType)) return fallbackModelType;
  const validTypes: ModelType[] = protocol === "OPENAI_COMPATIBLE"
    ? ["TEXT", "EMBEDDING"]
    : protocol === "RERANK"
      ? ["RERANK"]
      : ["TEXT", "IMAGE"];
  return validTypes.find((value) => selectedTypes.has(value)) ?? fallbackModelType;
}

function successEnvelope(data: unknown) {
  return { code: "SUCCESS", message: "ok", data };
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

function errorEnvelope(message: string, code = "MODEL_ERROR") {
  return { code, message, data: {} };
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();
    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        rejectPort(new Error("Failed to reserve a frontend test port."));
        return;
      }
      server.close(() => resolvePort(address.port));
    });
  });
}

async function waitForFrontend(url: string) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (frontendProcess?.exitCode !== null) {
      throw new Error(`Frontend dev server exited early.\n${frontendLogs}`);
    }
    try {
      if ((await fetch(url)).ok) return;
    } catch {
      // frontend still booting
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
  }
  throw new Error(`Frontend dev server did not become ready.\n${frontendLogs}`);
}

function toast(page: Page, message: string | RegExp) {
  return page.locator('[aria-label="Notifications"]').getByRole("status").filter({ hasText: message }).first();
}

function currentWorkspaceEntry(workspaceCode: string) {
  const lineage = workspaceLineages[workspaceCode];
  if (!lineage?.length) throw new Error(`Workspace ${workspaceCode} was not found.`);
  return lineage[lineage.length - 1];
}

function workspaceName(workspaceCode: string) {
  return currentWorkspaceEntry(workspaceCode).name;
}

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
    maxRetries: 1,
    maxTokens: modelType === "TEXT" ? 4096 : null,
    modelCapabilities: defaultModelCapabilitiesForType(modelType),
    modelCode,
    modelName,
    modelType,
    protocol,
    structuredOutputEnabled: protocol === "RESPONSES",
    temperature: modelType === "TEXT" ? 0.2 : null,
    thinkingEnabled: modelType === "TEXT",
    timeoutMs: 5000,
    topP: modelType === "TEXT" ? 1 : null,
    updatedAt: now,
    vendor,
  };
}

function maskApiKeyValue(value: string) {
  return value.trim() ? MASKED_API_KEY_VALUE : "";
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

function cloneBaseModel<T extends BaseModel>(model: T): T {
  return {
    ...model,
    modelCapabilities: [...model.modelCapabilities],
  };
}

function buildPlatformView(model: PlatformModel, platformDefaults: Partial<Record<ModelType, string>>): ModelRecord {
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
    structuredOutputEnabled: model.structuredOutputEnabled,
    temperature: model.temperature,
    maxTokens: model.maxTokens,
    topP: model.topP,
    timeoutMs: model.timeoutMs,
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
    isDefault: platformDefaults[model.modelType] === model.modelCode && model.enabled,
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
    structuredOutputEnabled: configModel.structuredOutputEnabled,
    temperature: configModel.temperature,
    maxTokens: configModel.maxTokens,
    topP: configModel.topP,
    timeoutMs: configModel.timeoutMs,
    maxRetries: configModel.maxRetries,
    lastVerifiedAt: configModel.lastVerifiedAt,
    lastVerifyStatus: configModel.lastVerifyStatus,
    lastVerifyMessage: configModel.lastVerifyMessage,
    sourceKind,
    sourceModelCode: sourceKind === "LOCAL" ? null : slotModelCode,
    sourceLabel: buildSourceLabel(sourceKind),
    sourceWorkspaceCode: inheritedFrom?.workspaceCode ?? null,
    sourceWorkspaceName: inheritedFrom?.workspaceName ?? null,
    sourceWorkspacePath: inheritedFrom?.workspacePath.map((entry) => ({ ...entry })) ?? [],
    allowInheritance,
    isDefault: false,
    createdAt: timestamps.createdAt,
    updatedAt: timestamps.updatedAt,
  };
}

function createMockState() {
  let nextModelSequence = 1000;
  let platformModels: PlatformModel[] = [
    createSeedBaseModel("MODEL-734600000000000001", "GPT-4o Main", "OpenAI", "RESPONSES", "TEXT", "https://api.openai.com/v1", "/responses", "sk-platform-openai-gpt4o-demo", "gpt-4o"),
    createSeedBaseModel("MODEL-734600000000000002", "Embedding Shared", "OpenAI", "OPENAI_COMPATIBLE", "EMBEDDING", "https://api.openai.com/v1", "/embeddings", "sk-platform-openai-embedding-demo", "text-embedding-3-small"),
    createSeedBaseModel("MODEL-734600000000000003", "Rerank Default", "Cohere", "RERANK", "RERANK", "https://api.cohere.ai/v1", "/rerank", "sk-platform-cohere-rerank-demo", "rerank-v3.5"),
  ];
  let workspaceLocalModels: WorkspaceLocalModel[] = [
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
  let workspaceOverrides: WorkspaceOverrideModel[] = [];
  let workspaceStatusPatches: WorkspaceStatusPatch[] = [];
  let platformDefaults: Partial<Record<ModelType, string>> = {
    TEXT: "MODEL-734600000000000001",
    EMBEDDING: "MODEL-734600000000000002",
    RERANK: "MODEL-734600000000000003",
  };
  let workspaceDefaults: Record<string, Partial<Record<ModelType, string>>> = {
    COMPLIANCE: {},
    ENTERPRISE: {},
    PLATFORM: {},
    TRON_CORE: {},
  };

  function generateModelCode() {
    nextModelSequence += 1;
    return `MODEL-MOCK-${nextModelSequence}`;
  }

  function resolveEffectivePlatformDefaults() {
    return modelTypeSequence.reduce<Partial<Record<ModelType, string>>>((defaults, modelType) => {
      const explicit = platformDefaults[modelType];
      const explicitRecord = explicit
        ? platformModels.find((model) => model.modelCode === explicit && model.enabled && model.modelType === modelType)
        : null;
      if (explicitRecord) {
        defaults[modelType] = explicitRecord.modelCode;
        return defaults;
      }

      const fallbackRecord = platformModels.find((model) => model.enabled && model.modelType === modelType);
      if (fallbackRecord) {
        defaults[modelType] = fallbackRecord.modelCode;
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
      workspacePath: lineage.map((entry) => ({ ...entry })),
    };
  }

  function statusPatchProvider(definitionModel: BaseModel, workspaceCode: string, enabled: boolean): EffectiveProvider {
    return {
      enabled,
      model: cloneBaseModel(definitionModel),
      workspaceCode,
      workspaceName: workspaceName(workspaceCode),
      workspacePath: (workspaceLineages[workspaceCode] ?? []).map((entry) => ({ ...entry })),
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

  function baseModelForDefinition(definition: DefinitionPointer): BaseModel {
    if (definition.kind === "platform") {
      const platformModel = platformModels.find((item) => item.modelCode === definition.modelCode);
      if (!platformModel) throw new Error("model was not found");
      return platformModel;
    }
    if (definition.kind === "override") {
      const overrideModel = workspaceOverrides.find((item) => item.modelCode === definition.modelCode);
      if (!overrideModel) throw new Error("model was not found");
      return overrideModel;
    }
    const localModel = workspaceLocalModels.find((item) => item.modelCode === definition.modelCode);
    if (!localModel) throw new Error("model was not found");
    return localModel;
  }

  function findWorkspaceOverride(workspaceCode: string, slotModelCode: string) {
    return workspaceOverrides.find((item) => item.workspaceCode === workspaceCode && item.slotModelCode === slotModelCode);
  }

  function findWorkspaceStatusPatch(workspaceCode: string, slotModelCode: string) {
    return workspaceStatusPatches.find((item) => item.workspaceCode === workspaceCode && item.slotModelCode === slotModelCode);
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
        inheritedProvider = statusPatchProvider(definitionProvider.model, ancestor.workspaceCode, ancestorPatch.enabled);
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
    const roots: Array<{ kind: "platform"; model: PlatformModel } | { kind: "local"; model: WorkspaceLocalModel }> = [
      ...platformModels.map((model) => ({ kind: "platform" as const, model })),
      ...workspaceLocalModels
        .filter((model) => model.workspaceCode === workspaceCode || (lineageCodes.has(model.workspaceCode) && model.allowInheritance))
        .map((model) => ({ kind: "local" as const, model })),
    ];

    const resolvedRecords = roots.map((root) => buildResolvedWorkspaceRecord(workspaceCode, root));
    const sortedViews = sortModelRecords(resolvedRecords.map((record) => record.view));
    const sortedRecords = sortedViews.map((view) => resolvedRecords.find((record) => record.view.modelCode === view.modelCode)!);
    const recordMap = new Map(sortedRecords.map((record) => [record.view.modelCode, record.view]));
    const platformDefaultMap = resolveEffectivePlatformDefaults();
    const resolutionOrder = [
      workspaceCode,
      ...(workspaceLineages[workspaceCode] ?? []).map((entry) => entry.workspaceCode).reverse().filter((code) => code !== workspaceCode),
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
          } satisfies WorkspaceDefaultRecord;
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
        } satisfies WorkspaceDefaultRecord;
      }

      return {
        modelType,
        modelCode: null,
        displayName: null,
        sourceLabel: null,
      } satisfies WorkspaceDefaultRecord;
    });

    sortedRecords.forEach((record) => {
      record.view.isDefault = defaults.some((item) => item.modelCode === record.view.modelCode);
    });

    return {
      catalog: {
        workspaceCode,
        defaults,
        records: sortedRecords.map((record) => record.view),
      } satisfies WorkspaceCatalog,
      resolvedRecords: sortedRecords,
    };
  }

  function resolveWorkspaceRecordState(workspaceCode: string, modelCode: string) {
    const state = resolveWorkspaceCatalogState(workspaceCode);
    const record = state.resolvedRecords.find((item) => item.view.modelCode === modelCode);
    if (!record) throw new Error("model was not found");
    return record;
  }

  function ensureWorkspaceDisplayNameUnique(workspaceCode: string, displayName: string, excludeModelCode?: string) {
    const normalized = displayName.trim();
    const records = resolveWorkspaceCatalogState(workspaceCode).catalog.records;
    if (records.some((record) => record.displayName === normalized && record.modelCode !== excludeModelCode)) {
      throw new Error(`displayName already exists in workspace ${workspaceCode}`);
    }
  }

  function updateBaseModel(
    model: BaseModel,
    payload: Record<string, unknown>,
    options: { allowInheritance?: boolean; fallbackApiKeyRef?: string } = {},
  ) {
    const capabilities = normalizeModelCapabilities((payload.modelCapabilities as ModelCapability[] | undefined) ?? model.modelCapabilities, model.modelType);
    model.displayName = String(payload.displayName ?? model.displayName).trim();
    model.vendor = String(payload.vendor ?? model.vendor).trim();
    model.protocol = (payload.protocol as ModelProtocol | undefined) ?? model.protocol;
    model.modelCapabilities = capabilities;
    model.modelType = getPrimaryModelTypeFromCapabilities(
      model.protocol,
      capabilities,
      (payload.modelType as ModelType | undefined) ?? model.modelType,
    );
    model.baseUrl = String(payload.baseUrl ?? model.baseUrl).trim();
    model.apiPath = typeof payload.apiPath === "string" && payload.apiPath.trim()
      ? payload.apiPath.trim()
      : payload.apiPath === null
        ? null
        : model.apiPath;
    const incomingKey = typeof payload.apiKeyRef === "string" ? payload.apiKeyRef.trim() : "";
    if (incomingKey && incomingKey !== MASKED_API_KEY_VALUE) {
      model.apiKeyRef = incomingKey;
    } else if (options.fallbackApiKeyRef) {
      model.apiKeyRef = options.fallbackApiKeyRef;
    }
    model.modelName = String(payload.modelName ?? model.modelName).trim();
    model.enabled = Boolean(payload.enabled ?? model.enabled);
    model.description = typeof payload.description === "string" && payload.description.trim()
      ? payload.description.trim()
      : payload.description === null
        ? null
        : model.description;
    model.thinkingEnabled = Boolean(payload.thinkingEnabled ?? model.thinkingEnabled);
    model.structuredOutputEnabled = Boolean(payload.structuredOutputEnabled ?? model.structuredOutputEnabled);
    model.temperature = typeof payload.temperature === "number" ? payload.temperature : model.temperature;
    model.maxTokens = typeof payload.maxTokens === "number"
      ? payload.maxTokens
      : payload.maxTokens === null
        ? null
        : model.maxTokens;
    model.topP = typeof payload.topP === "number" ? payload.topP : model.topP;
    model.timeoutMs = typeof payload.timeoutMs === "number" ? payload.timeoutMs : model.timeoutMs;
    model.maxRetries = typeof payload.maxRetries === "number" ? payload.maxRetries : model.maxRetries;
    model.updatedAt = now;
    if ("allowInheritance" in model) {
      (model as WorkspaceLocalModel).allowInheritance = options.allowInheritance ?? (model as WorkspaceLocalModel).allowInheritance;
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

  function verifyBaseModel(model: BaseModel) {
    model.lastVerifiedAt = now;
    model.lastVerifyStatus = "SUCCEEDED";
    model.lastVerifyMessage = `Verified via POST ${model.apiPath ?? "/responses"}`;
    model.updatedAt = now;
  }

  function deleteOwnedPointer(pointer: OwnedPointer) {
    if (!pointer) return;
    if (pointer.kind === "override") {
      workspaceOverrides = workspaceOverrides.filter((item) => item.modelCode !== pointer.modelCode);
      return;
    }
    if (pointer.kind === "statusPatch") {
      workspaceStatusPatches = workspaceStatusPatches.filter((item) => item.modelCode !== pointer.modelCode);
      return;
    }
    workspaceLocalModels = workspaceLocalModels.filter((item) => item.modelCode !== pointer.modelCode);
  }

  function removeWorkspaceSlotDependents(slotModelCode: string) {
    workspaceOverrides = workspaceOverrides.filter((item) => item.slotModelCode !== slotModelCode);
    workspaceStatusPatches = workspaceStatusPatches.filter((item) => item.slotModelCode !== slotModelCode);
  }

  return {
    createPlatformModel(payload: Record<string, unknown>) {
      const record = createSeedBaseModel(
        generateModelCode(),
        String(payload.displayName),
        String(payload.vendor ?? "OpenAI"),
        (payload.protocol as ModelProtocol | undefined) ?? "RESPONSES",
        (payload.modelType as ModelType | undefined) ?? "TEXT",
        String(payload.baseUrl ?? "https://api.openai.com/v1"),
        typeof payload.apiPath === "string" ? payload.apiPath : null,
        String(payload.apiKeyRef ?? "sk-platform-mock"),
        String(payload.modelName),
      );
      updateBaseModel(record, payload);
      platformModels.push(record);
      return record.modelCode;
    },
    createWorkspaceModel(workspaceCode: string, payload: Record<string, unknown>) {
      const record: WorkspaceLocalModel = {
        ...createSeedBaseModel(
          generateModelCode(),
          String(payload.displayName),
          String(payload.vendor ?? "OpenAI-Compatible"),
          (payload.protocol as ModelProtocol | undefined) ?? "OPENAI_COMPATIBLE",
          (payload.modelType as ModelType | undefined) ?? "TEXT",
          String(payload.baseUrl ?? "https://router.tron.ai/v1"),
          typeof payload.apiPath === "string" ? payload.apiPath : "/chat/completions",
          String(payload.apiKeyRef ?? `sk-workspace-${workspaceCode.toLowerCase()}`),
          String(payload.modelName),
        ),
        allowInheritance: Boolean(payload.allowInheritance ?? true),
        workspaceCode,
      };
      updateBaseModel(record, payload, { allowInheritance: Boolean(payload.allowInheritance ?? true) });
      workspaceLocalModels.push(record);
      return record.modelCode;
    },
    listPlatformModels() {
      const defaults = resolveEffectivePlatformDefaults();
      return sortModelRecords(platformModels.map((model) => buildPlatformView(model, defaults)));
    },
    updatePlatformModel(modelCode: string, payload: Record<string, unknown>) {
      const record = platformModels.find((item) => item.modelCode === modelCode);
      if (!record) throw new Error("model was not found");
      const previousType = record.modelType;
      const wasEnabled = record.enabled;
      updateBaseModel(record, payload);
      if (previousType !== record.modelType || (wasEnabled && !record.enabled)) {
        clearWorkspaceDefaultForModelCode(modelCode);
        modelTypeSequence.forEach((modelType) => {
          if (platformDefaults[modelType] === modelCode) {
            delete platformDefaults[modelType];
          }
        });
      }
      return buildPlatformView(record, resolveEffectivePlatformDefaults());
    },
    verifyPlatformModel(modelCode: string, payload?: Record<string, unknown>) {
      const record = platformModels.find((item) => item.modelCode === modelCode);
      if (!record) throw new Error("model was not found");
      if (!payload) {
        verifyBaseModel(record);
        return buildPlatformView(record, resolveEffectivePlatformDefaults());
      }
      const preview = cloneBaseModel(record);
      updateBaseModel(preview, payload);
      verifyBaseModel(preview);
      return buildPlatformView(preview, resolveEffectivePlatformDefaults());
    },
    getWorkspaceCatalog(workspaceCode: string) {
      return resolveWorkspaceCatalogState(workspaceCode).catalog;
    },
    setWorkspaceEnabled(workspaceCode: string, modelCode: string, enabled: boolean) {
      const record = resolveWorkspaceRecordState(workspaceCode, modelCode);
      if (record.view.sourceKind === "LOCAL") {
        const local = workspaceLocalModels.find((item) => item.modelCode === modelCode && item.workspaceCode === workspaceCode);
        if (!local) throw new Error("model was not found");
        local.enabled = enabled;
        local.updatedAt = now;
        if (!enabled) clearWorkspaceDefaultForModelCode(modelCode);
        return resolveWorkspaceRecordState(workspaceCode, modelCode).view;
      }
      if (record.view.sourceKind === "OVERRIDE") {
        const override = workspaceOverrides.find((item) => item.modelCode === record.currentOwned?.modelCode);
        if (!override) throw new Error("model was not found");
        override.enabled = enabled;
        override.updatedAt = now;
        if (!enabled) clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode, modelCode);
        return resolveWorkspaceRecordState(workspaceCode, modelCode).view;
      }

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
        workspaceStatusPatches.push({
          createdAt: now,
          enabled,
          modelCode: generateModelCode(),
          slotModelCode: modelCode,
          updatedAt: now,
          workspaceCode,
        });
      }
      if (!enabled) clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode, modelCode);
      return resolveWorkspaceRecordState(workspaceCode, modelCode).view;
    },
    setWorkspaceDefault(workspaceCode: string, modelCode: string) {
      const record = resolveWorkspaceRecordState(workspaceCode, modelCode);
      if (!record.view.enabled) throw new Error("default model must be enabled in the current workspace");
      workspaceDefaults[workspaceCode] = {
        ...(workspaceDefaults[workspaceCode] ?? {}),
        [record.view.modelType]: record.view.modelCode,
      };
      return { modelCode: record.view.modelCode, modelType: record.view.modelType, workspaceCode };
    },
    updateWorkspaceModel(workspaceCode: string, modelCode: string, payload: Record<string, unknown>) {
      const record = resolveWorkspaceRecordState(workspaceCode, modelCode);
      ensureWorkspaceDisplayNameUnique(workspaceCode, String(payload.displayName ?? record.view.displayName), modelCode);
      if (record.view.sourceKind === "LOCAL") {
        const local = workspaceLocalModels.find((item) => item.workspaceCode === workspaceCode && item.modelCode === modelCode);
        if (!local) throw new Error("model was not found");
        const previousType = local.modelType;
        const wasEnabled = local.enabled;
        updateBaseModel(local, payload, { allowInheritance: local.allowInheritance });
        if (previousType !== local.modelType || (wasEnabled && !local.enabled)) {
          clearWorkspaceDefaultForModelCode(modelCode);
        }
        return resolveWorkspaceRecordState(workspaceCode, modelCode).view;
      }

      if (record.currentOwned?.kind === "statusPatch") {
        deleteOwnedPointer(record.currentOwned);
      }

      const fallbackApiKeyRef = baseModelForDefinition(record.definition).apiKeyRef;
      const existingOverride = record.currentOwned?.kind === "override"
        ? workspaceOverrides.find((item) => item.modelCode === record.currentOwned?.modelCode)
        : findWorkspaceOverride(workspaceCode, modelCode);
      if (existingOverride) {
        const previousType = existingOverride.modelType;
        const wasEnabled = existingOverride.enabled;
        updateBaseModel(existingOverride, payload, { fallbackApiKeyRef });
        existingOverride.slotModelCode = modelCode;
        if (previousType !== existingOverride.modelType || (wasEnabled && !existingOverride.enabled)) {
          clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode, modelCode);
        }
      } else {
        const created: WorkspaceOverrideModel = {
          ...createSeedBaseModel(
            generateModelCode(),
            String(payload.displayName),
            String(payload.vendor ?? "OpenAI"),
            (payload.protocol as ModelProtocol | undefined) ?? "RESPONSES",
            (payload.modelType as ModelType | undefined) ?? "TEXT",
            String(payload.baseUrl ?? "https://api.openai.com/v1"),
            typeof payload.apiPath === "string" ? payload.apiPath : null,
            fallbackApiKeyRef,
            String(payload.modelName),
          ),
          slotModelCode: modelCode,
          workspaceCode,
        };
        updateBaseModel(created, payload, { fallbackApiKeyRef });
        workspaceOverrides.push(created);
        if (!created.enabled) clearWorkspaceDefaultForWorkspaceModelCode(workspaceCode, modelCode);
      }

      return resolveWorkspaceRecordState(workspaceCode, modelCode).view;
    },
    deleteWorkspaceModel(workspaceCode: string, modelCode: string) {
      const record = resolveWorkspaceRecordState(workspaceCode, modelCode);
      if (record.view.sourceKind === "INHERITED") {
        throw new Error("inherited models must be overridden or disabled instead of deleted");
      }
      if (record.view.sourceKind === "LOCAL") {
        workspaceLocalModels = workspaceLocalModels.filter((item) => !(item.workspaceCode === workspaceCode && item.modelCode === modelCode));
        removeWorkspaceSlotDependents(modelCode);
        clearWorkspaceDefaultForModelCode(modelCode);
        return { modelCode, workspaceCode };
      }
      deleteOwnedPointer(record.currentOwned);
      return { modelCode, workspaceCode };
    },
    verifyWorkspaceModel(workspaceCode: string, modelCode: string, payload?: Record<string, unknown>) {
      const record = resolveWorkspaceRecordState(workspaceCode, modelCode);
      if (!payload) {
        verifyBaseModel(baseModelForDefinition(record.definition));
        return resolveWorkspaceRecordState(workspaceCode, modelCode).view;
      }
      const preview = cloneBaseModel(baseModelForDefinition(record.definition));
      updateBaseModel(preview, payload, { fallbackApiKeyRef: baseModelForDefinition(record.definition).apiKeyRef });
      verifyBaseModel(preview);
      return {
        ...record.view,
        apiKeyRef: maskApiKeyValue(preview.apiKeyRef),
        apiPath: preview.apiPath,
        baseUrl: preview.baseUrl,
        description: preview.description,
        displayName: preview.displayName,
        lastVerifiedAt: preview.lastVerifiedAt,
        lastVerifyMessage: preview.lastVerifyMessage,
        lastVerifyStatus: preview.lastVerifyStatus,
        maxRetries: preview.maxRetries,
        maxTokens: preview.maxTokens,
        modelCapabilities: [...preview.modelCapabilities],
        modelName: preview.modelName,
        modelType: preview.modelType,
        protocol: preview.protocol,
        structuredOutputEnabled: preview.structuredOutputEnabled,
        temperature: preview.temperature,
        thinkingEnabled: preview.thinkingEnabled,
        timeoutMs: preview.timeoutMs,
        topP: preview.topP,
        updatedAt: preview.updatedAt,
        vendor: preview.vendor,
      };
    },
  };
}

async function routeApi(page: Page, configure?: (state: ReturnType<typeof createMockState>) => void) {
  const state = createMockState();
  configure?.(state);

  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const method = request.method();
    const payload = request.postDataJSON() as Record<string, unknown> | null;

    if (url.pathname === "/api/v1/auth/session" && method === "GET") {
      return fulfillJson(route, successEnvelope(authenticatedSession));
    }
    if (url.pathname === "/api/v1/auth/refresh" && method === "POST") {
      return fulfillJson(route, successEnvelope(authenticatedGrant));
    }
    if (url.pathname === "/api/v1/admin/workspaces/switcher" && method === "GET") {
      return fulfillJson(route, successEnvelope(workspaceSwitcher));
    }

    if (url.pathname === "/api/v1/admin/platform/models" && method === "GET") {
      return fulfillJson(route, successEnvelope(state.listPlatformModels()));
    }

    const platformMutationMatch = url.pathname.match(/^\/api\/v1\/admin\/platform\/models\/([^/]+)(?:\/([^/]+))?$/);
    if (platformMutationMatch) {
      const [, modelCode, action] = platformMutationMatch;
      try {
        if (method === "PATCH" && !action) {
          return fulfillJson(route, successEnvelope(state.updatePlatformModel(modelCode, payload ?? {})));
        }
        if (method === "POST" && action === "verify") {
          return fulfillJson(route, successEnvelope(state.verifyPlatformModel(modelCode, payload ?? undefined)));
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : "platform model mutation failed";
        return fulfillJson(route, errorEnvelope(message), 400);
      }
    }

    const workspaceCatalogMatch = url.pathname.match(/^\/api\/v1\/models\/workspaces\/([^/]+)$/);
    if (workspaceCatalogMatch && method === "GET") {
      return fulfillJson(route, successEnvelope(state.getWorkspaceCatalog(workspaceCatalogMatch[1])));
    }

    const workspaceMutationMatch = url.pathname.match(/^\/api\/v1\/models\/workspaces\/([^/]+)\/([^/]+)(?:\/([^/]+))?$/);
    if (workspaceMutationMatch) {
      const [, workspaceCode, modelCode, action] = workspaceMutationMatch;
      try {
        if (method === "PATCH" && !action) {
          return fulfillJson(route, successEnvelope(state.updateWorkspaceModel(workspaceCode, modelCode, payload ?? {})));
        }
        if (method === "DELETE" && !action) {
          return fulfillJson(route, successEnvelope(state.deleteWorkspaceModel(workspaceCode, modelCode)));
        }
        if (method === "POST" && action === "disable") {
          return fulfillJson(route, successEnvelope(state.setWorkspaceEnabled(workspaceCode, modelCode, false)));
        }
        if (method === "POST" && action === "enable") {
          return fulfillJson(route, successEnvelope(state.setWorkspaceEnabled(workspaceCode, modelCode, true)));
        }
        if (method === "POST" && action === "set-default") {
          return fulfillJson(route, successEnvelope(state.setWorkspaceDefault(workspaceCode, modelCode)));
        }
        if (method === "POST" && action === "verify") {
          return fulfillJson(route, successEnvelope(state.verifyWorkspaceModel(workspaceCode, modelCode, payload ?? undefined)));
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : "workspace model mutation failed";
        return fulfillJson(route, errorEnvelope(message), 400);
      }
    }

    return fulfillJson(route, { code: "NOT_FOUND", message: `${method} ${url.pathname} is not mocked`, data: {} }, 404);
  });
}

async function openModelsPage(page: Page) {
  await page.goto(`${frontendBaseUrl}/models`);
  await expect(page.getByRole("heading", { name: "Models" })).toBeVisible();
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;
  frontendProcess = spawn("npm", ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: frontendRoot,
    env: {
      ...process.env,
      CI: "1",
      VITE_TRON_ADMIN_DATA_SOURCE: "api",
      VITE_TRON_DATA_SOURCE: "api",
      VITE_TRON_MODEL_DATA_SOURCE: "api",
    },
    stdio: "pipe",
  });
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);
  await waitForFrontend(`${frontendBaseUrl}/models`);
});

test.afterAll(async () => {
  if (!frontendProcess || frontendProcess.killed) return;
  frontendProcess.kill("SIGTERM");
  await new Promise((resolveProcess) => frontendProcess?.once("exit", resolveProcess));
});

test("workspace models show local and inherited source semantics with inherited-from lineage", async ({ page }) => {
  await routeApi(page, (state) => {
    state.createWorkspaceModel("PLATFORM", {
      displayName: "Parent Shared Writer",
      vendor: "OpenAI-Compatible",
      protocol: "OPENAI_COMPATIBLE",
      modelType: "TEXT",
      modelCapabilities: ["TEXT_GENERATION"],
      baseUrl: "https://router.platform.ai/v1",
      apiPath: "/chat/completions",
      apiKeyRef: "sk-parent-shared-writer",
      modelName: "gpt-4.1-mini",
      enabled: true,
      allowInheritance: true,
      thinkingEnabled: true,
      structuredOutputEnabled: false,
    });
  });

  await openModelsPage(page);
  await expect(page.getByRole("columnheader", { name: "Source" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Inherited From" })).toBeVisible();
  await expect(page.getByText("Protocol Type", { exact: true })).toHaveCount(0);

  const tableBody = page.locator("tbody");
  const protocolFilter = page.getByLabel("Protocol", { exact: true });
  const typeFilter = page.getByLabel("Type", { exact: true });
  await expect(protocolFilter).toBeVisible();
  await expect(typeFilter).toBeVisible();
  await expect(protocolFilter).not.toContainText("·");
  await protocolFilter.selectOption("RESPONSES");
  await expect(tableBody).toContainText("GPT-4o Main");
  await expect(tableBody).not.toContainText("Core Fast");
  await protocolFilter.selectOption("ALL");
  await typeFilter.selectOption("EMBEDDING::TEXT_VECTOR");
  await expect(tableBody).toContainText("Embedding Shared");
  await expect(tableBody).not.toContainText("Core Fast");
  await typeFilter.selectOption("ALL");

  const inheritedRow = page.locator("tr", { has: page.getByText("Parent Shared Writer", { exact: true }) });
  await expect(inheritedRow).toContainText("Inherited");
  await expect(inheritedRow).toContainText("Enterprise / Platform");
  await expect(inheritedRow).not.toContainText("ENTERPRISE / PLATFORM");

  const localRow = page.locator("tr", { has: page.getByText("Core Fast", { exact: true }) });
  await expect(localRow).toContainText("Local");

  await page.getByRole("button", { name: "Open actions for GPT-4o Main", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: "Copy" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Edit" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Disable" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Verify" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Delete" })).toHaveCount(0);
  await expect(page.getByRole("menuitem", { name: "Revert" })).toHaveCount(0);
});

test("disabling an inherited platform model keeps source inherited and does not mutate platform defaults", async ({ page }) => {
  await routeApi(page);
  await openModelsPage(page);

  const inheritedRow = page.locator("tr", { has: page.getByText("GPT-4o Main", { exact: true }) });
  await expect(inheritedRow).toContainText("Default");
  await expect(inheritedRow).toContainText("Enabled");

  await page.getByRole("button", { name: "Open actions for GPT-4o Main", exact: true }).click();
  await page.getByRole("menuitem", { name: "Disable" }).click();
  await expect(toast(page, "GPT-4o Main disabled")).toBeVisible();
  await expect(inheritedRow).toContainText("Inherited");
  await expect(inheritedRow).toContainText("Disabled");
  await expect(inheritedRow.getByText("Default", { exact: true })).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/settings/models`);
  await expect(page.getByRole("heading", { name: "Platform Models" })).toBeVisible();
  const platformRow = page.locator("tr", { has: page.getByText("GPT-4o Main", { exact: true }) });
  await expect(platformRow).toContainText("Enabled");
  await expect(platformRow).toContainText("Default");
});

test("editing an inherited platform model creates an override that can be reverted", async ({ page }) => {
  await routeApi(page);
  await openModelsPage(page);

  await page.getByText("GPT-4o Main", { exact: true }).first().click();
  await expect(page).toHaveURL(/\/models\/detail\?modelCode=MODEL-734600000000000001$/);
  await page.getByRole("button", { name: "Edit" }).click();
  await page.getByLabel("Display Name").fill("GPT-4o Workspace");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(toast(page, "GPT-4o Workspace saved")).toBeVisible();
  await expect(page.getByText("Override", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "Revert" })).toBeVisible();

  await page.getByRole("button", { name: "Revert" }).click();
  await expect(toast(page, "GPT-4o Workspace reverted to inherited")).toBeVisible();
  await expect(page).toHaveURL(`${frontendBaseUrl}/models`);
  await expect(page.getByText("GPT-4o Main", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("GPT-4o Workspace", { exact: true })).toHaveCount(0);
});

test("workspace default follows the slot across inherited override and revert", async ({ page }) => {
  await routeApi(page, (state) => {
    state.createPlatformModel({
      displayName: "GPT-4.1 Shared",
      vendor: "OpenAI",
      protocol: "RESPONSES",
      modelType: "TEXT",
      modelCapabilities: ["TEXT_GENERATION"],
      baseUrl: "https://api.openai.com/v1",
      apiPath: "/responses",
      apiKeyRef: "sk-platform-gpt41-shared",
      modelName: "gpt-4.1",
      enabled: true,
      thinkingEnabled: true,
      structuredOutputEnabled: true,
    });
  });

  await openModelsPage(page);
  const sharedRow = page.locator("tr", { has: page.getByText("GPT-4.1 Shared", { exact: true }) });
  await page.getByRole("button", { name: "Open actions for GPT-4.1 Shared", exact: true }).click();
  await page.getByRole("menuitem", { name: "Set Default" }).click();
  await expect(toast(page, /set as the text default/i)).toBeVisible();
  await expect(sharedRow).toContainText("Default");

  await page.getByText("GPT-4.1 Shared", { exact: true }).first().click();
  await page.getByRole("button", { name: "Edit" }).click();
  await page.getByLabel("Display Name").fill("GPT-4.1 Workspace");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(toast(page, "GPT-4.1 Workspace saved")).toBeVisible();
  await expect(page.getByText("Override", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Default", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { name: "Revert" }).click();
  await expect(toast(page, "GPT-4.1 Workspace reverted to inherited")).toBeVisible();
  const revertedRow = page.locator("tr", { has: page.getByText("GPT-4.1 Shared", { exact: true }) });
  await expect(revertedRow).toContainText("Default");
  await expect(page.getByText("GPT-4.1 Workspace", { exact: true })).toHaveCount(0);
});

test("workspace detail verify uses the unsaved draft without saving the row", async ({ page }) => {
  await routeApi(page);
  await openModelsPage(page);

  await page.getByText("GPT-4o Main", { exact: true }).first().click();
  await expect(page).toHaveURL(/\/models\/detail\?modelCode=MODEL-734600000000000001$/);
  await page.getByRole("button", { name: "Edit" }).click();
  await page.getByLabel("Display Name").fill("GPT-4o Draft Verify");
  await page.getByRole("button", { name: "Verify" }).click();
  await expect(toast(page, "GPT-4o Draft Verify verified")).toBeVisible();
  await expect(page.getByRole("button", { name: "Save" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Cancel" })).toBeVisible();
  await expect(page.getByText("Last checked:", { exact: false })).toContainText("May");
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByText("GPT-4o Main", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("GPT-4o Draft Verify", { exact: true })).toHaveCount(0);
});

test("platform detail verify uses the unsaved draft without saving the row", async ({ page }) => {
  await routeApi(page);
  await page.goto(`${frontendBaseUrl}/settings/models`);
  await expect(page.getByRole("heading", { name: "Platform Models" })).toBeVisible();

  await page.getByRole("button", { name: "Open actions for GPT-4o Main", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: "Verify" })).toBeVisible();
  await page.getByRole("menuitem", { name: "Verify" }).click();
  await expect(toast(page, "GPT-4o Main verified")).toBeVisible();

  await page.getByText("GPT-4o Main", { exact: true }).first().click();
  await expect(page).toHaveURL(/\/settings\/models\/detail\?modelCode=MODEL-734600000000000001$/);
  await page.getByRole("button", { name: "Edit" }).click();
  await page.getByLabel("Display Name").fill("GPT-4o Platform Draft Verify");
  await page.getByRole("button", { name: "Verify" }).click();
  await expect(toast(page, "GPT-4o Platform Draft Verify verified")).toBeVisible();
  await expect(page.getByRole("button", { name: "Save" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Cancel" })).toBeVisible();
  await expect(page.getByText("Last checked:", { exact: false })).toContainText("May");
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByText("GPT-4o Main", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("GPT-4o Platform Draft Verify", { exact: true })).toHaveCount(0);
});

test("structured output clears and disables max tokens in workspace and platform editors", async ({ page }) => {
  await routeApi(page, (state) => {
    state.createPlatformModel({
      displayName: "Platform Chat Draft",
      vendor: "OpenAI-Compatible",
      protocol: "OPENAI_COMPATIBLE",
      modelType: "TEXT",
      modelCapabilities: ["TEXT_GENERATION"],
      baseUrl: "https://router.platform.ai/v1",
      apiPath: "/chat/completions",
      apiKeyRef: "sk-platform-chat-draft",
      modelName: "gpt-4.1-mini",
      enabled: true,
      thinkingEnabled: true,
      structuredOutputEnabled: false,
    });
  });
  await openModelsPage(page);

  await page.getByText("Core Fast", { exact: true }).first().click();
  await page.getByRole("button", { name: "Edit" }).click();
  const workspaceMaxTokens = page.getByLabel("Max Tokens");
  const workspaceStructuredOutput = page.getByRole("button", { name: "Structured Output Enabled" });
  await workspaceMaxTokens.fill("2048");
  await workspaceStructuredOutput.click();
  await expect(workspaceMaxTokens).toHaveValue("");
  await expect(workspaceMaxTokens).toBeDisabled();
  await expect(workspaceMaxTokens).toHaveCSS("background-color", "rgb(241, 237, 236)");
  await workspaceStructuredOutput.click();
  await expect(workspaceMaxTokens).toHaveValue("4096");
  await expect(workspaceMaxTokens).toBeEnabled();
  await page.getByRole("button", { name: "Cancel" }).click();

  await page.goto(`${frontendBaseUrl}/settings/models`);
  await expect(page.getByRole("heading", { name: "Platform Models" })).toBeVisible();
  await page.getByText("Platform Chat Draft", { exact: true }).first().click();
  await page.getByRole("button", { name: "Edit" }).click();
  const platformMaxTokens = page.getByLabel("Max Tokens");
  const platformStructuredOutput = page.getByRole("button", { name: "Structured Output Enabled" });
  await platformMaxTokens.fill("1024");
  await platformStructuredOutput.click();
  await expect(platformMaxTokens).toHaveValue("");
  await expect(platformMaxTokens).toBeDisabled();
  await expect(platformMaxTokens).toHaveCSS("background-color", "rgb(241, 237, 236)");
  await platformStructuredOutput.click();
  await expect(platformMaxTokens).toHaveValue("4096");
  await expect(platformMaxTokens).toBeEnabled();
});

import type {
  ModelCostLine,
  WorkspaceModelMutationResult,
  WorkspaceModelQuery,
  WorkspaceModelRecord,
  WorkspaceModelStatus,
} from "@/domain/model/model/ModelRegistryModel";
import { getWorkspaceOptionMocks } from "@/shared/constants/workspaceSpaces";

type PlatformModelSeed = Omit<WorkspaceModelRecord, "removedAt" | "sourceLabel" | "sourceWorkspaceCode" | "sourceWorkspaceName" | "status"> & {
  enabled: boolean;
};

const platformModels: PlatformModelSeed[] = [
  {
    modelCode: "gpt-4-turbo",
    name: "gpt-4-turbo",
    purpose: "Default Reasoning",
    vendor: "OpenAI",
    context: "128k tokens",
    price: "$10.00 / 1M Input",
    maxOutput: "4,096 tokens",
    trainingData: "Dec 2023",
    speed: "~42 tk/s",
    enabled: true,
  },
  {
    modelCode: "claude-3-opus-20240229",
    name: "claude-3-opus-20240229",
    purpose: "Complex Tasks",
    vendor: "Anthropic",
    context: "200k tokens",
    price: "$15.00 / 1M Input",
    maxOutput: "4,096 tokens",
    trainingData: "Aug 2023",
    speed: "~31 tk/s",
    enabled: true,
  },
  {
    modelCode: "gemini-1.5-pro",
    name: "gemini-1.5-pro",
    purpose: "Multimodal Review",
    vendor: "Google",
    context: "2M tokens",
    price: "$7.00 / 1M Input",
    maxOutput: "8,192 tokens",
    trainingData: "Apr 2024",
    speed: "~36 tk/s",
    enabled: true,
  },
  {
    modelCode: "gpt-3.5-turbo",
    name: "gpt-3.5-turbo",
    purpose: "Legacy Fast",
    vendor: "OpenAI",
    context: "16k tokens",
    price: "$0.50 / 1M Input",
    maxOutput: "2,048 tokens",
    trainingData: "Sep 2021",
    speed: "~91 tk/s",
    enabled: false,
  },
];

const removalState = new Map<string, Set<string>>();
const platformWorkspaceCode = "PLATFORM";
const platformWorkspaceName = "Platform";
const platformSourceLabel = "Platform";
const defaultTimestamp = "2026-05-22T00:00:00Z";

export function getModelCostPreview(): ModelCostLine[] {
  return [
    { label: "Input (1M Tokens)", value: "$10.00" },
    { label: "Output (1M Tokens)", value: "$30.00" },
    { label: "Context Window", value: "128,000" },
  ];
}

function getWorkspace(workspaceCode: string) {
  return getWorkspaceOptionMocks().find((record) => record.workspaceCode === workspaceCode);
}

function requireWorkspace(workspaceCode: string) {
  const record = getWorkspace(workspaceCode);
  if (!record) {
    throw new Error(`Workspace ${workspaceCode} was not found.`);
  }
  return record;
}

function requireEnabledPlatformModel(modelCode: string) {
  const record = platformModels.find((model) => model.modelCode === modelCode && model.enabled);
  if (!record) {
    throw new Error(`Model ${modelCode} was not found.`);
  }
  return record;
}

function getRemovalSet(workspaceCode: string) {
  const current = removalState.get(workspaceCode);
  if (current) return current;
  const created = new Set<string>();
  removalState.set(workspaceCode, created);
  return created;
}

function matchesSearch(record: WorkspaceModelRecord, search?: string) {
  if (!search?.trim()) return true;
  return [
    record.modelCode,
    record.name,
    record.purpose,
    record.vendor,
    record.context,
    record.sourceWorkspaceCode,
    record.sourceWorkspaceName,
    record.sourceLabel,
  ].join(" ").toLowerCase().includes(search.trim().toLowerCase());
}

function matchesStatus(record: WorkspaceModelRecord, status?: WorkspaceModelStatus) {
  return status ? record.status === status : true;
}

export function listWorkspaceModels(
  workspaceCode: string,
  query: WorkspaceModelQuery = {},
): WorkspaceModelRecord[] {
  requireWorkspace(workspaceCode);
  const removed = workspaceCode === platformWorkspaceCode ? new Set<string>() : getRemovalSet(workspaceCode);

  return platformModels
    .filter((model) => model.enabled)
    .map((model) => {
      const status: WorkspaceModelStatus = removed.has(model.modelCode) ? "Removed" : "Available";
      return {
        modelCode: model.modelCode,
        name: model.name,
        purpose: model.purpose,
        vendor: model.vendor,
        context: model.context,
        maxOutput: model.maxOutput,
        trainingData: model.trainingData,
        speed: model.speed,
        price: model.price,
        status,
        sourceWorkspaceCode: platformWorkspaceCode,
        sourceWorkspaceName: platformWorkspaceName,
        sourceLabel: platformSourceLabel,
        removedAt: status === "Removed" ? defaultTimestamp : null,
      };
    })
    .filter((record) => query.vendor ? record.vendor === query.vendor : true)
    .filter((record) => matchesStatus(record, query.status))
    .filter((record) => matchesSearch(record, query.search))
    .sort((left, right) => left.name.localeCompare(right.name) || left.modelCode.localeCompare(right.modelCode));
}

export function removeWorkspaceModelRecord(
  workspaceCode: string,
  modelCode: string,
): WorkspaceModelMutationResult {
  requireWorkspace(workspaceCode);
  requireEnabledPlatformModel(modelCode);
  if (workspaceCode === platformWorkspaceCode) {
    throw new Error("Platform workspace is the source of truth; remove from a child workspace instead");
  }

  getRemovalSet(workspaceCode).add(modelCode);
  return {
    workspaceCode,
    modelCode,
    status: "Removed",
    removedAt: defaultTimestamp,
  };
}

export function restoreWorkspaceModelRecord(
  workspaceCode: string,
  modelCode: string,
): WorkspaceModelMutationResult {
  requireWorkspace(workspaceCode);
  requireEnabledPlatformModel(modelCode);
  if (workspaceCode === platformWorkspaceCode) {
    throw new Error("Platform workspace is the source of truth; remove from a child workspace instead");
  }

  getRemovalSet(workspaceCode).delete(modelCode);
  return {
    workspaceCode,
    modelCode,
    status: "Available",
    removedAt: null,
  };
}

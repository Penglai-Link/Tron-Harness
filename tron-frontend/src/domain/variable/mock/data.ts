import type {
  PlatformVariableRecord,
  PromptVariableOption,
  VariableValueMode,
  VariableRecord,
  VariableSourceKind,
} from "@/domain/variable/model";
import { workspaceOptionMocks } from "@/shared/constants/workspaceSpaces";

const sensitiveMask = "********";

interface VariableSeed {
  createdAt: string;
  description: string | null;
  isSensitive?: boolean;
  key: string;
  updatedAt: string;
  value: string;
  workspaceCode: string;
}

const now = "2026-05-18T00:00:00Z";

const runtimePromptVariableOptions: PromptVariableOption[] = [
  ["visible_domains", "JSON", "Visible tool domains and their operations.", "AGENT_VISIBLE_DOMAINS", "Agent tool domains"],
  ["persistence_mode", "STRING", "How expanded domain tools are persisted.", "AGENT_DOMAIN_PERSISTENCE_MODE", "Agent domain persistence mode"],
  ["user_message", "STRING", "The first user message.", "CHAT_USER_MESSAGE", "Chat user message"],
  ["command", "STRING", "Command being analyzed.", "TERMINAL_EXPLAIN_COMMAND", "Terminal command"],
  ["output", "STRING", "Command output being analyzed.", "TERMINAL_EXPLAIN_OUTPUT", "Terminal output"],
  ["current_command", "STRING", "Current command intent.", "TERMINAL_CURRENT_COMMAND", "Terminal current command"],
  ["user_request", "STRING", "User request for a command.", "TERMINAL_USER_REQUEST", "Terminal user request"],
  ["task_context", "STRING", "Structured task metadata and description.", "TASK_CONTEXT", "Task context"],
  ["title", "STRING", "Tree node title.", "RAG_TREE_TITLE", "RAG tree title"],
  ["source_text", "STRING", "Text covered by the node.", "RAG_TREE_SOURCE_TEXT", "RAG tree source text"],
  ["child_summaries", "STRING", "Summaries generated for child nodes.", "RAG_TREE_CHILD_SUMMARIES", "RAG tree child summaries"],
].map(([key, type, description, runtimeProvider, sourceLabel]) => ({
  id: `RUNTIME:${runtimeProvider}:${key}`,
  key,
  type: type as PromptVariableOption["type"],
  description,
  sourceKind: "RUNTIME" as const,
  sourceLabel,
  managedVariableKey: null,
  runtimeProvider: runtimeProvider as NonNullable<PromptVariableOption["runtimeProvider"]>,
}));

const mockVariables: VariableSeed[] = [
  {
    workspaceCode: "ENTERPRISE",
    key: "DEFAULT_REGION",
    value: "us-east-1",
    description: "Shared default deployment region.",
    createdAt: now,
    updatedAt: now,
  },
  {
    workspaceCode: "ENTERPRISE",
    key: "ROUTER_TIMEOUT_MS",
    value: "800",
    description: "Root timeout for interactive routing.",
    createdAt: now,
    updatedAt: now,
  },
  {
    workspaceCode: "PLATFORM",
    key: "OPENAI_API_BASE_URL",
    value: "https://api.openai.com/v1",
    description: "Shared OpenAI gateway base URL.",
    createdAt: now,
    updatedAt: now,
  },
  {
    workspaceCode: "PLATFORM",
    key: "ROUTER_TIMEOUT_MS",
    value: "600",
    description: "Platform override for interactive latency.",
    createdAt: now,
    updatedAt: now,
  },
  {
    workspaceCode: "TRON_CORE",
    key: "OPENAI_API_KEY",
    value: sensitiveMask,
    description: "Workspace secret used by the demo workspace.",
    isSensitive: true,
    createdAt: now,
    updatedAt: now,
  },
  {
    workspaceCode: "TRON_CORE",
    key: "ROUTER_TIMEOUT_MS",
    value: "400",
    description: "Tron Core override for agent console flows.",
    createdAt: now,
    updatedAt: now,
  },
  {
    workspaceCode: "COMPLIANCE",
    key: "AUDIT_RETENTION_DAYS",
    value: "365",
    description: "Retention period for compliance exports.",
    createdAt: now,
    updatedAt: now,
  },
];

const mockPlatformVariables: PlatformVariableRecord[] = [
  {
    id: "734500000000100001",
    key: "GLOBAL_LLM_TIMEOUT_MS",
    value: "1200",
    description: "Platform-wide default timeout for LLM requests.",
    createdAt: now,
    updatedAt: now,
  },
  {
    id: "734500000000100002",
    key: "DEFAULT_REGION",
    value: "us-east-1",
    description: "Global platform deployment region.",
    createdAt: now,
    updatedAt: now,
  },
  {
    id: "734500000000100003",
    key: "PLATFORM_PROVIDER_CREDENTIAL",
    value: sensitiveMask,
    description: "Masked provider credential for platform inheritance.",
    createdAt: now,
    updatedAt: now,
    isSensitive: true,
    valueMasked: true,
    hasValue: true,
  },
];

export interface VariableWritePayload {
  description?: string | null;
  isSensitive?: boolean;
  key: string;
  value?: string;
  valueMode?: VariableValueMode;
}

export function listVariableCatalog(workspaceCode: string): VariableRecord[] {
  const lineage = workspaceLineage(workspaceCode);
  const currentWorkspace = lineage.at(-1);
  if (!currentWorkspace) return [];

  const resolved = new Map<string, VariableRecord>();
  for (const variable of mockPlatformVariables) {
    resolved.set(variable.key, {
      key: variable.key,
      value: variable.value,
      description: variable.description,
      isSensitive: Boolean(variable.isSensitive),
      valueMasked: Boolean(variable.isSensitive),
      hasValue: Boolean(variable.value),
      sourceKind: "Inherited",
      sourceWorkspaceCode: "PLATFORM",
      sourceWorkspaceName: "Platform Configuration",
      sourceLabel: "Platform Configuration",
      shadowedWorkspaceCode: null,
      shadowedWorkspaceName: null,
      shadowedLabel: null,
      createdAt: variable.createdAt,
      updatedAt: variable.updatedAt,
    });
  }
  for (const workspace of lineage) {
    const workspaceVariables = mockVariables
      .filter((variable) => variable.workspaceCode === workspace.workspaceCode)
      .sort((left, right) => left.key.localeCompare(right.key));
    for (const variable of workspaceVariables) {
      const previous = resolved.get(variable.key);
      const sourceKind: VariableSourceKind = previous
        ? (workspace.workspaceCode === currentWorkspace.workspaceCode ? "Override" : "Inherited")
        : (workspace.workspaceCode === currentWorkspace.workspaceCode ? "Local" : "Inherited");
      resolved.set(variable.key, {
        key: variable.key,
        value: variable.value,
        description: variable.description,
        isSensitive: Boolean(variable.isSensitive),
        valueMasked: Boolean(variable.isSensitive),
        hasValue: Boolean(variable.value),
        sourceKind,
        sourceWorkspaceCode: workspace.workspaceCode,
        sourceWorkspaceName: workspace.name,
        sourceLabel: workspace.label,
        shadowedWorkspaceCode: sourceKind === "Override" ? previous?.sourceWorkspaceCode ?? null : null,
        shadowedWorkspaceName: sourceKind === "Override" ? previous?.sourceWorkspaceName ?? null : null,
        shadowedLabel: sourceKind === "Override" ? previous?.sourceLabel ?? null : null,
        createdAt: variable.createdAt,
        updatedAt: variable.updatedAt,
      });
    }
  }

  return [...resolved.values()].sort((left, right) => left.key.localeCompare(right.key));
}

export function listPlatformVariableRecords() {
  return mockPlatformVariables
    .map((variable) => ({ ...variable }))
    .sort((left, right) => left.key.localeCompare(right.key));
}

function matchesPromptVariableOption(
  option: PromptVariableOption,
  search: string,
) {
  const normalizedSearch = search.trim().toLowerCase();
  if (!normalizedSearch) return true;
  return [option.key, option.description ?? "", option.sourceLabel, option.runtimeProvider ?? ""]
    .join(" ")
    .toLowerCase()
    .includes(normalizedSearch);
}

export function listWorkspacePromptVariableOptions(
  workspaceCode: string,
  search = "",
): PromptVariableOption[] {
  const managed = listVariableCatalog(workspaceCode)
    .filter((variable) => !variable.isSensitive && variable.hasValue)
    .map((variable) => ({
      id: `MANAGED:${variable.key}`,
      key: variable.key,
      type: "STRING" as const,
      description: variable.description,
      sourceKind: "MANAGED" as const,
      sourceLabel: variable.sourceLabel,
      managedVariableKey: variable.key,
      runtimeProvider: null,
    }));
  return [...managed, ...runtimePromptVariableOptions]
    .filter((variable) => matchesPromptVariableOption(variable, search));
}

export function listPlatformPromptVariableOptions(search = ""): PromptVariableOption[] {
  const managed = listPlatformVariableRecords()
    .filter((variable) => !variable.isSensitive && variable.hasValue)
    .map((variable) => ({
      id: `MANAGED:${variable.key}`,
      key: variable.key,
      type: "STRING" as const,
      description: variable.description,
      sourceKind: "MANAGED" as const,
      sourceLabel: "Platform Configuration",
      managedVariableKey: variable.key,
      runtimeProvider: null,
    }));
  return [...managed, ...runtimePromptVariableOptions]
    .filter((variable) => matchesPromptVariableOption(variable, search));
}

export function createPlatformVariableRecord(payload: VariableWritePayload) {
  const key = payload.key.trim();
  if (!key) throw new Error("Variable key is required");
  if (mockPlatformVariables.some((variable) => variable.key === key)) {
    throw new Error("Variable key already exists in platform variables");
  }
  const record: PlatformVariableRecord = {
    id: (734500000000100000n + BigInt(mockPlatformVariables.length + 1)).toString(),
    key,
    value: payload.isSensitive ? sensitiveMask : payload.value ?? "",
    description: normalizeDescription(payload.description),
    createdAt: now,
    updatedAt: now,
    isSensitive: Boolean(payload.isSensitive),
    valueMasked: Boolean(payload.isSensitive),
    hasValue: Boolean(payload.value),
  };
  mockPlatformVariables.push(record);
  return { ...record };
}

export function updatePlatformVariableRecord(key: string, payload: Omit<VariableWritePayload, "key">) {
  const existing = mockPlatformVariables.find((variable) => variable.key === key);
  if (!existing) throw new Error("Variable was not found");
  existing.isSensitive = Boolean(payload.isSensitive);
  existing.valueMasked = Boolean(payload.isSensitive);
  if (payload.valueMode === "PRESERVE" && existing.hasValue) {
    existing.value = existing.isSensitive ? sensitiveMask : existing.value;
  } else {
    existing.value = existing.isSensitive ? sensitiveMask : payload.value ?? "";
    existing.hasValue = Boolean(payload.value);
  }
  existing.description = normalizeDescription(payload.description);
  existing.updatedAt = now;
  return { ...existing };
}

export function deletePlatformVariableRecord(key: string) {
  const existingIndex = mockPlatformVariables.findIndex((variable) => variable.key === key);
  if (existingIndex < 0) throw new Error("Variable was not found");
  mockPlatformVariables.splice(existingIndex, 1);
  return { key, workspaceCode: "PLATFORM" };
}

export function createVariableRecord(workspaceCode: string, payload: VariableWritePayload) {
  const key = payload.key.trim();
  if (!key) throw new Error("Variable key is required");
  if (mockVariables.some((variable) => variable.workspaceCode === workspaceCode && variable.key === key)) {
    throw new Error("Variable key already exists in the current workspace");
  }
  mockVariables.push({
    workspaceCode,
    key,
    value: payload.isSensitive ? sensitiveMask : payload.value ?? "",
    description: normalizeDescription(payload.description),
    isSensitive: Boolean(payload.isSensitive),
    createdAt: now,
    updatedAt: now,
  });
  return listVariableCatalog(workspaceCode).find((variable) => variable.key === key)!;
}

export function updateVariableRecord(workspaceCode: string, key: string, payload: Omit<VariableWritePayload, "key">) {
  const existing = mockVariables.find((variable) => variable.workspaceCode === workspaceCode && variable.key === key);
  if (!existing) {
    if (listVariableCatalog(workspaceCode).some((variable) => variable.key === key)) {
      throw new Error("Override the inherited variable locally before modifying it");
    }
    throw new Error("Variable was not found");
  }
  existing.isSensitive = Boolean(payload.isSensitive);
  if (payload.valueMode === "PRESERVE" && existing.value) {
    existing.value = existing.isSensitive ? sensitiveMask : existing.value;
  } else {
    existing.value = existing.isSensitive ? sensitiveMask : payload.value ?? "";
  }
  existing.description = normalizeDescription(payload.description);
  existing.updatedAt = now;
  return listVariableCatalog(workspaceCode).find((variable) => variable.key === key)!;
}

export function deleteVariableRecord(workspaceCode: string, key: string) {
  const existingIndex = mockVariables.findIndex((variable) => variable.workspaceCode === workspaceCode && variable.key === key);
  if (existingIndex < 0) {
    if (listVariableCatalog(workspaceCode).some((variable) => variable.key === key)) {
      throw new Error("Override the inherited variable locally before modifying it");
    }
    throw new Error("Variable was not found");
  }
  mockVariables.splice(existingIndex, 1);
  return { key, workspaceCode };
}

function normalizeDescription(value?: string | null) {
  return value?.trim() ? value.trim() : null;
}

export function revealWorkspaceVariableRecord(workspaceCode: string, variableKey: string) {
  const record = listVariableCatalog(workspaceCode).find((variable) => variable.key === variableKey);
  if (!record) throw new Error("Variable was not found");
  return {
    value: record.isSensitive ? revealedPlaceholder(workspaceCode, variableKey) : record.value,
  };
}

export function revealPlatformVariableRecord(variableKey: string) {
  const record = mockPlatformVariables.find((variable) => variable.key === variableKey);
  if (!record) throw new Error("Variable was not found");
  return {
    value: record.isSensitive ? revealedPlaceholder("platform", variableKey) : record.value,
  };
}

function revealedPlaceholder(scope: string, key: string) {
  return ["revealed", scope.toLowerCase(), key.toLowerCase(), "value"].join("-");
}

function workspaceLineage(workspaceCode: string) {
  const byCode = new Map(workspaceOptionMocks.map((workspace) => [workspace.workspaceCode, workspace]));
  const lineage = [];
  let current = byCode.get(workspaceCode);
  while (current) {
    lineage.unshift(current);
    current = current.parentWorkspaceCode ? byCode.get(current.parentWorkspaceCode) : undefined;
  }
  return lineage;
}

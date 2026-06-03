import type {
  PlatformVariableRecord,
  VariableValueMode,
  VariableRecord,
  VariableSourceKind,
} from "@/domain/variable/model";
import { workspaceOptionMocks } from "@/shared/constants/workspaceSpaces";

const sensitiveMask = "********";

interface VariableSeed {
  description: string | null;
  isSensitive?: boolean;
  key: string;
  updatedAt: string;
  value: string;
  workspaceCode: string;
}

const now = "2026-05-18T00:00:00Z";

const mockVariables: VariableSeed[] = [
  {
    workspaceCode: "ENTERPRISE",
    key: "DEFAULT_REGION",
    value: "us-east-1",
    description: "Shared default deployment region.",
    updatedAt: now,
  },
  {
    workspaceCode: "ENTERPRISE",
    key: "ROUTER_TIMEOUT_MS",
    value: "800",
    description: "Root timeout for interactive routing.",
    updatedAt: now,
  },
  {
    workspaceCode: "PLATFORM",
    key: "OPENAI_API_BASE_URL",
    value: "https://api.openai.com/v1",
    description: "Shared OpenAI gateway base URL.",
    updatedAt: now,
  },
  {
    workspaceCode: "PLATFORM",
    key: "ROUTER_TIMEOUT_MS",
    value: "600",
    description: "Platform override for interactive latency.",
    updatedAt: now,
  },
  {
    workspaceCode: "TRON_CORE",
    key: "OPENAI_API_KEY",
    value: sensitiveMask,
    description: "Workspace secret used by the demo workspace.",
    isSensitive: true,
    updatedAt: now,
  },
  {
    workspaceCode: "TRON_CORE",
    key: "ROUTER_TIMEOUT_MS",
    value: "400",
    description: "Tron Core override for agent console flows.",
    updatedAt: now,
  },
  {
    workspaceCode: "COMPLIANCE",
    key: "AUDIT_RETENTION_DAYS",
    value: "365",
    description: "Retention period for compliance exports.",
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

export type StepUpTarget =
  | {
    scope: "platform";
    variableKey: string;
  }
  | {
    scope: "workspace";
    variableKey: string;
    workspaceCode: string;
  };

export interface StepUpPasswordMockPayload {
  password: string;
  target: StepUpTarget;
}

const mockStepUpTargets = new Map<string, StepUpTarget>();

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

export function verifyStepUpPasswordRecord(payload: StepUpPasswordMockPayload) {
  if (!payload.password.trim()) throw new Error("Step-up password is required");
  if (!payload.target.variableKey.trim()) throw new Error("Step-up target variable key is required");
  if (payload.target.scope === "workspace" && !payload.target.workspaceCode.trim()) {
    throw new Error("Step-up target workspace is required");
  }
  const stepUpToken = `mock-step-up-${Date.now()}-${mockStepUpTargets.size + 1}`;
  mockStepUpTargets.set(stepUpToken, payload.target);
  return {
    stepUpToken,
    expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
  };
}

export function revealWorkspaceVariableRecord(workspaceCode: string, variableKey: string, stepUpToken: string) {
  if (!stepUpToken.trim()) throw new Error("Step-up token is required");
  const target = mockStepUpTargets.get(stepUpToken);
  if (target?.scope !== "workspace" || target.workspaceCode !== workspaceCode || target.variableKey !== variableKey) {
    throw new Error("Step-up token target mismatch");
  }
  const record = listVariableCatalog(workspaceCode).find((variable) => variable.key === variableKey);
  if (!record) throw new Error("Variable was not found");
  return {
    value: record.isSensitive ? revealedPlaceholder(workspaceCode, variableKey) : record.value,
  };
}

export function revealPlatformVariableRecord(variableKey: string, stepUpToken: string) {
  if (!stepUpToken.trim()) throw new Error("Step-up token is required");
  const target = mockStepUpTargets.get(stepUpToken);
  if (target?.scope !== "platform" || target.variableKey !== variableKey) {
    throw new Error("Step-up token target mismatch");
  }
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

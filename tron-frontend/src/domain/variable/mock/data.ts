import type { VariableRecord, VariableSourceKind } from "@/domain/variable/model";
import { workspaceOptionMocks } from "@/shared/constants/workspaceSpaces";

interface VariableSeed {
  description: string | null;
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
    value: "sk-tron-core-demo",
    description: "Workspace secret used by the demo workspace.",
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

export interface VariableWritePayload {
  description?: string | null;
  key: string;
  value: string;
}

export function listVariableCatalog(workspaceCode: string): VariableRecord[] {
  const lineage = workspaceLineage(workspaceCode);
  const currentWorkspace = lineage.at(-1);
  if (!currentWorkspace) return [];

  const resolved = new Map<string, VariableRecord>();
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

export function createVariableRecord(workspaceCode: string, payload: VariableWritePayload) {
  const key = payload.key.trim();
  if (!key) throw new Error("Variable key is required");
  if (mockVariables.some((variable) => variable.workspaceCode === workspaceCode && variable.key === key)) {
    throw new Error("Variable key already exists in the current workspace");
  }
  mockVariables.push({
    workspaceCode,
    key,
    value: payload.value,
    description: normalizeDescription(payload.description),
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
  existing.value = payload.value;
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

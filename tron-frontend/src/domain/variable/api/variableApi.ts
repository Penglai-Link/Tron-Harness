import { requestApi } from "@/shared/api/httpClient";

import type {
  PlatformVariableRecord,
  PromptVariableOption,
  VariableValueMode,
  VariableRecord,
} from "@/domain/variable/model";

export const variableApiEndpoints = {
  verifyStepUpPassword: "POST /api/v1/auth/verify-step-up-password",
  listPlatformVariables: "GET /api/v1/admin/platform/variables",
  listPlatformPromptOptions: "GET /api/v1/admin/platform/variables/prompt-options",
  createPlatformVariable: "POST /api/v1/admin/platform/variables",
  updatePlatformVariable: "PATCH /api/v1/admin/platform/variables/{variableKey}",
  deletePlatformVariable: "DELETE /api/v1/admin/platform/variables/{variableKey}",
  revealPlatformVariable: "POST /api/v1/admin/platform/variables/reveal-platform-variable",
  listVariables: "GET /api/v1/variables/workspaces/{workspaceCode}",
  listWorkspacePromptOptions: "GET /api/v1/variables/workspaces/{workspaceCode}/prompt-options",
  createVariable: "POST /api/v1/variables/workspaces/{workspaceCode}",
  updateVariable: "PATCH /api/v1/variables/workspaces/{workspaceCode}/{variableKey}",
  deleteVariable: "DELETE /api/v1/variables/workspaces/{workspaceCode}/{variableKey}",
  revealWorkspaceVariable: "POST /api/v1/variables/reveal-workspace-variable",
} as const;

export interface VariableCatalogQuery {
  search?: string;
}

export interface CreateVariablePayload {
  key: string;
  value: string;
  description?: string | null;
  isSensitive?: boolean;
  valueMode?: VariableValueMode;
}

export interface UpdateVariablePayload {
  value?: string;
  description?: string | null;
  isSensitive?: boolean;
  valueMode?: VariableValueMode;
}

export interface VariableMutationResult {
  key: string;
  workspaceCode: string;
}

export interface StepUpPasswordPayload {
  password: string;
  target:
    | {
      scope: "platform";
      variableKey: string;
    }
    | {
      scope: "workspace";
      variableKey: string;
      workspaceCode: string;
    };
}

export interface StepUpPasswordResult {
  expiresAt: string;
  stepUpToken: string;
}

export interface RevealWorkspaceVariablePayload {
  stepUpToken: string;
  variableKey: string;
  workspaceCode: string;
}

export interface RevealPlatformVariablePayload {
  stepUpToken: string;
  variableKey: string;
}

export interface RevealVariableResult {
  value: string;
}

export function verifyStepUpPassword(payload: StepUpPasswordPayload) {
  return requestApi<StepUpPasswordResult>(variableApiEndpoints.verifyStepUpPassword, {
    body: payload,
  });
}

export function fetchPlatformVariables(signal?: AbortSignal) {
  return requestApi<PlatformVariableRecord[]>(variableApiEndpoints.listPlatformVariables, {
    signal,
  });
}

export function fetchPlatformPromptVariableOptions(
  query: VariableCatalogQuery = {},
  signal?: AbortSignal,
) {
  return requestApi<PromptVariableOption[]>(variableApiEndpoints.listPlatformPromptOptions, {
    cache: "no-store",
    queryParams: { search: query.search ?? "" },
    signal,
  });
}

export function createPlatformVariable(payload: CreateVariablePayload) {
  return requestApi<PlatformVariableRecord>(variableApiEndpoints.createPlatformVariable, {
    body: payload,
  });
}

export function updatePlatformVariable(variableKey: string, payload: UpdateVariablePayload) {
  return requestApi<PlatformVariableRecord>(variableApiEndpoints.updatePlatformVariable, {
    body: payload,
    pathParams: { variableKey },
  });
}

export function deletePlatformVariable(variableKey: string) {
  return requestApi<VariableMutationResult>(variableApiEndpoints.deletePlatformVariable, {
    pathParams: { variableKey },
  });
}

export function revealPlatformVariable(payload: RevealPlatformVariablePayload) {
  return requestApi<RevealVariableResult>(variableApiEndpoints.revealPlatformVariable, {
    body: payload,
  });
}

export function fetchVariableCatalog(
  workspaceCode: string,
  query: VariableCatalogQuery = {},
  signal?: AbortSignal,
) {
  return requestApi<VariableRecord[]>(variableApiEndpoints.listVariables, {
    pathParams: { workspaceCode },
    queryParams: {
      search: query.search ?? "",
    },
    signal,
  });
}

export function fetchWorkspacePromptVariableOptions(
  workspaceCode: string,
  query: VariableCatalogQuery = {},
  signal?: AbortSignal,
) {
  return requestApi<PromptVariableOption[]>(variableApiEndpoints.listWorkspacePromptOptions, {
    cache: "no-store",
    pathParams: { workspaceCode },
    queryParams: { search: query.search ?? "" },
    signal,
  });
}

export function createVariable(workspaceCode: string, payload: CreateVariablePayload) {
  return requestApi<VariableRecord>(variableApiEndpoints.createVariable, {
    body: payload,
    pathParams: { workspaceCode },
  });
}

export function updateVariable(
  workspaceCode: string,
  variableKey: string,
  payload: UpdateVariablePayload,
) {
  return requestApi<VariableRecord>(variableApiEndpoints.updateVariable, {
    body: payload,
    pathParams: {
      workspaceCode,
      variableKey,
    },
  });
}

export function deleteVariable(workspaceCode: string, variableKey: string) {
  return requestApi<VariableMutationResult>(variableApiEndpoints.deleteVariable, {
    pathParams: {
      workspaceCode,
      variableKey,
    },
  });
}

export function revealWorkspaceVariable(payload: RevealWorkspaceVariablePayload) {
  return requestApi<RevealVariableResult>(variableApiEndpoints.revealWorkspaceVariable, {
    body: payload,
  });
}

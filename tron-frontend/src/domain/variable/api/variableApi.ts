import { requestApi } from "@/shared/api/httpClient";

import type { VariableRecord } from "@/domain/variable/model";

export const variableApiEndpoints = {
  listVariables: "GET /api/v1/variables/workspaces/{workspaceCode}",
  createVariable: "POST /api/v1/variables/workspaces/{workspaceCode}",
  updateVariable: "PATCH /api/v1/variables/workspaces/{workspaceCode}/{variableKey}",
  deleteVariable: "DELETE /api/v1/variables/workspaces/{workspaceCode}/{variableKey}",
} as const;

export interface VariableCatalogQuery {
  search?: string;
}

export interface CreateVariablePayload {
  key: string;
  value: string;
  description?: string | null;
}

export interface UpdateVariablePayload {
  value: string;
  description?: string | null;
}

export interface VariableMutationResult {
  key: string;
  workspaceCode: string;
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

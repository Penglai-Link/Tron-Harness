import { requestApi } from "@/shared/api/httpClient";

import type {
  WorkspaceModelMutationResult,
  WorkspaceModelQuery,
  WorkspaceModelRecord,
} from "@/domain/model/model";

export const modelApiEndpoints = {
  listModels: "GET /api/v1/models/workspaces/{workspaceCode}",
  removeWorkspaceModel: "POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/remove",
  restoreWorkspaceModel: "POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/restore",
  getModelCostPreview: "GET /api/v1/models/cost-preview",
} as const;

export function fetchWorkspaceModels(
  workspaceCode: string,
  query: WorkspaceModelQuery = {},
  signal?: AbortSignal,
) {
  return requestApi<WorkspaceModelRecord[]>(modelApiEndpoints.listModels, {
    pathParams: { workspaceCode },
    queryParams: {
      search: query.search ?? "",
      status: query.status ?? "",
      vendor: query.vendor ?? "",
    },
    signal,
  });
}

export function removeWorkspaceModel(workspaceCode: string, modelCode: string) {
  return requestApi<WorkspaceModelMutationResult>(modelApiEndpoints.removeWorkspaceModel, {
    pathParams: { workspaceCode, modelCode },
  });
}

export function restoreWorkspaceModel(workspaceCode: string, modelCode: string) {
  return requestApi<WorkspaceModelMutationResult>(modelApiEndpoints.restoreWorkspaceModel, {
    pathParams: { workspaceCode, modelCode },
  });
}

import { requestApi } from "@/shared/api/httpClient";
import type { ModelSelectOption } from "@/domain/model/model";
export const modelApiEndpoints = {
  listProviderModelSelectOptions: "GET /api/v1/model-providers/select-options",
} as const;
export function fetchModelSelectOptions(workspaceCode = "", signal?: AbortSignal) {
  return requestApi<ModelSelectOption[]>(modelApiEndpoints.listProviderModelSelectOptions, {
    queryParams: { workspaceCode }, signal,
  });
}

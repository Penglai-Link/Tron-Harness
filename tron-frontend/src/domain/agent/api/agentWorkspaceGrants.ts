import { requestApi } from "@/shared/api/httpClient";

export function fetchAgentWorkspaceGrants(code: string, signal?: AbortSignal) {
  return requestApi<{ workspaceCodes: string[] }>("GET /api/v1/agents/registry/{code}/workspace-grants", { pathParams: { code }, signal });
}

export function saveAgentWorkspaceGrants(code: string, workspaceCodes: string[]) {
  return requestApi("PUT /api/v1/agents/registry/{code}/workspace-grants", { pathParams: { code }, body: { workspaceCodes } });
}

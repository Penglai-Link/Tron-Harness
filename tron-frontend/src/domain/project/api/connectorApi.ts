import { requestApi } from "@/shared/api/httpClient";

export interface ConnectorRecord { connectionCode: string; workspaceCode: string; providerId: string; name: string; endpoint: string }
export interface ConnectorRepository { id: string; name: string; fullName: string; webUrl: string; cloneUrl: string | null; sshUrl: string | null; defaultBranch: string | null }
export interface RepositorySearchResult { records: ConnectorRepository[]; nextCursor: string | null }

export function listConnections(workspaceCode: string, signal?: AbortSignal) {
  return requestApi<ConnectorRecord[]>("GET /api/v1/connector-connections?workspaceCode={workspaceCode}", { pathParams: { workspaceCode }, signal });
}
export function createConnection(input: { workspaceCode: string; providerId: string; name: string; endpoint: string; token: string }) {
  return requestApi<ConnectorRecord>("POST /api/v1/connector-connections", { body: input });
}
export function updateConnection(connectionCode: string, input: { workspaceCode: string; providerId: string; name: string; endpoint: string; token: string | null }) {
  return requestApi<ConnectorRecord>("PATCH /api/v1/connector-connections/{connectionCode}", { pathParams: { connectionCode }, body: input });
}
export function deleteConnection(connectionCode: string, workspaceCode: string) {
  return requestApi<void>("DELETE /api/v1/connector-connections/{connectionCode}", { pathParams: { connectionCode }, queryParams: { workspaceCode } });
}
export function checkConnection(connectionCode: string, workspaceCode: string) {
  return requestApi<void>("POST /api/v1/connector-connections/{connectionCode}/check", { pathParams: { connectionCode }, body: { workspaceCode } });
}
export function searchRepositories(connectionCode: string, workspaceCode: string, query: string, cursor: string | null = null) {
  return requestApi<RepositorySearchResult>("POST /api/v1/connector-connections/{connectionCode}/repositories/search", { pathParams: { connectionCode }, body: { workspaceCode, query, cursor } });
}

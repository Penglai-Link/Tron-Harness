import { requestApi } from "@/shared/api/httpClient";

export interface ConnectorRecord { connectionCode: string; providerId: string; name: string; endpoint: string }
export interface ConnectorRepository { id: string; name: string; fullName: string; webUrl: string; cloneUrl: string | null; sshUrl: string | null; defaultBranch: string | null }
export interface RepositorySearchResult { records: ConnectorRepository[]; nextCursor: string | null }

export function listConnections(signal?: AbortSignal) {
  return requestApi<ConnectorRecord[]>("GET /api/v1/connector-connections", { signal });
}
export function createConnection(input: { providerId: string; name: string; endpoint: string; token: string }) {
  return requestApi<ConnectorRecord>("POST /api/v1/connector-connections", { body: input });
}
export function updateConnection(connectionCode: string, input: { providerId: string; name: string; endpoint: string; token: string | null }) {
  return requestApi<ConnectorRecord>("PATCH /api/v1/connector-connections/{connectionCode}", { pathParams: { connectionCode }, body: input });
}
export function deleteConnection(connectionCode: string) {
  return requestApi<void>("DELETE /api/v1/connector-connections/{connectionCode}", { pathParams: { connectionCode } });
}
export function checkConnection(connectionCode: string) {
  return requestApi<void>("POST /api/v1/connector-connections/{connectionCode}/check", { pathParams: { connectionCode } });
}
export function searchRepositories(connectionCode: string, query: string, cursor: string | null = null) {
  return requestApi<RepositorySearchResult>("POST /api/v1/connector-connections/{connectionCode}/repositories/search", { pathParams: { connectionCode }, body: { query, cursor } });
}

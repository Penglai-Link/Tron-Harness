import { requestApi } from "@/shared/api/httpClient";

import type {
  ResourceApiPage, ResourceApiRecord,
} from "./resourceApiContract";
import type {
  ConnectionCommitApiResult,
  ConnectionDeleteApiRequest,
  ConnectionDeleteApiResult,
  ConnectionPageApiRequest,
  DataSourceCreateApiRequest,
  DataSourceUpdateApiRequest,
  ServerAccountCreateApiRequest,
  ServerCreateApiRequest,
  ServerCreateApiResult,
} from "./resourceConnectionApiContract";
import { toResourceKindCode } from "./resourceApiContract";

export const resourceConnectionApiEndpoints = {
  connectionDelete: "POST /api/v1/resource-connections/delete",
  connectionPage: "POST /api/v1/resource-connections/page",
  createDataSource: "POST /api/v1/resource-connections/data-sources",
  updateDataSource: "POST /api/v1/resource-connections/data-sources/update",
  createServer: "POST /api/v1/resource-connections/servers",
  createServerAccount: "POST /api/v1/resource-connections/server-accounts",
} as const;

export function pageConnections(request: ConnectionPageApiRequest) {
  return requestApi<ResourceApiPage<ResourceApiRecord>>(
    resourceConnectionApiEndpoints.connectionPage,
    { body: request },
  );
}

export function createDataSourceConnection(request: DataSourceCreateApiRequest) {
  return requestApi<ConnectionCommitApiResult>(
    resourceConnectionApiEndpoints.createDataSource,
    { body: { ...request, kind: toResourceKindCode(request.kind) } },
  );
}

export function updateDataSourceConnection(request: DataSourceUpdateApiRequest) {
  return requestApi<ConnectionCommitApiResult>(
    resourceConnectionApiEndpoints.updateDataSource,
    { body: request },
  );
}

export function createServerConnection(request: ServerCreateApiRequest) {
  return requestApi<ServerCreateApiResult>(
    resourceConnectionApiEndpoints.createServer,
    { body: request },
  );
}

export function createServerAccountConnection(request: ServerAccountCreateApiRequest) {
  return requestApi<ConnectionCommitApiResult>(
    resourceConnectionApiEndpoints.createServerAccount,
    { body: request },
  );
}

export function deleteConnection(request: ConnectionDeleteApiRequest) {
  return requestApi<ConnectionDeleteApiResult>(
    resourceConnectionApiEndpoints.connectionDelete,
    { body: request },
  );
}

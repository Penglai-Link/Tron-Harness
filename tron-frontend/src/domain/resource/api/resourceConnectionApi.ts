import { requestApi } from "@/shared/api/httpClient";

import type { ResourceActivationApiResult } from "./resourceApiContract";
import type {
  DataSourceCreateApiRequest,
  ServerAccountCreateApiRequest,
  ServerCreateApiRequest,
  ServerCreateApiResult,
} from "./resourceConnectionApiContract";

export const resourceConnectionApiEndpoints = {
  createDataSource: "POST /api/v1/resource-connections/data-sources",
  createServer: "POST /api/v1/resource-connections/servers",
  createServerAccount: "POST /api/v1/resource-connections/server-accounts",
} as const;

export function createDataSourceConnection(request: DataSourceCreateApiRequest) {
  return requestApi<ResourceActivationApiResult>(
    resourceConnectionApiEndpoints.createDataSource,
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
  return requestApi<ResourceActivationApiResult>(
    resourceConnectionApiEndpoints.createServerAccount,
    { body: request },
  );
}

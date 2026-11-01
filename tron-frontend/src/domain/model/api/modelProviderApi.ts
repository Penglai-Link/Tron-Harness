import { requestApi } from "@/shared/api/httpClient";

import type {
  ModelProviderCode,
  ProviderCatalogModelStatus,
  ProviderCatalogModelType,
  ProviderModelQuality,
} from "@/domain/model/model";

export type ProviderConnectionStatus =
  | "Disconnected"
  | "PendingAuthorization"
  | "Connected"
  | "Degraded"
  | "AuthRequired";

export type ProviderSyncStatus = "Running" | "Succeeded" | "Failed";
export type ProviderCatalogProtocol =
  | "CHAT_COMPLETIONS"
  | "RESPONSES"
  | "EMBEDDINGS";

export interface ModelProviderConnection {
  providerType?: string | null;
  providerCode: string;
  source?: "Plugin" | "PersonAi" | "Custom";
  personAiUserId: string | null;
  personAiUsername: string | null;
  displayName: string;
  status: ProviderConnectionStatus;
  serverBaseUrl: string;
  authorizedAt: number | null;
  authorizationExpiresAt: number | null;
  lastSyncAt: number | null;
  lastError: string | null;
  catalogModelCount: number;
  updatedAt: number;
}

export interface ProviderCatalogModel {
  dimension?: number | null;
  tags?: string[];
  providerCode: ModelProviderCode;
  catalogModelType: ProviderCatalogModelType;
  modelId: string | null;
  modelCode: string;
  modelName: string;
  protocols: ProviderCatalogProtocol[];
  upstreamVendorCode: string | null;
  upstreamVendorName: string | null;
  contextWindowTokens: number | null;
  qualityType: ProviderModelQuality | null;
  inputCoefficient: number | null;
  cacheHitCoefficient: number | null;
  outputCoefficient: number | null;
  status: ProviderCatalogModelStatus;
  firstSeenAt: number;
  lastSeenAt: number | null;
  updatedAt: number;
}

export interface ProviderAccountBalance {
  nickname?: string | null;
  username: string | null;
  vipLevel: number | null;
  remainingPoints: number | null;
}

export interface ProviderSyncRun {
  syncRunCode: string;
  providerCode: ModelProviderCode;
  status: ProviderSyncStatus;
  startedAt: number;
  completedAt: number | null;
  modelCount: number | null;
  errorSummary: string | null;
}

export interface ProviderAuthorizationSession {
  providerCode: ModelProviderCode;
  sessionId: string;
  loginUrl: string;
  status: string;
  expiresAt: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface ProviderAuthorizationPollResult {
  completed: boolean;
  session: ProviderAuthorizationSession;
  connection: ModelProviderConnection | null;
  textVerified: boolean | null;
  embeddingVerified: boolean | null;
  profileId: string | null;
}

export const modelProviderApiEndpoints = {
  list: "GET /api/v1/model-providers",
  detail: "GET /api/v1/model-providers/{providerCode}",
  catalog: "GET /api/v1/model-providers/{providerCode}/catalog",
  accountBalance: "GET /api/v1/model-providers/{providerCode}/account-balance",
  syncRuns: "GET /api/v1/model-providers/{providerCode}/sync-runs",
  authorize: "POST /api/v1/model-providers/{providerCode}/reauthorization-sessions",
  authorization: "GET /api/v1/model-providers/{providerCode}/reauthorization-sessions/{sessionId}",
  pollAuthorization: "POST /api/v1/model-providers/{providerCode}/reauthorization-sessions/{sessionId}/poll",
  profileAuthorize: "POST /api/v1/model-providers/{providerCode}/authorization-sessions",
  profilePollAuthorization: "POST /api/v1/model-providers/{providerCode}/authorization-sessions/{sessionId}/poll",
  sync: "POST /api/v1/model-providers/{providerCode}/sync",
  disconnect: "DELETE /api/v1/model-providers/{providerCode}",
} as const;

export function fetchModelProviderConnections(signal?: AbortSignal) {
  return requestApi<ModelProviderConnection[]>(modelProviderApiEndpoints.list, { signal });
}

export function fetchModelProviderConnection(providerCode: string, signal?: AbortSignal) {
  return requestApi<ModelProviderConnection>(modelProviderApiEndpoints.detail, {
    pathParams: { providerCode },
    signal,
  });
}

export function fetchProviderCatalog(providerCode: string, signal?: AbortSignal) {
  return requestApi<ProviderCatalogModel[]>(modelProviderApiEndpoints.catalog, {
    pathParams: { providerCode },
    signal,
  });
}

export function fetchProviderAccountBalance(
  providerCode: string,
  signal?: AbortSignal,
) {
  return requestApi<ProviderAccountBalance>(modelProviderApiEndpoints.accountBalance, {
    pathParams: { providerCode },
    signal,
  });
}

export function fetchProviderSyncRuns(providerCode: string, signal?: AbortSignal) {
  return requestApi<ProviderSyncRun[]>(modelProviderApiEndpoints.syncRuns, {
    pathParams: { providerCode },
    signal,
  });
}

export function createModelProviderAuthorization(
  providerCode: string,
  payload: { serverBaseUrl?: string | null } = {},
  purpose: "Provider" | "Profile" = "Provider",
) {
  return requestApi<ProviderAuthorizationSession>(purpose === "Profile" ? modelProviderApiEndpoints.profileAuthorize : modelProviderApiEndpoints.authorize, {
    body: payload,
    pathParams: { providerCode },
  });
}

export function fetchModelProviderAuthorization(
  providerCode: string,
  sessionId: string,
  signal?: AbortSignal,
) {
  return requestApi<ProviderAuthorizationSession>(modelProviderApiEndpoints.authorization, {
    pathParams: { providerCode, sessionId },
    signal,
  });
}

export function pollModelProviderAuthorization(
  providerCode: string,
  sessionId: string,
  purpose: "Provider" | "Profile" = "Provider",
) {
  return requestApi<ProviderAuthorizationPollResult>(purpose === "Profile" ? modelProviderApiEndpoints.profilePollAuthorization : modelProviderApiEndpoints.pollAuthorization, {
    body: {},
    pathParams: { providerCode, sessionId },
  });
}

export function syncModelProvider(providerCode: string) {
  return requestApi<ProviderSyncRun>(modelProviderApiEndpoints.sync, {
    body: {},
    pathParams: { providerCode },
  });
}

export function disconnectModelProvider(providerCode: string) {
  return requestApi<ModelProviderConnection>(modelProviderApiEndpoints.disconnect, {
    pathParams: { providerCode },
  });
}

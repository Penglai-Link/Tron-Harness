import { requestApi } from "@/shared/api/httpClient";

import type {
  ManagedOpenAppDetail,
  ManagedOpenAppSecretRevealResult,
  ManagedOpenAppSummary,
  OpenApiCapabilityCatalogGroup,
  OpenAppStatus,
} from "@/domain/open-app/model";

export const openAppApiEndpoints = {
  listPlatformOpenApps: "GET /api/v1/admin/platform/open-apps",
  createPlatformOpenApp: "POST /api/v1/admin/platform/open-apps",
  listPlatformOpenAppCatalog: "GET /api/v1/admin/platform/open-apps/catalog",
  getPlatformOpenApp: "GET /api/v1/admin/platform/open-apps/{openAppCode}",
  updatePlatformOpenApp: "PATCH /api/v1/admin/platform/open-apps/{openAppCode}",
  rotatePlatformOpenAppKey: "POST /api/v1/admin/platform/open-apps/{openAppCode}/rotate-key",
  replacePlatformOpenAppUserGrants: "PUT /api/v1/admin/platform/open-apps/{openAppCode}/user-grants",
  replacePlatformOpenAppCapabilityGrants: "PUT /api/v1/admin/platform/open-apps/{openAppCode}/capability-grants",
} as const;

export interface CreateOpenAppPayload {
  name: string;
  description?: string | null;
}

export interface UpdateOpenAppPayload {
  name: string;
  description?: string | null;
  status: OpenAppStatus;
}

export interface ReplaceOpenAppUserGrantsPayload {
  userCodes: string[];
}

export interface ReplaceOpenAppCapabilityGrantsPayload {
  grants: Array<{
    capabilityCode: string;
    operationCode: string;
  }>;
}

export function fetchPlatformOpenApps(signal?: AbortSignal) {
  return requestApi<ManagedOpenAppSummary[]>(openAppApiEndpoints.listPlatformOpenApps, { signal });
}

export function createPlatformOpenApp(payload: CreateOpenAppPayload) {
  return requestApi<ManagedOpenAppSecretRevealResult>(
    openAppApiEndpoints.createPlatformOpenApp,
    { body: payload },
  );
}

export function fetchPlatformOpenAppCatalog(signal?: AbortSignal) {
  return requestApi<OpenApiCapabilityCatalogGroup[]>(
    openAppApiEndpoints.listPlatformOpenAppCatalog,
    { signal },
  );
}

export function fetchPlatformOpenApp(openAppCode: string, signal?: AbortSignal) {
  return requestApi<ManagedOpenAppDetail>(openAppApiEndpoints.getPlatformOpenApp, {
    pathParams: { openAppCode },
    signal,
  });
}

export function updatePlatformOpenApp(openAppCode: string, payload: UpdateOpenAppPayload) {
  return requestApi<ManagedOpenAppDetail>(openAppApiEndpoints.updatePlatformOpenApp, {
    body: payload,
    pathParams: { openAppCode },
  });
}

export function rotatePlatformOpenAppKey(openAppCode: string) {
  return requestApi<ManagedOpenAppSecretRevealResult>(
    openAppApiEndpoints.rotatePlatformOpenAppKey,
    { pathParams: { openAppCode } },
  );
}

export function replacePlatformOpenAppUserGrants(
  openAppCode: string,
  payload: ReplaceOpenAppUserGrantsPayload,
) {
  return requestApi<ManagedOpenAppDetail>(
    openAppApiEndpoints.replacePlatformOpenAppUserGrants,
    { body: payload, pathParams: { openAppCode } },
  );
}

export function replacePlatformOpenAppCapabilityGrants(
  openAppCode: string,
  payload: ReplaceOpenAppCapabilityGrantsPayload,
) {
  return requestApi<ManagedOpenAppDetail>(
    openAppApiEndpoints.replacePlatformOpenAppCapabilityGrants,
    { body: payload, pathParams: { openAppCode } },
  );
}

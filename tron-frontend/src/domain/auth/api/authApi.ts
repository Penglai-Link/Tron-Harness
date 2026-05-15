import { AUTH_CALLBACK_PATH } from "@/domain/auth/constants";
import type {
  AuthProvidersPayload,
  AuthSession,
  InternalLoginPayload,
} from "@/domain/auth/model";
import {
  buildApiUrl,
  parseEndpoint,
  requestApi,
} from "@/shared/api/httpClient";

export const authApiEndpoints = {
  currentSession: "GET /api/v1/auth/session",
  listProviders: "GET /api/v1/auth/providers",
  login: "POST /api/v1/auth/login",
  logout: "POST /api/v1/auth/logout",
  oauth2Authorize: "GET /api/v1/auth/oauth2/authorize/{providerCode}",
  refresh: "POST /api/v1/auth/refresh",
} as const;

export function fetchAuthProviders(signal?: AbortSignal) {
  return requestApi<AuthProvidersPayload>(authApiEndpoints.listProviders, { signal });
}

export function fetchCurrentAuthSession(signal?: AbortSignal) {
  return requestApi<AuthSession>(authApiEndpoints.currentSession, { signal });
}

export function loginInternal(payload: InternalLoginPayload) {
  return requestApi<AuthSession>(authApiEndpoints.login, { body: payload });
}

export function refreshAuthSession() {
  return requestApi<AuthSession>(authApiEndpoints.refresh, { body: {} });
}

export function logoutAuthSession() {
  return requestApi<{ loggedOut: boolean }>(authApiEndpoints.logout, { body: {} });
}

export function buildOAuth2AuthorizeUrl(providerCode: string, redirectPath: string) {
  const { path } = parseEndpoint(authApiEndpoints.oauth2Authorize);
  return buildApiUrl(path, { providerCode }, { redirect: redirectPath });
}

export function buildOAuth2CallbackUrl(redirectPath: string) {
  return `${AUTH_CALLBACK_PATH}?redirect=${encodeURIComponent(redirectPath)}`;
}

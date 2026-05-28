import { AUTH_CALLBACK_PATH } from "@/domain/auth/constants";
import type {
  AuthGrant,
  AuthProvidersPayload,
  AuthSession,
  InternalLoginPayload,
} from "@/domain/auth/model";
import { getAuthDataSource } from "@/domain/auth/api/authDataSource";
import {
  authorizeMockOAuth2,
  fetchMockAuthProviders,
  fetchMockCurrentAuthSession,
  loginMockInternal,
  logoutMockAuthSession,
  refreshMockAuthSession,
} from "@/domain/auth/mock/data";
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
  if (getAuthDataSource() === "mock") {
    return fetchMockAuthProviders(signal);
  }

  return requestApi<AuthProvidersPayload>(authApiEndpoints.listProviders, {
    retryOnUnauthorized: false,
    signal,
  });
}

export function fetchCurrentAuthSession(signal?: AbortSignal) {
  if (getAuthDataSource() === "mock") {
    return fetchMockCurrentAuthSession(signal);
  }

  return requestApi<AuthSession>(authApiEndpoints.currentSession, { signal });
}

export function loginInternal(payload: InternalLoginPayload) {
  if (getAuthDataSource() === "mock") {
    return loginMockInternal(payload);
  }

  return requestApi<AuthGrant>(authApiEndpoints.login, {
    body: payload,
    retryOnUnauthorized: false,
  });
}

export function refreshAuthSession() {
  if (getAuthDataSource() === "mock") {
    return refreshMockAuthSession();
  }

  return requestApi<AuthGrant>(authApiEndpoints.refresh, {
    body: {},
    retryOnUnauthorized: false,
  });
}

export function logoutAuthSession() {
  if (getAuthDataSource() === "mock") {
    return logoutMockAuthSession();
  }

  return requestApi<{ loggedOut: boolean }>(authApiEndpoints.logout, {
    body: {},
    retryOnUnauthorized: false,
  });
}

export function buildOAuth2AuthorizeUrl(providerCode: string, redirectPath: string) {
  if (getAuthDataSource() === "mock") {
    authorizeMockOAuth2(providerCode);
    return buildOAuth2CallbackUrl(redirectPath);
  }

  const { path } = parseEndpoint(authApiEndpoints.oauth2Authorize);
  const authorizeUrl = new URL(
    buildApiUrl(path, { providerCode }, { redirect: redirectPath }),
  );

  // Keep the loopback host aligned with the registered GitHub callback host so the
  // backend-issued OAuth state cookie survives the full authorize -> callback round-trip.
  if (
    authorizeUrl.protocol === "http:"
    && (authorizeUrl.hostname === "localhost" || authorizeUrl.hostname === "0.0.0.0")
  ) {
    authorizeUrl.hostname = "127.0.0.1";
  }

  return authorizeUrl.toString();
}

export function buildOAuth2CallbackUrl(redirectPath: string) {
  return `${AUTH_CALLBACK_PATH}?redirect=${encodeURIComponent(redirectPath)}`;
}

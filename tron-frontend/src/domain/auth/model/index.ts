import {
  DEFAULT_AUTH_REDIRECT,
  LOGIN_PATH,
} from "@/domain/auth/constants";

export interface AuthProvider {
  code: string;
  name: string;
  type: "internal" | "oauth2";
}

export interface AuthProvidersPayload {
  providers: AuthProvider[];
}

export interface AuthenticatedUser {
  displayName: string;
  email: string;
  passwordResetRequired: boolean;
  roleCodes: string[];
  spaceCodes: string[];
  status: string;
  userCode: string;
  username: string;
}

export interface AuthSession {
  authMethod: "internal" | "oauth2";
  authenticated: boolean;
  provider: AuthProvider | null;
  user: AuthenticatedUser;
}

export interface InternalLoginPayload {
  login: string;
  password: string;
}

export function normalizeRedirectPath(candidate: string | null | undefined, fallback = DEFAULT_AUTH_REDIRECT) {
  if (!candidate || !candidate.startsWith("/") || candidate.startsWith("//")) {
    return fallback;
  }

  return candidate;
}

export function resolveAuthRedirect(search: string, fallback = DEFAULT_AUTH_REDIRECT) {
  return normalizeRedirectPath(new URLSearchParams(search).get("redirect"), fallback);
}

export function buildLoginUrl(redirectPath: string) {
  const safeRedirect = normalizeRedirectPath(redirectPath, DEFAULT_AUTH_REDIRECT);
  return `${LOGIN_PATH}?redirect=${encodeURIComponent(safeRedirect)}`;
}

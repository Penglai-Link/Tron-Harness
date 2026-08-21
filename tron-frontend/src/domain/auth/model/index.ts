import {
  DEFAULT_AUTH_REDIRECT,
  LOGIN_PATH,
} from "@/domain/auth/constants";

export interface AuthProvider {
  code: string;
  name: string;
  type: "internal" | "ldap" | "oauth2";
}

export interface AuthProvidersPayload {
  providers: AuthProvider[];
}

export interface AuthenticatedUser {
  avatarUrl: string | null;
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
  authMethod: "internal" | "ldap" | "oauth2";
  authenticated: boolean;
  provider: AuthProvider | null;
  user: AuthenticatedUser;
}

export interface AuthGrant {
  accessToken: string;
  tokenType: "Bearer";
  expiresIn: number;
  session: AuthSession;
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

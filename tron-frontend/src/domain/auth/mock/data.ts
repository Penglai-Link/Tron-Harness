import type {
  AuthGrant,
  AuthProvider,
  AuthProvidersPayload,
  AuthSession,
  AuthenticatedUser,
  InternalLoginPayload,
} from "@/domain/auth/model";
import { ApiError } from "@/shared/api/httpClient";

const MOCK_SESSION_STORAGE_KEY = "tron.auth.mock.session";
const MOCK_LOGGED_OUT_STORAGE_KEY = "tron.auth.mock.loggedOut";
const MOCK_ACCESS_TOKEN_TTL_SECONDS = 900;

const internalProvider: AuthProvider = {
  code: "internal",
  name: "Internal Account",
  type: "internal",
};

const devOAuth2Provider: AuthProvider = {
  code: "dev",
  name: "Dev OAuth2",
  type: "oauth2",
};

const ldapProvider: AuthProvider = {
  code: "ldap",
  name: "LDAP",
  type: "ldap",
};

const mockProviders: AuthProvider[] = [
  internalProvider,
  ldapProvider,
  devOAuth2Provider,
];

const defaultMockUser: AuthenticatedUser = {
  displayName: "Auth Operator",
  email: "auth.operator@tron.ai",
  passwordResetRequired: false,
  roleCodes: ["ADMIN", "DEVELOPER"],
  spaceCodes: ["PLATFORM", "DATA"],
  status: "Active",
  userCode: "USR-AUTH-0001",
  username: "auth.operator",
};

let inMemorySession: AuthSession | null | undefined;
let nextAccessTokenId = 1;

function createMockSession(options: {
  authMethod: AuthSession["authMethod"];
  provider: AuthProvider | null;
  user?: AuthenticatedUser;
}): AuthSession {
  return {
    authMethod: options.authMethod,
    authenticated: true,
    provider: options.provider,
    user: options.user ?? defaultMockUser,
  };
}

function createDefaultMockSession() {
  return createMockSession({
    authMethod: "internal",
    provider: null,
  });
}

function getStorage() {
  if (typeof window === "undefined") {
    return null;
  }

  return window.localStorage;
}

function readStoredSession() {
  const storage = getStorage();
  if (!storage) {
    return inMemorySession ?? createDefaultMockSession();
  }

  if (storage.getItem(MOCK_LOGGED_OUT_STORAGE_KEY) === "true") {
    return null;
  }

  const stored = storage.getItem(MOCK_SESSION_STORAGE_KEY);
  if (!stored) {
    return createDefaultMockSession();
  }

  try {
    return JSON.parse(stored) as AuthSession;
  } catch {
    storage.removeItem(MOCK_SESSION_STORAGE_KEY);
    return createDefaultMockSession();
  }
}

function writeSession(session: AuthSession | null) {
  inMemorySession = session;
  const storage = getStorage();
  if (!storage) {
    return;
  }

  if (!session) {
    storage.removeItem(MOCK_SESSION_STORAGE_KEY);
    storage.setItem(MOCK_LOGGED_OUT_STORAGE_KEY, "true");
    return;
  }

  storage.setItem(MOCK_SESSION_STORAGE_KEY, JSON.stringify(session));
  storage.removeItem(MOCK_LOGGED_OUT_STORAGE_KEY);
}

function getActiveSession() {
  if (inMemorySession === undefined) {
    inMemorySession = readStoredSession();
  }

  return inMemorySession;
}

function createUnauthorizedError(message = "authentication required") {
  return new ApiError(message, 401, {
    code: "AUTH_UNAUTHORIZED",
    data: {},
  });
}

function ensureActiveSession() {
  const session = getActiveSession();
  if (!session?.authenticated) {
    throw createUnauthorizedError();
  }

  return session;
}

function issueGrant(session: AuthSession): AuthGrant {
  writeSession(session);

  return {
    accessToken: `mock-auth-token-${nextAccessTokenId++}`,
    tokenType: "Bearer",
    expiresIn: MOCK_ACCESS_TOKEN_TTL_SECONDS,
    session,
  };
}

function resolveMockPromise<T>(value: T, signal?: AbortSignal) {
  if (signal?.aborted) {
    return Promise.reject(new DOMException("The operation was aborted.", "AbortError"));
  }

  return Promise.resolve(value);
}

export function fetchMockAuthProviders(signal?: AbortSignal) {
  return resolveMockPromise<AuthProvidersPayload>({
    providers: mockProviders,
  }, signal);
}

export function fetchMockCurrentAuthSession(signal?: AbortSignal) {
  if (signal?.aborted) {
    return Promise.reject(new DOMException("The operation was aborted.", "AbortError"));
  }

  try {
    return Promise.resolve(ensureActiveSession());
  } catch (error) {
    return Promise.reject(error);
  }
}

export function loginMockInternal(payload: InternalLoginPayload) {
  const login = payload.login.trim().toLowerCase();
  const acceptsDefaultOperator = login === "auth.operator" || login === "auth.operator@tron.ai";

  if (!acceptsDefaultOperator || payload.password !== "TronAuthPass123!") {
    return Promise.reject(new ApiError("invalid credentials", 401, {
      code: "AUTH_INVALID_CREDENTIALS",
      data: {},
    }));
  }

  return Promise.resolve(issueGrant(createDefaultMockSession()));
}

export function loginMockLdap(payload: InternalLoginPayload) {
  const login = payload.login.trim().toLowerCase();
  const acceptsDefaultOperator = login === "auth.operator" || login === "auth.operator@tron.ai";

  if (!acceptsDefaultOperator || payload.password !== "TronAuthPass123!") {
    return Promise.reject(new ApiError("invalid credentials", 401, {
      code: "AUTH_INVALID_CREDENTIALS",
      data: {},
    }));
  }

  return Promise.resolve(issueGrant(createMockSession({
    authMethod: "ldap",
    provider: ldapProvider,
  })));
}

export function refreshMockAuthSession() {
  try {
    return Promise.resolve(issueGrant(ensureActiveSession()));
  } catch (error) {
    return Promise.reject(error);
  }
}

export function logoutMockAuthSession() {
  writeSession(null);
  return Promise.resolve({ loggedOut: true });
}

export function authorizeMockOAuth2(providerCode: string) {
  const provider = mockProviders.find((candidate) => candidate.code === providerCode && candidate.type === "oauth2");
  if (!provider) {
    throw new ApiError("OAuth2 provider unavailable", 404, {
      code: "AUTH_PROVIDER_UNAVAILABLE",
      data: { providerCode },
    });
  }

  issueGrant(createMockSession({
    authMethod: "oauth2",
    provider,
  }));
}

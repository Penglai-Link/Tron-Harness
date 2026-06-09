import { create } from "zustand";

import {
  LOGIN_PATH,
} from "@/domain/auth/constants";
import {
  buildLoginUrl,
} from "@/domain/auth/model";
import {
  loginLdap,
  loginInternal,
  logoutAuthSession,
  refreshAuthSession,
} from "@/domain/auth/api";
import type {
  AuthGrant,
  AuthSession,
  InternalLoginPayload,
} from "@/domain/auth/model";
import {
  isUnauthorizedApiError,
  setApiAccessToken,
  setApiUnauthorizedHandler,
} from "@/shared/api/httpClient";

export type AuthStatus = "checking" | "authenticated" | "unauthenticated";

interface AuthState {
  session: AuthSession | null;
  status: AuthStatus;
  ensureSessionFresh: () => Promise<boolean>;
  initialize: () => Promise<void>;
  loginWithInternalAccount: (payload: InternalLoginPayload) => Promise<AuthSession>;
  loginWithLdapAccount: (payload: InternalLoginPayload) => Promise<AuthSession>;
  markUnauthenticated: () => void;
  logout: () => Promise<void>;
  recoverFromUnauthorized: () => Promise<boolean>;
}

type AuthStateSetter = (
  partial: Partial<AuthState> | ((state: AuthState) => Partial<AuthState>),
) => void;

type RefreshFailureMode = "preserve-session" | "unauthenticate";

const FOREGROUND_REFRESH_WINDOW_MS = 30_000;
const REFRESH_RETRY_DELAY_MS = 30_000;

let initializePromise: Promise<void> | null = null;
let refreshPromise: Promise<boolean> | null = null;
let sessionRefreshTimer: number | null = null;
let accessTokenExpiresAtMs: number | null = null;
let authEpoch = 0;

function redirectToLoginIfNeeded() {
  if (typeof window === "undefined") {
    return;
  }

  const { pathname, search } = window.location;
  if (pathname === LOGIN_PATH) {
    return;
  }

  window.location.replace(buildLoginUrl(`${pathname}${search}`));
}

function applyAuthenticatedSession(
  set: AuthStateSetter,
  grant: AuthGrant,
  epoch: number,
) {
  if (epoch !== authEpoch) {
    return false;
  }

  setApiAccessToken(grant.accessToken);
  accessTokenExpiresAtMs = Date.now() + Math.max(grant.expiresIn, 1) * 1000;
  set({
    session: grant.session,
    status: "authenticated",
  });

  scheduleSessionRefresh(set, grant.expiresIn, epoch);
  return true;
}

function clearSessionRefreshTimer() {
  if (sessionRefreshTimer === null) {
    return;
  }

  window.clearTimeout(sessionRefreshTimer);
  sessionRefreshTimer = null;
}

function transitionToUnauthenticated(set: AuthStateSetter, status: AuthStatus = "unauthenticated") {
  authEpoch += 1;
  clearSessionRefreshTimer();
  accessTokenExpiresAtMs = null;
  setApiAccessToken(null);
  set({
    session: null,
    status,
  });
}

function computeRefreshDelayMs(expiresInSeconds: number) {
  const ttlMs = Math.max(expiresInSeconds * 1000, 1_000);

  if (ttlMs <= 90_000) {
    return Math.max(Math.floor(ttlMs * 0.6), 500);
  }

  return Math.max(ttlMs - 60_000, 30_000);
}

function scheduleSessionRefresh(set: AuthStateSetter, expiresInSeconds: number, epoch: number) {
  clearSessionRefreshTimer();
  sessionRefreshTimer = window.setTimeout(() => {
    void refreshSession(set, {
      allowBootstrap: false,
      epoch,
      failureMode: "preserve-session",
    });
  }, computeRefreshDelayMs(expiresInSeconds));
}

function scheduleSessionRefreshRetry(set: AuthStateSetter, epoch: number) {
  clearSessionRefreshTimer();
  sessionRefreshTimer = window.setTimeout(() => {
    void refreshSession(set, {
      allowBootstrap: false,
      epoch,
      failureMode: "preserve-session",
    });
  }, REFRESH_RETRY_DELAY_MS);
}

async function refreshSession(
  set: AuthStateSetter,
  options: {
    allowBootstrap: boolean;
    epoch?: number;
    failureMode: RefreshFailureMode;
    markChecking?: boolean;
  },
) {
  const { status, session } = useAuthStore.getState();
  if (!options.allowBootstrap && (status !== "authenticated" || !session)) {
    return false;
  }

  if (refreshPromise) {
    return refreshPromise;
  }

  const epoch = options.epoch ?? authEpoch;
  if (options.markChecking) {
    set((state) => (state.status === "authenticated" ? state : { session: null, status: "checking" }));
  }

  const pendingRefresh = refreshAuthSession()
    .then((grant) => applyAuthenticatedSession(set, grant, epoch))
    .catch((error: unknown) => {
      if (epoch !== authEpoch) {
        return false;
      }

      if (options.failureMode === "unauthenticate" || isUnauthorizedApiError(error)) {
        transitionToUnauthenticated(set);
        return false;
      }

      scheduleSessionRefreshRetry(set, epoch);
      return false;
    })
    .finally(() => {
      if (refreshPromise === pendingRefresh) {
        refreshPromise = null;
      }
    });

  refreshPromise = pendingRefresh;
  return pendingRefresh;
}

export const useAuthStore = create<AuthState>((set) => ({
  session: null,
  status: "checking",
  ensureSessionFresh: async () => {
    const { status } = useAuthStore.getState();
    if (status !== "authenticated") {
      return false;
    }

    if (
      accessTokenExpiresAtMs === null
      || accessTokenExpiresAtMs - Date.now() > FOREGROUND_REFRESH_WINDOW_MS
    ) {
      return true;
    }

    return refreshSession(set, {
      allowBootstrap: false,
      failureMode: "preserve-session",
    });
  },
  initialize: async () => {
    if (initializePromise) {
      return initializePromise;
    }

    set((state) => (state.status === "authenticated" ? state : { session: null, status: "checking" }));

    initializePromise = refreshSession(set, {
      allowBootstrap: true,
      failureMode: "unauthenticate",
      markChecking: true,
    })
      .then(() => undefined)
      .finally(() => {
        initializePromise = null;
      });

    return initializePromise;
  },
  loginWithInternalAccount: async (payload) => {
    const loginEpoch = ++authEpoch;
    const grant = await loginInternal(payload);
    applyAuthenticatedSession(set, grant, loginEpoch);
    return grant.session;
  },
  loginWithLdapAccount: async (payload) => {
    const loginEpoch = ++authEpoch;
    const grant = await loginLdap(payload);
    applyAuthenticatedSession(set, grant, loginEpoch);
    return grant.session;
  },
  logout: async () => {
    authEpoch += 1;
    clearSessionRefreshTimer();
    set({
      status: "checking",
    });

    try {
      await logoutAuthSession();
    } finally {
      transitionToUnauthenticated(set);
    }
  },
  markUnauthenticated: () => {
    transitionToUnauthenticated(set);
  },
  recoverFromUnauthorized: async () => {
    return refreshSession(set, {
      allowBootstrap: false,
      failureMode: "preserve-session",
    });
  },
}));

setApiUnauthorizedHandler(() => {
  return useAuthStore.getState().recoverFromUnauthorized();
});

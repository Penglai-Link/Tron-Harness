import { create } from "zustand";

import {
  loginInternal,
  logoutAuthSession,
  refreshAuthSession,
} from "@/domain/auth/api";
import type {
  AuthSession,
  InternalLoginPayload,
} from "@/domain/auth/model";
import {
  setApiAccessToken,
  setApiUnauthorizedHandler,
} from "@/shared/api/httpClient";

export type AuthStatus = "checking" | "authenticated" | "unauthenticated";

interface AuthState {
  session: AuthSession | null;
  status: AuthStatus;
  initialize: () => Promise<void>;
  loginWithInternalAccount: (payload: InternalLoginPayload) => Promise<AuthSession>;
  markUnauthenticated: () => void;
  logout: () => Promise<void>;
}

let initializePromise: Promise<void> | null = null;

function applyAuthenticatedSession(
  set: (partial: Partial<AuthState>) => void,
  accessToken: string,
  session: AuthSession,
) {
  setApiAccessToken(accessToken);
  set({
    session,
    status: "authenticated",
  });
}

export const useAuthStore = create<AuthState>((set) => ({
  session: null,
  status: "checking",
  initialize: async () => {
    if (initializePromise) {
      return initializePromise;
    }

    set((state) => (state.status === "authenticated" ? state : { session: null, status: "checking" }));

    initializePromise = refreshAuthSession()
      .then((grant) => {
        applyAuthenticatedSession(set, grant.accessToken, grant.session);
      })
      .catch(() => {
        setApiAccessToken(null);
        set({
          session: null,
          status: "unauthenticated",
        });
      })
      .finally(() => {
        initializePromise = null;
      });

    return initializePromise;
  },
  loginWithInternalAccount: async (payload) => {
    const grant = await loginInternal(payload);
    applyAuthenticatedSession(set, grant.accessToken, grant.session);
    return grant.session;
  },
  logout: async () => {
    set({
      status: "checking",
    });

    try {
      await logoutAuthSession();
    } finally {
      setApiAccessToken(null);
      set({
        session: null,
        status: "unauthenticated",
      });
    }
  },
  markUnauthenticated: () => {
    setApiAccessToken(null);
    set({
      session: null,
      status: "unauthenticated",
    });
  },
}));

setApiUnauthorizedHandler(() => {
  useAuthStore.getState().markUnauthenticated();
});

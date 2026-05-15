import { create } from "zustand";

import {
  fetchCurrentAuthSession,
  loginInternal,
  logoutAuthSession,
} from "@/domain/auth/api";
import type {
  AuthSession,
  InternalLoginPayload,
} from "@/domain/auth/model";
import { setApiUnauthorizedHandler } from "@/shared/api/httpClient";

export type AuthStatus = "checking" | "authenticated" | "unauthenticated";

interface AuthState {
  session: AuthSession | null;
  status: AuthStatus;
  initialize: () => Promise<void>;
  loginWithInternalAccount: (payload: InternalLoginPayload) => Promise<AuthSession>;
  markUnauthenticated: () => void;
  logout: () => Promise<void>;
  setSession: (session: AuthSession) => void;
}

let initializePromise: Promise<void> | null = null;

export const useAuthStore = create<AuthState>((set) => ({
  session: null,
  status: "checking",
  initialize: async () => {
    if (initializePromise) {
      return initializePromise;
    }

    set((state) => (state.status === "authenticated" ? state : { session: null, status: "checking" }));

    initializePromise = fetchCurrentAuthSession()
      .then((session) => {
        set({
          session,
          status: "authenticated",
        });
      })
      .catch(() => {
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
    const session = await loginInternal(payload);
    set({
      session,
      status: "authenticated",
    });
    return session;
  },
  logout: async () => {
    set({
      status: "checking",
    });

    try {
      await logoutAuthSession();
    } finally {
      set({
        session: null,
        status: "unauthenticated",
      });
    }
  },
  markUnauthenticated: () => {
    set({
      session: null,
      status: "unauthenticated",
    });
  },
  setSession: (session) => {
    set({
      session,
      status: "authenticated",
    });
  },
}));

setApiUnauthorizedHandler(() => {
  useAuthStore.getState().markUnauthenticated();
});

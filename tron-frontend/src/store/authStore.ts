import { create } from "zustand";

import type {
  AuthSession,
  AuthenticatedUser,
} from "@/domain/auth/model";
import { fetchBootstrapProfileStatus } from "@/domain/profile/api";
import { useTabStore } from "@/store/tabStore";
import { setApiUnauthorizedHandler } from "@/shared/api/httpClient";

export type AuthStatus = "checking" | "authenticated" | "unauthenticated";

interface AuthState {
  session: AuthSession | null;
  status: AuthStatus;
  ensureSessionFresh: () => Promise<boolean>;
  initialize: () => Promise<void>;
  markUnauthenticated: () => void;
  logout: () => Promise<void>;
  recoverFromUnauthorized: () => Promise<boolean>;
  updateSessionUser: (patch: Partial<AuthenticatedUser>) => void;
}

let initializePromise: Promise<void> | null = null;

function sessionFromProfile(profile: {
  personAiUserId: string;
  personAiUsername: string;
}, avatarUrl: string | null): AuthSession {
  return {
    authMethod: "internal",
    authenticated: true,
    provider: { code: "person-ai", name: "PersonAI", type: "internal" },
    user: {
      avatarUrl,
      displayName: profile.personAiUsername,
      email: "",
      status: "active",
      userCode: profile.personAiUserId,
      username: profile.personAiUsername,
    },
  };
}

function transitionToUnauthenticated(set: (state: Partial<AuthState>) => void) {
  useTabStore.getState().resetAllTabs();
  set({ session: null, status: "unauthenticated" });
}

async function loadActiveProfile(
  set: (state: Partial<AuthState>) => void,
): Promise<boolean> {
  try {
    const bootstrap = await fetchBootstrapProfileStatus();
    const activeProfile = bootstrap.activeProfile;
    if (!activeProfile || activeProfile.status !== "ACTIVE") {
      transitionToUnauthenticated(set);
      return false;
    }
    set({
      session: sessionFromProfile(activeProfile, bootstrap.avatarUrl),
      status: "authenticated",
    });
    return true;
  } catch {
    transitionToUnauthenticated(set);
    return false;
  }
}

export const useAuthStore = create<AuthState>((set) => ({
  session: null,
  status: "checking",
  ensureSessionFresh: () => loadActiveProfile(set),
  initialize: async () => {
    if (initializePromise) return initializePromise;
    set((state) => state.status === "authenticated" ? state : { session: null, status: "checking" });
    initializePromise = loadActiveProfile(set).then(() => undefined).finally(() => {
      initializePromise = null;
    });
    return initializePromise;
  },
  markUnauthenticated: () => transitionToUnauthenticated(set),
  logout: async () => transitionToUnauthenticated(set),
  recoverFromUnauthorized: async (): Promise<boolean> => true,
  updateSessionUser: (patch) => {
    set((state) => state.session ? {
      session: {
        ...state.session,
        user: { ...state.session.user, ...patch },
      },
    } : state);
  },
}));

setApiUnauthorizedHandler(() => useAuthStore.getState().recoverFromUnauthorized());

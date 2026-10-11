import { create } from "zustand";

import type {
  AuthSession,
  AuthenticatedUser,
} from "@/domain/auth/model";
import { fetchActiveProfileToken, fetchBootstrapProfileStatus } from "@/domain/profile/api";
import { useTabStore } from "@/store/tabStore";
import { DEFAULT_TIME_ZONE, useTimeZoneStore } from "@/shared/lib/userTimeZone";
import { isUnauthorizedApiError, setApiAccessToken, setApiUnauthorizedHandler } from "@/shared/api/httpClient";

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
let sessionLoadGeneration = 0;
let recoveryPromise: Promise<boolean> | null = null;

function sessionFromProfile(profile: {
  personAiUserId: string;
  personAiUsername: string;
}, avatarUrl: string | null, account: {
  vipLevel: number | null;
  remainingPoints: number | null;
} | null): AuthSession {
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
      vipLevel: account?.vipLevel ?? null,
      remainingPoints: account?.remainingPoints ?? null,
    },
  };
}

function transitionToUnauthenticated(set: (state: Partial<AuthState>) => void) {
  sessionLoadGeneration += 1;
  setApiAccessToken(null);
  useTimeZoneStore.getState().setTimeZone(DEFAULT_TIME_ZONE);
  useTabStore.getState().resetAllTabs();
  set({ session: null, status: "unauthenticated" });
}

async function loadActiveProfile(
  set: (state: Partial<AuthState>) => void,
  generation: number,
): Promise<boolean> {
  try {
    const bootstrap = await fetchBootstrapProfileStatus();
    if (generation !== sessionLoadGeneration) {
      return useAuthStore.getState().status === "authenticated";
    }
    const activeProfile = bootstrap.activeProfile;
    if (!activeProfile || activeProfile.status !== "ACTIVE") {
      transitionToUnauthenticated(set);
      return false;
    }
    const token = await fetchActiveProfileToken();
    if (generation !== sessionLoadGeneration) {
      return useAuthStore.getState().status === "authenticated";
    }
    setApiAccessToken(token.token);
    useTimeZoneStore.getState().setTimeZone(bootstrap.timeZone ?? DEFAULT_TIME_ZONE);
    set({
      session: sessionFromProfile(activeProfile, bootstrap.avatarUrl, bootstrap.account),
      status: "authenticated",
    });
    return true;
  } catch (error) {
    if (generation === sessionLoadGeneration) {
      // A refresh outage does not invalidate an existing session.
      if (!isUnauthorizedApiError(error) && useAuthStore.getState().status === "authenticated") {
        return false;
      }
      transitionToUnauthenticated(set);
      return false;
    }
    return useAuthStore.getState().status === "authenticated";
  }
}

export const useAuthStore = create<AuthState>((set) => ({
  session: null,
  status: "checking",
  ensureSessionFresh: () => loadActiveProfile(set, ++sessionLoadGeneration),
  initialize: async () => {
    if (initializePromise) return initializePromise;
    set((state) => state.status === "authenticated" ? state : { session: null, status: "checking" });
    initializePromise = loadActiveProfile(set, ++sessionLoadGeneration).then(() => undefined).finally(() => {
      initializePromise = null;
    });
    return initializePromise;
  },
  markUnauthenticated: () => transitionToUnauthenticated(set),
  logout: async () => transitionToUnauthenticated(set),
  recoverFromUnauthorized: (): Promise<boolean> => {
    // All concurrent 401s share one refresh; replay only after a real token load.
    if (recoveryPromise) return recoveryPromise;
    recoveryPromise = loadActiveProfile(set, ++sessionLoadGeneration).finally(() => {
      recoveryPromise = null;
    });
    return recoveryPromise;
  },
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

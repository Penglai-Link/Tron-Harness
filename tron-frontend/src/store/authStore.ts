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
  ensureSessionFresh: (signal?: AbortSignal) => Promise<boolean>;
  initialize: () => Promise<void>;
  markUnauthenticated: () => void;
  logout: () => Promise<void>;
  recoverFromUnauthorized: () => Promise<boolean>;
  updateSessionUser: (patch: Partial<AuthenticatedUser>) => void;
  refreshAccountSnapshot: () => Promise<boolean>;
}

let initializePromise: Promise<void> | null = null;
let sessionLoadGeneration = 0;
let recoveryPromise: Promise<boolean> | null = null;
let accountRefresh: { userCode: string; promise: Promise<boolean> } | null = null;
let accountStateRevision = 0;

function sessionFromProfile(profile: {
  personAiUserId: string;
  personAiUsername: string;
}, avatarUrl: string | null, account: {
  nickname?: string | null;
  vipLevel: number | null;
  remainingPoints: number | null;
  todayCheckedIn?: boolean | null;
} | null): AuthSession {
  return {
    authMethod: "internal",
    authenticated: true,
    provider: { code: "person-ai", name: "PersonAI", type: "internal" },
    user: {
      avatarUrl,
      displayName: account?.nickname?.trim() || profile.personAiUsername,
      email: "",
      status: "active",
      userCode: profile.personAiUserId,
      username: profile.personAiUsername,
      vipLevel: account?.vipLevel ?? null,
      remainingPoints: account?.remainingPoints ?? null,
      todayCheckedIn: account?.todayCheckedIn ?? null,
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
  signal?: AbortSignal,
): Promise<boolean> {
  const accountRevision = accountStateRevision;
  try {
    const bootstrap = await fetchBootstrapProfileStatus(signal);
    if (signal?.aborted) return false;
    if (generation !== sessionLoadGeneration) {
      return useAuthStore.getState().status === "authenticated";
    }
    const activeProfile = bootstrap.activeProfile;
    if (!activeProfile || activeProfile.status !== "ACTIVE") {
      transitionToUnauthenticated(set);
      return false;
    }
    const token = await fetchActiveProfileToken(signal);
    if (signal?.aborted) return false;
    if (generation !== sessionLoadGeneration) {
      return useAuthStore.getState().status === "authenticated";
    }
    setApiAccessToken(token.token);
    useTimeZoneStore.getState().setTimeZone(bootstrap.timeZone ?? DEFAULT_TIME_ZONE);
    const session = sessionFromProfile(activeProfile, bootstrap.avatarUrl, bootstrap.account);
    const current = useAuthStore.getState().session;
    if (accountRevision !== accountStateRevision && current?.user.userCode === session.user.userCode) {
      // A sign-in receipt or a newer snapshot arrived while credentials loaded.
      // Refresh the token without rolling visible account values backwards.
      session.user = { ...current.user, username: session.user.username };
    }
    set({
      session,
      status: "authenticated",
    });
    return true;
  } catch (error) {
    if (signal?.aborted) return false;
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
  refreshAccountSnapshot: (): Promise<boolean> => {
    const session = useAuthStore.getState().session;
    if (!session) return Promise.resolve(false);
    const userCode = session.user.userCode;
    if (accountRefresh && accountRefresh.userCode === userCode) return accountRefresh.promise;
    const generation = sessionLoadGeneration;
    const revision = accountStateRevision;
    const promise = fetchBootstrapProfileStatus().then((profile) => {
      const activeProfile = profile.activeProfile;
      if (generation !== sessionLoadGeneration || revision !== accountStateRevision || useAuthStore.getState().session?.user.userCode !== userCode
          || !activeProfile || activeProfile.personAiUserId !== userCode) return false;
      // Only merge a real cached snapshot. An unavailable read never clears the
      // account already shown by either the sidebar or its menu.
      if (profile.account) useAuthStore.getState().updateSessionUser({
        displayName: profile.account.nickname?.trim() || activeProfile.personAiUsername,
        vipLevel: profile.account.vipLevel,
        remainingPoints: profile.account.remainingPoints,
        todayCheckedIn: profile.account.todayCheckedIn ?? null,
        avatarUrl: profile.avatarUrl,
      });
      return true;
    }).catch(() => false).finally(() => {
      if (accountRefresh?.promise === promise) accountRefresh = null;
    });
    accountRefresh = { userCode, promise };
    return promise;
  },
  ensureSessionFresh: (signal) => loadActiveProfile(set, ++sessionLoadGeneration, signal),
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
    accountStateRevision += 1;
    set((state) => state.session ? {
      session: {
        ...state.session,
        user: { ...state.session.user, ...patch },
      },
    } : state);
  },
}));

setApiUnauthorizedHandler(() => useAuthStore.getState().recoverFromUnauthorized());

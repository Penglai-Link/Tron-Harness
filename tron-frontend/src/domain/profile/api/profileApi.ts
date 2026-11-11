import { requestApi } from "@/shared/api/httpClient";
import {
  createModelProviderAuthorization,
  pollModelProviderAuthorization,
} from "@/domain/model/api/modelProviderApi";

export interface PersonAiProfile {
  profileId: string;
  personAiUserId: string;
  personAiUsername: string;
  status: "ACTIVE" | "INACTIVE" | string;
  createdAt: string;
  lastActivatedAt: string | null;
  avatarUrl: string | null;
}

export interface PersonAiAccountSummary {
  nickname?: string | null;
  vipLevel: number | null;
  remainingPoints: number | null;
  todayCheckedIn?: boolean | null;
  todayRewardPoints?: number | null;
  updatedAt?: number | null;
}

export interface BootstrapProfileStatus {
  timeZone?: string;
  hasProfiles: boolean;
  activeProfileId: string | null;
  activeProfile: PersonAiProfile | null;
  avatarUrl: string | null;
  account: PersonAiAccountSummary | null;
}

export interface PersonAiAuthorizationSession {
  providerCode: string;
  sessionId: string;
  loginUrl: string;
  status: string;
  expiresAt: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface PersonAiAuthorizationPollResult {
  completed: boolean;
  session: PersonAiAuthorizationSession;
  connection: {
    personAiUserId: string | null;
    personAiUsername: string | null;
  } | null;
  profileId: string | null;
}

export const profileApiEndpoints = {
  bootstrapStatus: "GET /api/v1/bootstrap/status",
  list: "GET /api/v1/profiles",
  pending: "POST /api/v1/profiles/pending",
  activate: "POST /api/v1/profiles/{profileId}/activate",
} as const;

export function fetchBootstrapProfileStatus(signal?: AbortSignal) {
  return requestApi<BootstrapProfileStatus>(profileApiEndpoints.bootstrapStatus, {
    retryOnUnauthorized: false,
    signal,
  });
}

export function fetchActiveProfileToken(signal?: AbortSignal) {
  return requestApi<{ token: string }>("GET /api/v1/profiles/active/token", {
    retryOnUnauthorized: false,
    signal,
  });
}

export function updateProfileTimeZone(timeZone: string) {
  return requestApi<{ timeZone: string }>("PATCH /api/v1/profiles/active/preferences", {
    body: { timeZone },
  });
}

export function fetchProfiles(signal?: AbortSignal) {
  return requestApi<PersonAiProfile[]>(profileApiEndpoints.list, {
    retryOnUnauthorized: false,
    signal,
  });
}

export function createPendingProfile() {
  return requestApi<PersonAiProfile>(profileApiEndpoints.pending, {
    retryOnUnauthorized: false,
    body: {},
  });
}

export function createPersonAiAuthorization(signal?: AbortSignal) {
  return createModelProviderAuthorization("PersonAi", {}, "Profile", signal);
}

export function pollPersonAiAuthorization(
  sessionId: string,
  signal?: AbortSignal,
) {
  return pollModelProviderAuthorization("PersonAi", sessionId, "Profile", signal);
}

export function registerProfile(personAiUserId: string, personAiUsername: string) {
  return requestApi<PersonAiProfile>("POST /api/v1/profiles", {
    retryOnUnauthorized: false,
    body: { personAiUserId, personAiUsername },
  });
}

export function activateProfile(profileId: string) {
  return requestApi<PersonAiProfile>(profileApiEndpoints.activate, {
    retryOnUnauthorized: false,
    pathParams: { profileId },
  });
}

export function removeProfile(profileId: string) {
  return requestApi<Record<string, never>>("DELETE /api/v1/profiles/{profileId}", {
    retryOnUnauthorized: false,
    pathParams: { profileId },
  });
}

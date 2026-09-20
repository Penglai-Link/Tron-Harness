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
}

export interface BootstrapProfileStatus {
  hasProfiles: boolean;
  activeProfileId: string | null;
  activeProfile: PersonAiProfile | null;
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
  textVerified: boolean | null;
  embeddingVerified: boolean | null;
  profileId: string | null;
  restartRequired: boolean;
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

export function createPersonAiAuthorization() {
  return createModelProviderAuthorization("PersonAi");
}

export function pollPersonAiAuthorization(
  sessionId: string,
) {
  return pollModelProviderAuthorization("PersonAi", sessionId);
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

import { requestApi } from "@/shared/api/httpClient";

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
}

export const profileApiEndpoints = {
  bootstrapStatus: "GET /api/v1/bootstrap/status",
  list: "GET /api/v1/profiles",
  register: "POST /api/v1/profiles",
  pending: "POST /api/v1/profiles/pending",
  activate: "POST /api/v1/profiles/{profileId}/activate",
  personAiAuthorize: "POST /api/v1/profiles/person-ai/authorization-sessions",
  personAiPoll: "POST /api/v1/profiles/person-ai/authorization-sessions/{sessionId}/poll",
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

export function registerProfile(personAiUserId: string, personAiUsername: string) {
  return requestApi<PersonAiProfile>(profileApiEndpoints.register, {
    retryOnUnauthorized: false,
    body: { personAiUserId, personAiUsername },
  });
}

export function createPendingProfile() {
  return requestApi<PersonAiProfile>(profileApiEndpoints.pending, {
    retryOnUnauthorized: false,
    body: {},
  });
}

export function createPersonAiAuthorization() {
  return requestApi<PersonAiAuthorizationSession>(profileApiEndpoints.personAiAuthorize, {
    retryOnUnauthorized: false,
    body: {},
  });
}

export function pollPersonAiAuthorization(
  sessionId: string,
) {
  return requestApi<PersonAiAuthorizationPollResult>(profileApiEndpoints.personAiPoll, {
    retryOnUnauthorized: false,
    pathParams: { sessionId },
    body: {},
  });
}

export function activateProfile(profileId: string) {
  return requestApi<PersonAiProfile>(profileApiEndpoints.activate, {
    retryOnUnauthorized: false,
    pathParams: { profileId },
  });
}

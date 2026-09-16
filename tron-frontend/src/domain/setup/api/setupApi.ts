import { requestApi } from "@/shared/api/httpClient";
import type { ModelMutationPayload } from "@/domain/model/api/modelApi";

export interface SetupStatus {
  required: boolean;
  initialized: boolean;
  initializedAt: string | null;
  initializedBy: string | null;
  ready: boolean;
  admin: {
    userCode: string;
    username: string;
    displayName: string;
    passwordResetRequired: boolean;
  };
  crypto: {
    configured: boolean;
    algorithm: string;
    keyLength: number;
    fingerprint: string | null;
  };
  workspace: {
    configured: boolean;
    activeCount: number;
  };
  models: {
    configured: boolean;
    requiredTypes: string[];
  };
}

export interface SetupCompletion {
  initialized: boolean;
  initializedAt: string | null;
  initializedBy: string | null;
}

export interface SetupAdminActivation {
  activated: boolean;
}

export interface SetupCompletionPayload {
  newPassword: string;
  displayName: string;
  encryptionKeyBase64: string;
  workspaceName: string;
  workspaceDescription?: string | null;
  modelAccess: "Manual" | "PersonAi";
  models?: {
    text: ModelMutationPayload;
    embedding: ModelMutationPayload;
    rerank?: ModelMutationPayload;
  };
}

export const setupApiEndpoints = {
  status: "GET /api/v1/setup/status",
  session: "POST /api/v1/setup/session",
  activateAdmin: "POST /api/v1/setup/activate-admin",
  complete: "POST /api/v1/setup/complete",
} as const;

export function fetchSetupStatus(signal?: AbortSignal) {
  return requestApi<SetupStatus>(setupApiEndpoints.status, {
    retryOnUnauthorized: false,
    signal,
  });
}

export function completeSetup(payload: SetupCompletionPayload) {
  return requestApi<SetupCompletion>(setupApiEndpoints.complete, {
    body: payload,
  });
}

export function createSetupSession(newPassword: string) {
  return requestApi<import("@/domain/auth/model").AuthGrant>(setupApiEndpoints.session, {
    body: { newPassword },
    retryOnUnauthorized: false,
  });
}

export function activateSetupAdmin(newPassword: string) {
  return requestApi<SetupAdminActivation>(setupApiEndpoints.activateAdmin, {
    body: { newPassword },
  });
}

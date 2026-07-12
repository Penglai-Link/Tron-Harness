import { getAuthDataSource } from "@/domain/auth/api/authDataSource";
import type {
  AccountProfile,
  ChangeAccountPasswordPayload,
  ChangeAccountPasswordResult,
  UpdateAccountProfilePayload,
} from "@/domain/account/model";
import {
  changeMockAccountPassword,
  fetchMockAccountProfile,
  updateMockAccountProfile,
} from "@/domain/account/mock";
import { requestApi } from "@/shared/api/httpClient";

export const accountApiEndpoints = {
  getProfile: "GET /api/v1/account/get-profile",
  updateProfile: "POST /api/v1/account/update-profile",
  changePassword: "POST /api/v1/account/change-password",
} as const;

export function fetchAccountProfile(signal?: AbortSignal) {
  if (getAuthDataSource() === "mock") {
    return fetchMockAccountProfile();
  }

  return requestApi<AccountProfile>(accountApiEndpoints.getProfile, { signal });
}

export function updateAccountProfile(payload: UpdateAccountProfilePayload) {
  if (getAuthDataSource() === "mock") {
    return updateMockAccountProfile(payload);
  }

  return requestApi<AccountProfile>(accountApiEndpoints.updateProfile, {
    body: payload,
  });
}

export function changeAccountPassword(payload: ChangeAccountPasswordPayload) {
  if (getAuthDataSource() === "mock") {
    return changeMockAccountPassword(payload);
  }

  return requestApi<ChangeAccountPasswordResult>(accountApiEndpoints.changePassword, {
    body: payload,
  });
}

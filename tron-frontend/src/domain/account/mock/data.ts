import type {
  AccountProfile,
  ChangeAccountPasswordPayload,
  ChangeAccountPasswordResult,
  UpdateAccountProfilePayload,
} from "@/domain/account/model";

let mockAccountProfile: AccountProfile = {
  avatarUrl: null,
  displayName: "Bootstrap Admin",
  email: "admin@tron.ai",
  firstName: "Bootstrap",
  lastName: "Admin",
  locale: "en-US",
  passwordChangeAllowed: true,
  passwordResetRequired: false,
  phone: null,
  roleCodes: ["ADMIN"],
  sourceType: "Internal",
  spaceCodes: ["PLATFORM"],
  status: "Active",
  timezone: "Asia/Shanghai",
  updatedAt: "2026-07-11T00:00:00Z",
  userCode: "USR-BOOTSTRAP-ADMIN",
  username: "admin",
};

export function fetchMockAccountProfile() {
  return Promise.resolve({ ...mockAccountProfile });
}

export function updateMockAccountProfile(payload: UpdateAccountProfilePayload) {
  mockAccountProfile = {
    ...mockAccountProfile,
    ...payload,
    updatedAt: new Date().toISOString(),
  };
  return Promise.resolve({ ...mockAccountProfile });
}

export function changeMockAccountPassword(
  _payload: ChangeAccountPasswordPayload,
): Promise<ChangeAccountPasswordResult> {
  mockAccountProfile = {
    ...mockAccountProfile,
    passwordResetRequired: false,
    updatedAt: new Date().toISOString(),
  };
  return Promise.resolve({
    changed: true,
    passwordResetRequired: false,
  });
}

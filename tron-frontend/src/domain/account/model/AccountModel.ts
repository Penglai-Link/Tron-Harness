export type AccountSourceType = "Internal" | "GitHub" | "LDAP";

export interface AccountPhone {
  countryCode: string;
  number: string;
}

export interface AccountProfile {
  avatarUrl: string | null;
  displayName: string;
  email: string;
  firstName: string;
  lastName: string;
  locale: string;
  passwordChangeAllowed: boolean;
  passwordResetRequired: boolean;
  phone: AccountPhone | null;
  roleCodes: string[];
  sourceType: AccountSourceType;
  spaceCodes: string[];
  status: string;
  timezone: string;
  updatedAt: string;
  userCode: string;
  username: string;
}

export interface UpdateAccountProfilePayload {
  avatarUrl?: string | null;
  displayName?: string;
  firstName?: string;
  lastName?: string;
  locale?: string;
  phone?: AccountPhone | null;
  timezone?: string;
}

export interface ChangeAccountPasswordPayload {
  currentPassword: string;
  newPassword: string;
}

export interface ChangeAccountPasswordResult {
  changed: boolean;
  passwordResetRequired: boolean;
}

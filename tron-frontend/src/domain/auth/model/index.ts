export interface AuthProvider {
  code: string;
  name: string;
  type: "internal";
}

export interface AuthenticatedUser {
  avatarUrl: string | null;
  displayName: string;
  email: string;
  status: string;
  userCode: string;
  username: string;
  vipLevel: number | null;
  remainingPoints: number | null;
  todayCheckedIn?: boolean | null;
}

export interface AuthSession {
  authMethod: "internal";
  authenticated: boolean;
  provider: AuthProvider | null;
  user: AuthenticatedUser;
}

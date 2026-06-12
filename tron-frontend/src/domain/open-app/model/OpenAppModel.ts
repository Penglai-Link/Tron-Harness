export type OpenAppStatus = "Enabled" | "Disabled" | "Archived";

export interface OpenApiCapabilityCatalogOperation {
  operationCode: string;
  operationName: string;
  description: string;
  routes: string[];
}

export interface OpenApiCapabilityCatalogGroup {
  capabilityCode: string;
  capabilityName: string;
  description: string;
  operations: OpenApiCapabilityCatalogOperation[];
}

export interface ManagedOpenAppSummary {
  openAppCode: string;
  name: string;
  description: string | null;
  status: OpenAppStatus;
  appKeyFingerprint: string;
  createdAt: string;
  updatedAt: string;
  lastRotatedAt: string | null;
  userGrantCount: number;
  capabilityGrantCount: number;
}

export interface ManagedOpenAppUserGrant {
  userCode: string;
}

export interface ManagedOpenAppCapabilityGrant {
  capabilityCode: string;
  operationCode: string;
}

export interface ManagedOpenAppDetail {
  openAppCode: string;
  name: string;
  description: string | null;
  status: OpenAppStatus;
  appKeyFingerprint: string;
  createdAt: string;
  updatedAt: string;
  lastRotatedAt: string | null;
  userGrants: ManagedOpenAppUserGrant[];
  capabilityGrants: ManagedOpenAppCapabilityGrant[];
}

export interface ManagedOpenAppSecretReveal {
  openAppCode: string;
  appKey: string;
  appKeyFingerprint: string;
}

export interface ManagedOpenAppSecretRevealResult {
  openApp: ManagedOpenAppDetail;
  secretReveal: ManagedOpenAppSecretReveal;
}

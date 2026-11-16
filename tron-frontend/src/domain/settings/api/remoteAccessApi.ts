import { requestApi } from "@/shared/api/httpClient";

export interface RemoteAccessStatus {
  active: boolean;
  allowInsecureHttp: boolean;
  baseUrl: string;
  bindAddress: string;
  enabled: boolean;
  lanAddress: string | null;
  machineName: string;
  pairingUri: string;
  port: number;
  publicBaseUrl: string;
  restartRequired: boolean;
  token: string;
  tokenConfigured: boolean;
  productCode: string;
  productName: string;
  nodeId: string;
  jwtTtlSeconds: number;
}

export interface UpdateRemoteAccessInput {
  enabled: boolean;
  allowInsecureHttp?: boolean;
  machineName?: string;
  publicBaseUrl?: string;
  productCode?: string;
  productName?: string;
}

export const remoteAccessApiEndpoints = {
  status: "GET /api/v1/remote/status",
  update: "PUT /api/v1/remote/settings",
  rotateToken: "POST /api/v1/remote/token/rotate",
} as const;

export function getRemoteAccessStatus(signal?: AbortSignal) {
  return requestApi<RemoteAccessStatus>(remoteAccessApiEndpoints.status, { signal });
}

export function updateRemoteAccess(input: UpdateRemoteAccessInput) {
  return requestApi<RemoteAccessStatus>(remoteAccessApiEndpoints.update, { body: input });
}

export function rotateRemoteAccessToken() {
  return requestApi<RemoteAccessStatus>(remoteAccessApiEndpoints.rotateToken, { body: {} });
}
export interface ServicePairing {
  pairingUri: string | null;
  pairingTicketExpiresAt: number | null;
  transport: "public" | "lan";
  nodeKeyFingerprint: string;
  baseUrl: string;
  lanBaseUrl?: string;
  publicBaseUrl: string;
}

export interface ServiceDevice {
  deviceId: string;
  name: string;
  platform: string;
  createdAtMs: number;
  lastSeenAtMs: number;
  scopes: string[];
  revoked: boolean;
}

export interface ServiceStatus {
  enabled: boolean;
  active: boolean;
  transport: "public" | "lan";
  baseUrl: string;
  publicBaseUrl: string;
  nodeKeyFingerprint: string;
  pairedDeviceCount: number;
  lanBaseUrl?: string;
  pairingTicketExpiresAt: number | null;
  pairingUri: string | null;
  restartRequired: boolean;
}

export async function getServicePairing(signal?: AbortSignal) {
  return requestApi<ServicePairing>("GET /api/v1/remote/service/pairing", { signal });
}

export async function refreshServicePairing() {
  return requestApi<ServicePairing>("POST /api/v1/remote/service/pairing/refresh", { body: {} });
}

export async function getServiceDevices(signal?: AbortSignal) {
  return requestApi<ServiceDevice[]>("GET /api/v1/remote/service/devices", { signal });
}

export async function revokeServiceDevice(deviceId: string) {
  return requestApi<ServiceStatus>(`DELETE /api/v1/remote/service/devices/${encodeURIComponent(deviceId)}`);
}

export async function deleteServiceDevice(deviceId: string) {
  return requestApi<ServiceStatus>(`DELETE /api/v1/remote/service/devices/${encodeURIComponent(deviceId)}/record`);
}

export async function getServiceStatus(signal?: AbortSignal) {
  return requestApi<ServiceStatus>("GET /api/v1/remote/service/status", { signal });
}

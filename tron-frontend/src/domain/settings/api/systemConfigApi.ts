import { requestApi } from "@/shared/api/httpClient";
import type { SystemConfigChange, SystemConfigDocument, SystemConfigUpdateResult } from "@/domain/settings/model/SystemConfig";

export const systemConfigApiEndpoints = {
  get: "GET /api/v1/system/config",
  update: "PUT /api/v1/system/config",
} as const;

export function getSystemConfig(signal?: AbortSignal) {
  return requestApi<SystemConfigDocument>(systemConfigApiEndpoints.get, { signal });
}

export function updateSystemConfig(revision: string, changes: SystemConfigChange[], confirmMigration: boolean) {
  return requestApi<SystemConfigUpdateResult>(systemConfigApiEndpoints.update, {
    body: { changes, confirmMigration, revision },
  });
}

import { requestApi } from "@/shared/api/httpClient";
import type { ToolPermission, ToolPermissionRecord, ToolPermissionsPayload, ToolRiskLevel } from "../model/ToolPermissionModel";

type WirePermission = Capitalize<ToolPermission>;
type WireRisk = Capitalize<ToolRiskLevel>;
interface WirePayload { permissions: { toolCode: string; permission: WirePermission; confirmLevel: WireRisk }[] }
function decode(payload: WirePayload): ToolPermissionRecord[] {
  return payload.permissions.map((item) => ({ ...item, permission: item.permission.toLowerCase() as ToolPermission, confirmLevel: item.confirmLevel.toLowerCase() as ToolRiskLevel }));
}
function capitalize<T extends string>(value: T): Capitalize<T> {
  return (value[0].toUpperCase() + value.slice(1)) as Capitalize<T>;
}
export async function fetchToolPermissions(signal?: AbortSignal) {
  return decode(await requestApi<WirePayload>("GET /api/v1/tools/permissions", { signal }));
}
export async function updateToolPermissions(payload: ToolPermissionsPayload) {
  return decode(await requestApi<WirePayload>("PUT /api/v1/tools/permissions", {
    body: { permissions: payload.permissions.map((item) => ({ ...item, permission: capitalize(item.permission), confirmLevel: capitalize(item.confirmLevel) })) },
  }));
}

export type ToolPermission = "run" | "ask" | "no";
export type ToolRiskLevel = "safe" | "low" | "medium" | "high" | "critical";
export interface ToolPermissionRecord {
  toolCode: string;
  permission: ToolPermission;
  confirmLevel: ToolRiskLevel;
}
export interface ToolPermissionsPayload { permissions: ToolPermissionRecord[] }

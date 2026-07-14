export type SystemConfigEffect = "hotUpdate" | "restart" | "migration";
export type SystemConfigValue = string | number | boolean | null;

export interface SystemConfigItem {
  effect: SystemConfigEffect;
  overriddenByEnvironment: boolean;
  path: string;
  sensitive: boolean;
  value: SystemConfigValue;
  valueType: "string" | "number" | "boolean";
}

export interface SystemConfigDocument {
  items: SystemConfigItem[];
  path: string;
  revision: string;
}

export interface SystemConfigChange {
  path: string;
  value: SystemConfigValue;
}

export interface SystemConfigUpdateResult {
  changedPaths: string[];
  hotUpdatePaths: string[];
  migrationPaths: string[];
  restartPaths: string[];
  revision: string;
}

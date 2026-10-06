export type SystemConfigEffect = "hotUpdate" | "restart" | "migration";
export type SystemConfigValue = string | number | boolean | null | SystemConfigValue[] | { [key: string]: SystemConfigValue };

export type MineruNodeConfig = {
  id: string;
  baseUrl: string;
  enabled: boolean;
  allowRemote: boolean;
  capacity: number;
  weight: number;
  resourceGroup: string;
  resourceGroupCapacity: number;
};

export interface SystemConfigItem {
  effect: SystemConfigEffect;
  overriddenByEnvironment: boolean;
  path: string;
  sensitive: boolean;
  value: SystemConfigValue;
  valueType: "string" | "number" | "boolean" | "array";
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

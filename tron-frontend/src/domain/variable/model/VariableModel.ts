export type VariableSourceKind = "Local" | "Inherited" | "Override";
export type VariableValueMode = "PRESERVE" | "REPLACE";

export interface SensitiveVariableDisplay {
  isSensitive?: boolean;
  valueMasked?: boolean;
  hasValue?: boolean;
}

export interface VariableRecord extends SensitiveVariableDisplay {
  key: string;
  value: string;
  description: string | null;
  sourceKind: VariableSourceKind;
  sourceWorkspaceCode: string;
  sourceWorkspaceName: string;
  sourceLabel: string;
  shadowedWorkspaceCode: string | null;
  shadowedWorkspaceName: string | null;
  shadowedLabel: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface PlatformVariableRecord extends SensitiveVariableDisplay {
  id: string;
  key: string;
  value: string;
  description: string | null;
  createdAt: string;
  updatedAt: string;
}

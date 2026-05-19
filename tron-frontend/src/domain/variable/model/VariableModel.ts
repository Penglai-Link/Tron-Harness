export type VariableSourceKind = "Local" | "Inherited" | "Override";

export interface VariableRecord {
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
  updatedAt: string;
}

import type { WorkspaceStatus } from "@/domain/admin/model";

export interface WorkspaceOption {
  workspaceCode: string;
  label: string;
  name: string;
  description: string | null;
  parentWorkspaceCode: string | null;
  depth: number;
  status: WorkspaceStatus;
}

export interface PrimarySpace {
  workspaceCode: string;
  name: string;
  description: string | null;
  status: WorkspaceStatus;
  childCount: number;
}

export interface SecondarySpace {
  workspaceCode: string;
  primaryWorkspaceCode: string;
  parentWorkspaceCode?: string | null;
  label: string;
  name: string;
  description: string | null;
  status: WorkspaceStatus;
  depth: number;
  childCount: number;
}

export interface WorkspaceSwitcherData {
  primarySpaces: PrimarySpace[];
  secondarySpaces: SecondarySpace[];
}

export interface WorkspaceSpacesContextValue {
  primarySpaces: PrimarySpace[];
  secondarySpaces: SecondarySpace[];
  selectedPrimarySpace?: PrimarySpace;
  selectedPrimarySpaceId: string;
  selectedSecondarySpace?: SecondarySpace;
  selectedSecondarySpaceId: string;
}

export interface PrimarySpace {
  id: string;
  name: string;
  description: string;
  region: string;
  status: "Active" | "Review" | "Archived";
}

export interface SecondarySpace {
  id: string;
  primarySpaceId: string;
  name: string;
  description: string;
  inheritedResources: number;
  localOverrides: number;
  status: "Active" | "Review" | "Disabled";
}

export interface WorkspaceSpacesContextValue {
  primarySpaces: PrimarySpace[];
  secondarySpaces: SecondarySpace[];
  selectedPrimarySpace?: PrimarySpace;
  selectedPrimarySpaceId: string;
  selectedSecondarySpace?: SecondarySpace;
  selectedSecondarySpaceId: string;
}

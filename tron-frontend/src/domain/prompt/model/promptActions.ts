import type { PromptSourceKind } from "./PromptModel";

export type WorkspacePromptAction = "EDIT" | "OVERRIDE" | "DUPLICATE" | "SET_ENABLED" | "DELETE" | "REVERT";

const workspacePromptActionMatrix: Record<PromptSourceKind, WorkspacePromptAction[]> = {
  LOCAL: ["EDIT", "DUPLICATE", "SET_ENABLED", "DELETE"],
  INHERITED: ["OVERRIDE", "SET_ENABLED"],
  OVERRIDE: ["EDIT", "SET_ENABLED", "REVERT"],
  PLATFORM: [],
};

export function getWorkspacePromptActions(sourceKind: PromptSourceKind) {
  return [...workspacePromptActionMatrix[sourceKind]];
}

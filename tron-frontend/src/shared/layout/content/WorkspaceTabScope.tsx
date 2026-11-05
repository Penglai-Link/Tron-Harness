import { useRef, type ReactNode } from "react";
import { WorkspaceSpacesContext, useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import type { WorkspaceSpacesContextValue } from "@/shared/types/WorkspaceSpace";

/** Cached pages keep the space they were opened in, including while another
 * space is visible. Switching tabs never rebinds a chat adapter or its drafts. */
export function WorkspaceTabScope({ workspaceCode, children }: { workspaceCode?: string; children: ReactNode }) {
  const current = useWorkspaceSpaces();
  const owned = useRef<WorkspaceSpacesContextValue | null>(null);
  const currentCode = current.selectedSecondarySpaceId || current.selectedPrimarySpaceId;
  if (currentCode && (!workspaceCode || workspaceCode === currentCode)) owned.current = current;
  if (!owned.current) return null;
  return <WorkspaceSpacesContext.Provider value={owned.current}>{children}</WorkspaceSpacesContext.Provider>;
}

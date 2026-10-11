import { useWorkspaceSpaces } from "@/shared";
import { TraceExplorer } from "../components/TraceExplorer";
export function TracesPage() {
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspaceCode =
    (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  return <TraceExplorer key={workspaceCode} workspaceCode={workspaceCode} />;
}

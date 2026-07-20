import { useResourcePage } from "./useResourcePage";

export function useResourceList(workspaceCode: string, workspaceName: string) {
  const page = useResourcePage({
    context: "ALL",
    current: 1,
    lifecycle: "ALL",
    scopeKind: "WORKSPACE",
    search: "",
    size: 50,
    validation: "ALL",
    workspaceCode: workspaceCode || null,
    workspaceName,
  });
  return page.data.records;
}

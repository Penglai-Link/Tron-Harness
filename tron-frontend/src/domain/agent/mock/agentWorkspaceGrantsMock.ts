const grants = new Map<string, string[]>();

export function listAgentWorkspaceGrantsMock(code: string) {
  return { workspaceCodes: [...(grants.get(code) ?? [])] };
}

export function saveAgentWorkspaceGrantsMock(code: string, workspaceCodes: string[]) {
  grants.set(code, [...new Set(workspaceCodes)]);
}

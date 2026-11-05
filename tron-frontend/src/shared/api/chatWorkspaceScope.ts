// Session ownership is routing context, never a replacement for server authorization.
// Keep it independent of React and stores so async work retains its original space.
const sessionWorkspaces = new Map<string, string>();

export function rememberChatWorkspace(sessionCode: string, workspaceCode: string | null | undefined) {
  if (sessionCode && workspaceCode) sessionWorkspaces.set(sessionCode, workspaceCode);
}

export function chatWorkspaceForSession(sessionCode: string | null | undefined) {
  return sessionCode ? sessionWorkspaces.get(sessionCode) : undefined;
}

export function clearChatWorkspaces() {
  sessionWorkspaces.clear();
}

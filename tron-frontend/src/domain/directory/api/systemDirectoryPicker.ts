import { pickDirectoryPath } from "./directoryApi";
import { electronFs } from "@/shared/fs/electronFs";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { getApiBaseUrl } from "@/shared/api/httpClient";

// Picking a path does not register it as a workspace or grant ongoing access.
export async function chooseSystemDirectoryPath(workspaceCode: string): Promise<string | null> {
  if (electronFs.isAvailable()) {
    const status = await new DesktopBridge().getBackendStatus();
    const apiUrl = new URL(getApiBaseUrl() || window.location.origin, window.location.origin);
    if (status.ready && apiUrl.origin === new URL(status.backendUrl).origin) {
      const root = await electronFs.pickRoot();
      if (!root) return null;
      try { return root.path; }
      finally { await electronFs.closeRoot(root.rootId); }
    }
  }
  return (await pickDirectoryPath(workspaceCode)).path;
}

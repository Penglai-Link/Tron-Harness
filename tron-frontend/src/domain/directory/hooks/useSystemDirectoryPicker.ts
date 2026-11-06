import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import { pickDirectory, registerDirectory, type DirectoryRecord } from "@/domain/directory/api/directoryApi";
import { electronFs } from "@/shared/fs/electronFs";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { getApiBaseUrl } from "@/shared/api/httpClient";

export function useSystemDirectoryPicker(workspaceCode: string, onPicked: (directory: DirectoryRecord) => void | Promise<void>) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const [saving, setSaving] = useState(false);
  const picking = useRef(false);

  async function choose() {
    if (!workspaceCode || picking.current) return;
    picking.current = true;
    setSaving(true);
    try {
      let useNativeBridge = false;
      if (electronFs.isAvailable()) {
        const status = await new DesktopBridge().getBackendStatus();
        const apiUrl = new URL(getApiBaseUrl() || window.location.origin, window.location.origin);
        useNativeBridge = status.ready && apiUrl.origin === new URL(status.backendUrl).origin;
      }
      let directory: DirectoryRecord | null;
      if (useNativeBridge) {
        const root = await electronFs.pickRoot();
        if (!root) return;
        try { directory = await registerDirectory(workspaceCode, root.path); }
        finally { await electronFs.closeRoot(root.rootId); }
      } else {
        directory = await pickDirectory(workspaceCode);
      }
      if (!directory) return;
      window.dispatchEvent(new Event("tron:directories-changed"));
      await onPicked(directory);
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("chatSidebar.directory.saveFailed"));
    } finally {
      picking.current = false;
      setSaving(false);
    }
  }
  return { choose, saving };
}

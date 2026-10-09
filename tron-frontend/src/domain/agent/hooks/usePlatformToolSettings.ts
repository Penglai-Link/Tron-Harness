import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { fetchToolDomains, listTools } from "../api";
import { listToolDomainsMock, listToolsMock } from "../mock";
import { fetchToolPermissions, updateToolPermissions } from "../api/toolPermissionsApi";
import type { PlatformToolRecord, ToolCatalogDomainRecord } from "../model";
import type { ToolPermissionRecord } from "../model/ToolPermissionModel";

export function usePlatformToolSettings() {
  const { t } = useTranslation("pages");
  const pageT = (key: string, fallback: string) => t(`settingsTools.${key}`, { defaultValue: fallback });
  const toast = useToast();
  const dataSource = getServiceDataSource("agent");
  const [domains, setDomains] = useState<ToolCatalogDomainRecord[]>([]);
  const [tools, setTools] = useState<PlatformToolRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [permissions, setPermissions] = useState<ToolPermissionRecord[]>([]);
  const [savedPermissions, setSavedPermissions] = useState<ToolPermissionRecord[]>([]);
  const dirty = JSON.stringify(permissions) !== JSON.stringify(savedPermissions);

  useEffect(() => {
    if (!dirty) return;
    const protect = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = ""; };
    window.addEventListener("beforeunload", protect);
    return () => window.removeEventListener("beforeunload", protect);
  }, [dirty]);
  async function refreshTools() {
    setLoading(true);
    try {
      const [nextDomains, nextTools, nextPermissions] = dataSource === "api"
        ? await Promise.all([
          fetchToolDomains(),
          listTools(),
          fetchToolPermissions(),
        ])
        : await Promise.all([
          listToolDomainsMock(),
          listToolsMock(),
          Promise.resolve(permissions),
        ]);

      setDomains(nextDomains);
      setTools(nextTools);
      const byCode = new Map(nextPermissions.map((item) => [item.toolCode, item]));
      const normalized = nextTools.map((tool) => byCode.get(tool.toolCode) ?? { toolCode: tool.toolCode, permission: "run" as const, confirmLevel: "low" as const });
      setPermissions(normalized);
      setSavedPermissions(normalized);
      setLoaded(true);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform tools could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshTools();
  }, [dataSource]);

  async function savePermissions() {
    if (loading || saving || !loaded || !dirty) return;
    setSaving(true);
    try {
      const saved = dataSource === "api" ? await updateToolPermissions({ permissions }) : permissions;
      setPermissions(saved);
      setSavedPermissions(saved);
      toast.success(pageT("permissions.saved", "Permissions saved"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("permissions.saveError", "Permissions could not be saved. Your changes are preserved."));
    } finally { setSaving(false); }
  }

  return { domains, tools, loading, saving, loaded, permissions, dirty, refreshTools, savePermissions,
    discard: () => setPermissions(savedPermissions),
    setPermission: (next: ToolPermissionRecord) => setPermissions((current) => current.map((item) => item.toolCode === next.toolCode ? next : item)),
  };
}

import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { fetchToolCategories, fetchToolDomains, listTools } from "../api";
import { listToolDomainsMock, listToolsMock } from "../mock";
import { fetchToolPermissions, updateToolPermissions } from "../api/toolPermissionsApi";
import type { PlatformToolRecord, ToolCatalogDomainRecord, ToolCategoryRecord } from "../model";
import type { ToolPermissionRecord } from "../model/ToolPermissionModel";

export function usePlatformToolSettings() {
  const { t, i18n } = useTranslation("pages");
  const pageT = (key: string, fallback: string) => t(`settingsTools.${key}`, { defaultValue: fallback });
  const toast = useToast();
  const dataSource = getServiceDataSource("agent");
  const [domains, setDomains] = useState<ToolCatalogDomainRecord[]>([]);
  const [tools, setTools] = useState<PlatformToolRecord[]>([]);
  const [categories, setCategories] = useState<ToolCategoryRecord[]>([]);
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
  const requestVersion = useRef(0);
  const edits = useRef({ dirty, permissions });
  edits.current = { dirty, permissions };
  async function refreshTools(preserveEdits = false) {
    const version = ++requestVersion.current;
    setLoading(true);
    try {
      const [nextDomains, nextTools, nextPermissions, nextCategories] = dataSource === "api"
        ? await Promise.all([
          fetchToolDomains(),
          listTools(),
          fetchToolPermissions(),
          fetchToolCategories(),
        ])
        : await Promise.all([
          listToolDomainsMock(),
          listToolsMock(),
          Promise.resolve(permissions),
          Promise.resolve([]),
        ]);

      if (version !== requestVersion.current) return;
      setDomains(nextDomains);
      setTools(nextTools);
      setCategories(nextCategories);
      const byCode = new Map(nextPermissions.map((item) => [item.toolCode, item]));
      const normalized = nextTools.map((tool) => byCode.get(tool.toolCode) ?? { toolCode: tool.toolCode, permission: "run" as const, confirmLevel: "low" as const });
      if (preserveEdits && edits.current.dirty) {
        const changed = new Map(edits.current.permissions.map(item => [item.toolCode, item]));
        setPermissions(normalized.map(item => changed.get(item.toolCode) ?? item));
      } else { setPermissions(normalized); }
      setSavedPermissions(normalized);
      setLoaded(true);
    } catch (error) {
      if (version !== requestVersion.current) return;
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform tools could not be loaded"));
    } finally {
      if (version === requestVersion.current) setLoading(false);
    }
  }

  useEffect(() => {
    void refreshTools(true);
    return () => { ++requestVersion.current; };
  }, [dataSource, i18n.resolvedLanguage]);

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

  return { domains, tools, categories, loading, saving, loaded, permissions, dirty, refreshTools: () => refreshTools(), savePermissions,
    discard: () => setPermissions(savedPermissions),
    setPermission: (next: ToolPermissionRecord) => setPermissions((current) => current.map((item) => item.toolCode === next.toolCode ? next : item)),
  };
}

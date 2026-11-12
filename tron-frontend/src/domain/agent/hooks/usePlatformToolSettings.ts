import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { fetchToolCatalogIssues, fetchToolCategories, fetchToolDomains, listTools, type ToolCatalogIssueRecord } from "../api";
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
  const [catalogIssues, setCatalogIssues] = useState<ToolCatalogIssueRecord[]>([]);
  const [loadErrors, setLoadErrors] = useState<string[]>([]);
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
      // Refresh discovery first so domains, permissions and tools share the new catalog.
      const [discovery] = await Promise.allSettled([
        dataSource === "api" ? fetchToolCatalogIssues() : Promise.resolve([] as ToolCatalogIssueRecord[]),
      ]);
      if (version !== requestVersion.current) return;
      const results = dataSource === "api"
        ? await Promise.allSettled([
          fetchToolDomains(), listTools(), fetchToolPermissions(), fetchToolCategories(),
        ] as const)
        : await Promise.allSettled([
          Promise.resolve(listToolDomainsMock()), Promise.resolve(listToolsMock()),
          Promise.resolve(permissions), Promise.resolve([] as ToolCategoryRecord[]),
        ] as const);
      if (version !== requestVersion.current) return;
      const [domainResult, toolResult, permissionResult, categoryResult] = results;
      const labels = ["domains", "tools", "permissions", "categories", "diagnostics"];
      setLoadErrors([...results, discovery].flatMap((result, index) => result.status === "rejected"
        ? [`${pageT(`loadParts.${labels[index]}`, labels[index])}: ${result.reason instanceof Error ? result.reason.message : pageT("errors.load", "Platform tools could not be loaded")}`]
        : []));
      if (domainResult.status === "fulfilled") setDomains(domainResult.value);
      if (toolResult.status === "fulfilled") setTools(toolResult.value);
      if (categoryResult.status === "fulfilled") {
        setCategories(categoryResult.value);
      } else if (toolResult.status === "fulfilled") {
        const derived = new Map<string, ToolCategoryRecord>();
        for (const tool of toolResult.value) {
          if (!tool.category) continue;
          const { code, names } = tool.category;
          const category = derived.get(code) ?? { code, name: names[i18n.resolvedLanguage ?? "en-US"] ?? names["en-US"] ?? code, toolCount: 0 };
          category.toolCount++;
          derived.set(code, category);
        }
        setCategories([...derived.values()]);
      }
      if (discovery.status === "fulfilled") setCatalogIssues(discovery.value);
      // Failed permission reads must never become default grants that can be saved.
      const canEdit = toolResult.status === "fulfilled" && permissionResult.status === "fulfilled";
      setLoaded(canEdit);
      if (canEdit) {
        const byCode = new Map(permissionResult.value.map((item) => [item.toolCode, item]));
        for (const tool of toolResult.value) {
          if (!byCode.has(tool.toolCode)) byCode.set(tool.toolCode, { toolCode: tool.toolCode, permission: "run", confirmLevel: "low" });
        }
        // Keep settings for tools omitted by a partial discovery result.
        const normalized = [...byCode.values()];
        if (preserveEdits && edits.current.dirty) {
          const changed = new Map(edits.current.permissions.map(item => [item.toolCode, item]));
          setPermissions(normalized.map(item => changed.get(item.toolCode) ?? item));
        } else { setPermissions(normalized); }
        setSavedPermissions(normalized);
      }
    } catch (error) {
      if (version !== requestVersion.current) return;
      setLoaded(false);
      setLoadErrors([error instanceof Error ? error.message : pageT("errors.load", "Platform tools could not be loaded")]);
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

  return { domains, tools, categories, catalogIssues, loadErrors, loading, saving, loaded, permissions, dirty, refreshTools: () => refreshTools(), savePermissions,
    discard: () => setPermissions(savedPermissions),
    setPermission: (next: ToolPermissionRecord) => setPermissions((current) => current.map((item) => item.toolCode === next.toolCode ? next : item)),
  };
}

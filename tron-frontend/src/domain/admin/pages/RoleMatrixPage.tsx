import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Copy,
  Plus,
  RotateCcw,
  Save,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  TextInput,
  ToggleSwitch,
  useToast,
} from "@/shared";

import {
  usePermissionDomainList,
  useRoleList,
  useUserList,
} from "@/domain/admin/hooks";
import { replaceAdminRoles } from "@/domain/admin/api";
import type { RoleRecord } from "@/domain/admin/model";
import {
  adminPermissionCodes,
  adminPermissionLegend,
  permissionDomains,
} from "@/domain/admin/constants";
import { cn } from "@/shared/lib/utils";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { SelectItem } from "@/components/ui/select";

type PermissionCode = (typeof adminPermissionCodes)[number];
type RoleKind = "System" | "Custom";
type ManagedBy = "Platform" | "Workspace";
type RoleTypeFilter = "All" | RoleKind;

interface EditableRole {
  code: string;
  description: string;
  id: string;
  kind: RoleKind;
  managedBy: ManagedBy;
  name: string;
  permissions: Record<string, PermissionCode[]>;
  updatedAt: string;
}

const STORAGE_KEY = "tron.admin.role-matrix.v1";
const ALL_DOMAINS = "All domains";
const ROLE_TYPE_OPTIONS: readonly RoleTypeFilter[] = ["All", "System", "Custom"];
const HEADER_ACTION_BUTTON_CLASS = "w-[7.5rem] justify-center";
const SYSTEM_ROLE_CODES_BY_ID: Record<string, string> = {
  admin: "ADMIN",
  "workspace-admin": "WORKSPACE_ADMIN",
  developer: "DEVELOPER",
  viewer: "VIEWER",
};
const SYSTEM_ROLE_CODES_BY_NAME: Record<string, string> = {
  Admin: "ADMIN",
  "Workspace Admin": "WORKSPACE_ADMIN",
  Developer: "DEVELOPER",
  Viewer: "VIEWER",
};
const MANAGED_BY_DEFAULTS: Record<string, ManagedBy> = {
  ADMIN: "Platform",
  WORKSPACE_ADMIN: "Workspace",
  DEVELOPER: "Platform",
  VIEWER: "Platform",
};
const DATE_FORMATTER = new Intl.DateTimeFormat("en-US", {
  dateStyle: "medium",
  timeStyle: "short",
});

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function normalizeRoleCodeInput(value: string) {
  return value
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .replace(/_{2,}/g, "_");
}

function formatTimestamp(value: string) {
  return DATE_FORMATTER.format(new Date(value));
}

function sortPermissionCodes(codes: readonly string[]) {
  return adminPermissionCodes.filter((code) => codes.includes(code)) as PermissionCode[];
}

function buildPermissionMap(source: Record<string, string[]> | undefined, domains: readonly string[]) {
  return Object.fromEntries(
    domains.map((domain) => [domain, sortPermissionCodes(source?.[domain] ?? [])]),
  ) as Record<string, PermissionCode[]>;
}

function cloneRole(role: EditableRole, domains: readonly string[]): EditableRole {
  return {
    code: role.code,
    description: role.description,
    id: role.id,
    kind: role.kind,
    managedBy: role.managedBy,
    name: role.name,
    permissions: buildPermissionMap(role.permissions, domains),
    updatedAt: role.updatedAt,
  };
}

function serializeRoles(roles: readonly EditableRole[], domains: readonly string[]) {
  return JSON.stringify(
    [...roles]
      .map((role) => ({
        code: role.code,
        description: role.description,
        id: role.id,
        kind: role.kind,
        managedBy: role.managedBy,
        name: role.name,
        permissions: Object.fromEntries(
          domains.map((domain) => [domain, sortPermissionCodes(role.permissions[domain] ?? [])]),
        ),
        updatedAt: role.updatedAt,
      }))
      .sort((left, right) => left.code.localeCompare(right.code)),
  );
}

function normalizeRoleRecord(role: RoleRecord, domains: readonly string[]): EditableRole {
  const code = normalizeRoleCodeInput(
    role.code
    ?? SYSTEM_ROLE_CODES_BY_ID[role.id]
    ?? SYSTEM_ROLE_CODES_BY_NAME[role.name]
    ?? role.id,
  );

  return {
    code,
    description: role.description,
    id: role.id,
    kind: role.kind ?? (code.startsWith("CUSTOM_") ? "Custom" : "System"),
    managedBy: role.managedBy ?? (MANAGED_BY_DEFAULTS[code] ?? "Workspace"),
    name: role.name,
    permissions: buildPermissionMap(role.permissions, domains),
    updatedAt: role.updatedAt ?? new Date().toISOString(),
  };
}

function normalizePersistedRole(value: unknown, domains: readonly string[]): EditableRole | null {
  if (!isRecord(value)) return null;

  const code = typeof value.code === "string" ? normalizeRoleCodeInput(value.code) : "";
  const id = typeof value.id === "string" ? value.id : "";
  const name = typeof value.name === "string" ? value.name : "";
  const description = typeof value.description === "string" ? value.description : "";
  const updatedAt = typeof value.updatedAt === "string" ? value.updatedAt : new Date().toISOString();
  const kind: RoleKind = value.kind === "System" ? "System" : "Custom";
  const managedBy: ManagedBy = value.managedBy === "Platform" ? "Platform" : "Workspace";
  const permissions = buildPermissionMap(isRecord(value.permissions) ? value.permissions as Record<string, string[]> : undefined, domains);

  if (!code || !id || !name) return null;

  return {
    code,
    description,
    id,
    kind,
    managedBy,
    name,
    permissions,
    updatedAt,
  };
}

function readPersistedRoles(domains: readonly string[]) {
  if (typeof window === "undefined") return null;

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return null;
    const roles = parsed
      .map((item) => normalizePersistedRole(item, domains))
      .filter((item): item is EditableRole => item !== null);
    return roles.length ? roles : null;
  } catch {
    return null;
  }
}

function nextCustomRoleOrdinal(roles: readonly EditableRole[]) {
  let ordinal = 1;

  while (roles.some((role) => role.code === `CUSTOM_ROLE_${String(ordinal).padStart(2, "0")}`)) {
    ordinal += 1;
  }

  return ordinal;
}

function makeCustomRole(domains: readonly string[], roles: readonly EditableRole[], source?: EditableRole): EditableRole {
  const ordinal = nextCustomRoleOrdinal(roles);
  const suffix = String(ordinal).padStart(2, "0");
  const baseName = source ? `${source.name} Copy` : `New Custom Role ${suffix}`;
  const baseCode = source ? `${source.code}_COPY` : `CUSTOM_ROLE_${suffix}`;

  return {
    code: normalizeRoleCodeInput(baseCode),
    description: source?.description ?? "Role draft created from the role matrix workspace.",
    id: `custom-role-${Date.now()}-${suffix}`,
    kind: "Custom",
    managedBy: "Workspace",
    name: baseName,
    permissions: buildPermissionMap(source?.permissions, domains),
    updatedAt: new Date().toISOString(),
  };
}

function validateRoleCatalog(roles: readonly EditableRole[]) {
  const codeSet = new Set<string>();
  const nameSet = new Set<string>();

  for (const role of roles) {
    if (!role.name.trim()) return "Role name is required.";
    if (!role.code.trim()) return "Role code is required.";

    const normalizedCode = normalizeRoleCodeInput(role.code);
    if (!normalizedCode) return "Role code must contain letters or numbers.";
    if (codeSet.has(normalizedCode)) return `Duplicate role code: ${normalizedCode}`;
    codeSet.add(normalizedCode);

    const normalizedName = role.name.trim().toLowerCase();
    if (nameSet.has(normalizedName)) return `Duplicate role name: ${role.name.trim()}`;
    nameSet.add(normalizedName);
  }

  return null;
}

function roleSearchText(role: EditableRole) {
  return [
    role.name,
    role.code,
    role.description,
    role.kind,
    role.managedBy,
  ].join(" ").toLowerCase();
}

export function RoleMatrixPage() {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const rawRoles = useRoleList();
  const domainsFromHook = usePermissionDomainList();
  const allUsers = useUserList({
    current: 1,
    search: "",
    size: 200,
    status: "All",
  });
  const dataSource = getServiceDataSource("admin");
  const allDomains: string[] = domainsFromHook.length ? [...domainsFromHook] : [...permissionDomains];
  const initialCatalog = useMemo(
    () => rawRoles.map((role) => normalizeRoleRecord(role, allDomains)),
    [allDomains, rawRoles],
  );
  const [catalogInitialized, setCatalogInitialized] = useState(false);
  const [savedRoles, setSavedRoles] = useState<EditableRole[]>([]);
  const [workingRoles, setWorkingRoles] = useState<EditableRole[]>([]);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [roleType, setRoleType] = useState<RoleTypeFilter>("All");
  const [domainFilter, setDomainFilter] = useState<string>(ALL_DOMAINS);
  const [selectedRoleId, setSelectedRoleId] = useState<string>("");
  const [showChangedOnly, setShowChangedOnly] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleteConfirmRoleId, setDeleteConfirmRoleId] = useState<string | null>(null);
  const [renameFocusRoleId, setRenameFocusRoleId] = useState<string | null>(null);
  const actionTimestampsRef = useRef(new Map<string, number>());

  useEffect(() => {
    if (catalogInitialized || !initialCatalog.length) return;
    const persisted = dataSource === "api" ? null : readPersistedRoles(allDomains);
    const baseline = (persisted ?? initialCatalog).map((role) => cloneRole(role, allDomains));
    setSavedRoles(baseline);
    setWorkingRoles(baseline.map((role) => cloneRole(role, allDomains)));
    setSelectedRoleId(baseline[0]?.id ?? "");
    setCatalogInitialized(true);
  }, [allDomains, catalogInitialized, dataSource, initialCatalog]);

  useEffect(() => {
    if (!catalogInitialized || !initialCatalog.length) return;
    if (dataSource !== "api") return;
    const baseline = initialCatalog.map((role) => cloneRole(role, allDomains));
    setSavedRoles((currentSavedRoles) => {
      const baselineSignature = serializeRoles(baseline, allDomains);
      const savedSignature = serializeRoles(currentSavedRoles, allDomains);

      if (baselineSignature === savedSignature) return currentSavedRoles;

      setWorkingRoles((currentRoles) => {
        if (!currentRoles.length) return baseline.map((role) => cloneRole(role, allDomains));
        const currentSignature = serializeRoles(currentRoles, allDomains);
        if (currentSignature !== savedSignature) return currentRoles;
        return baseline.map((role) => cloneRole(role, allDomains));
      });

      return baseline;
    });
  }, [allDomains, catalogInitialized, dataSource, initialCatalog]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedQuery(query.trim().toLowerCase());
    }, 280);

    return () => window.clearTimeout(timer);
  }, [query]);

  const membersByRoleCode = useMemo(() => {
    const nextMap = new Map<string, string[]>();

    for (const user of allUsers.records) {
      for (const roleCode of user.roleCodes) {
        const names = nextMap.get(roleCode) ?? [];
        names.push(user.displayName);
        nextMap.set(roleCode, names);
      }
    }

    return nextMap;
  }, [allUsers.records]);

  const dirtyIds = useMemo(() => {
    if (!workingRoles.length || !savedRoles.length) return new Set<string>();
    const savedById = new Map(savedRoles.map((role) => [role.id, role] as const));
    const nextDirtyIds = new Set<string>();

    for (const role of workingRoles) {
      const baseline = savedById.get(role.id);
      if (!baseline) {
        nextDirtyIds.add(role.id);
        continue;
      }
      if (serializeRoles([role], allDomains) !== serializeRoles([baseline], allDomains)) {
        nextDirtyIds.add(role.id);
      }
    }

    return nextDirtyIds;
  }, [allDomains, savedRoles, workingRoles]);

  const filteredRoles = useMemo(() => {
    return workingRoles.filter((role) => {
      if (roleType !== "All" && role.kind !== roleType) return false;
      if (showChangedOnly && !dirtyIds.has(role.id)) return false;
      if (!debouncedQuery) return true;
      return roleSearchText(role).includes(debouncedQuery);
    });
  }, [debouncedQuery, dirtyIds, roleType, showChangedOnly, workingRoles]);

  useEffect(() => {
    if (!filteredRoles.length) {
      setSelectedRoleId("");
      return;
    }
    if (!filteredRoles.some((role) => role.id === selectedRoleId)) {
      setSelectedRoleId(filteredRoles[0].id);
    }
  }, [filteredRoles, selectedRoleId]);

  useEffect(() => {
    if (domainFilter !== ALL_DOMAINS && !allDomains.includes(domainFilter)) {
      setDomainFilter(ALL_DOMAINS);
    }
  }, [allDomains, domainFilter]);

  const visibleDomains = domainFilter === ALL_DOMAINS ? allDomains : [domainFilter];
  const selectedRole = workingRoles.find((role) => role.id === selectedRoleId) ?? filteredRoles[0] ?? null;

  useEffect(() => {
    if (deleteConfirmRoleId && deleteConfirmRoleId !== selectedRole?.id) {
      setDeleteConfirmRoleId(null);
    }
  }, [deleteConfirmRoleId, selectedRole?.id]);

  const hasDirtyChanges = dirtyIds.size > 0;

  function updateRole(roleId: string, updater: (role: EditableRole) => EditableRole) {
    setWorkingRoles((currentRoles) => currentRoles.map((role) => (role.id === roleId ? updater(role) : role)));
  }

  function runDebouncedAction(key: string, action: () => void, delayMs = 260) {
    const now = Date.now();
    const lastTriggeredAt = actionTimestampsRef.current.get(key) ?? 0;
    if (now - lastTriggeredAt < delayMs) return;
    actionTimestampsRef.current.set(key, now);
    action();
  }

  function clearSearch() {
    setQuery("");
    setDebouncedQuery("");
  }

  function clearFilters() {
    clearSearch();
    setRoleType("All");
    setDomainFilter(ALL_DOMAINS);
    setShowChangedOnly(false);
  }

  function selectRole(roleId: string) {
    setDeleteConfirmRoleId(null);
    setSelectedRoleId(roleId);
  }

  function renameRole(roleId: string, nextName: string) {
    updateRole(roleId, (role) => ({
      ...role,
      name: nextName,
      updatedAt: new Date().toISOString(),
    }));
  }

  function togglePermission(roleId: string, domain: string, code: PermissionCode) {
    updateRole(roleId, (role) => {
      const currentCodes = role.permissions[domain] ?? [];
      const nextCodes = currentCodes.includes(code)
        ? currentCodes.filter((item) => item !== code)
        : sortPermissionCodes([...currentCodes, code]);

      return {
        ...role,
        permissions: {
          ...role.permissions,
          [domain]: nextCodes,
        },
        updatedAt: new Date().toISOString(),
      };
    });
    selectRole(roleId);
  }

  function addCustomRole() {
    const role = makeCustomRole(allDomains, workingRoles);
    setDeleteConfirmRoleId(null);
    setWorkingRoles((currentRoles) => [role, ...currentRoles]);
    setRoleType("All");
    clearSearch();
    setSelectedRoleId(role.id);
    setRenameFocusRoleId(role.id);
    toast.info("Draft role created");
  }

  function cloneSelectedRole() {
    if (!selectedRole) return;
    const clone = makeCustomRole(allDomains, workingRoles, selectedRole);
    setDeleteConfirmRoleId(null);
    setWorkingRoles((currentRoles) => [clone, ...currentRoles]);
    setRoleType("All");
    clearSearch();
    setSelectedRoleId(clone.id);
    setRenameFocusRoleId(clone.id);
    toast.success(`Cloned ${selectedRole.name}`);
  }

  function resetSelectedRole() {
    if (!selectedRole) return;
    setDeleteConfirmRoleId(null);
    const baseline = savedRoles.find((role) => role.id === selectedRole.id);

    if (!baseline) {
      setRenameFocusRoleId(null);
      setWorkingRoles((currentRoles) => currentRoles.filter((role) => role.id !== selectedRole.id));
      toast.info("Unsaved custom role removed");
      return;
    }

    setWorkingRoles((currentRoles) => currentRoles.map((role) => (role.id === selectedRole.id ? cloneRole(baseline, allDomains) : role)));
    toast.info(`Reset ${selectedRole.name}`);
  }

  function deleteSelectedRole() {
    if (!selectedRole) return;
    if (selectedRole.kind === "System") {
      toast.warning("System roles cannot be deleted");
      return;
    }
    if (deleteConfirmRoleId !== selectedRole.id) {
      setDeleteConfirmRoleId(selectedRole.id);
      toast.warning(`Click Confirm to delete ${selectedRole.name}`);
      return;
    }

    setDeleteConfirmRoleId(null);
    setRenameFocusRoleId(null);
    setWorkingRoles((currentRoles) => currentRoles.filter((role) => role.id !== selectedRole.id));
    toast.success(`${selectedRole.name} deleted`);
  }

  async function saveDraft() {
    const validationError = validateRoleCatalog(workingRoles);
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setSaving(true);

    try {
      await new Promise((resolve) => window.setTimeout(resolve, 280));
      const snapshot = workingRoles.map((role) => cloneRole(role, allDomains));
      const persistedRoles = dataSource === "api"
        ? (await replaceAdminRoles({
          roles: snapshot.map((role) => ({
            id: role.id,
            code: role.code,
            name: role.name,
            description: role.description,
            kind: role.kind,
            managedBy: role.managedBy,
            permissions: role.permissions,
            updatedAt: role.updatedAt,
          })),
        })).map((role) => normalizeRoleRecord(role, allDomains))
        : snapshot;

      if (dataSource !== "api") {
        window.localStorage.setItem(STORAGE_KEY, JSON.stringify(snapshot));
      }
      setDeleteConfirmRoleId(null);
      setSavedRoles(persistedRoles.map((role) => cloneRole(role, allDomains)));
      setWorkingRoles(persistedRoles.map((role) => cloneRole(role, allDomains)));
      setRenameFocusRoleId(null);
      if (selectedRole) {
        const persistedSelected = persistedRoles.find((role) => role.code === selectedRole.code);
        if (persistedSelected) setSelectedRoleId(persistedSelected.id);
      }
      toast.success(t("pages:admin.roles.messages.saved", "Role matrix draft saved"));
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to save role matrix draft";
      toast.error(message);
    } finally {
      setSaving(false);
    }
  }

  function discardChanges() {
    setDeleteConfirmRoleId(null);
    setRenameFocusRoleId(null);
    setWorkingRoles(savedRoles.map((role) => cloneRole(role, allDomains)));
    toast.info(t("pages:admin.roles.messages.discarded", "Unsaved changes discarded"));
  }

  return (
    <>
      <PageHeader
        title={t("pages:admin.roles.title", "Users & Roles")}
        actions={
          <>
            <Button
              className={HEADER_ACTION_BUTTON_CLASS}
              size="sm"
              variant="secondary"
              onClick={() => runDebouncedAction("header-add-role", addCustomRole)}
            >
              <Plus size={16} aria-hidden="true" />
              {t("pages:admin.roles.actions.add", "Add Role")}
            </Button>
            <Button
              className={HEADER_ACTION_BUTTON_CLASS}
              disabled={!selectedRole}
              size="sm"
              type="button"
              variant="secondary"
              onClick={() => runDebouncedAction("header-clone-role", cloneSelectedRole)}
            >
              <Copy size={14} aria-hidden="true" />
              {t("pages:admin.roles.actions.clone", "Clone")}
            </Button>
            <Button
              className={HEADER_ACTION_BUTTON_CLASS}
              disabled={!selectedRole}
              size="sm"
              type="button"
              variant="secondary"
              onClick={() => runDebouncedAction("header-reset-role", resetSelectedRole)}
            >
              <RotateCcw size={14} aria-hidden="true" />
              {t("pages:admin.roles.actions.reset", "Reset")}
            </Button>
            <Button
              className={HEADER_ACTION_BUTTON_CLASS}
              disabled={!hasDirtyChanges || saving}
              size="sm"
              variant="secondary"
              onClick={() => runDebouncedAction("header-discard-role-draft", discardChanges)}
            >
              <RotateCcw size={16} aria-hidden="true" />
              {t("pages:admin.roles.actions.discard", "Discard")}
            </Button>
            <Button
              className={HEADER_ACTION_BUTTON_CLASS}
              disabled={!hasDirtyChanges || saving}
              size="sm"
              variant="primary"
              aria-busy={saving}
              onClick={() => runDebouncedAction("header-save-role-draft", () => {
                void saveDraft();
              })}
            >
              <Save size={16} aria-hidden="true" />
              {saving ? t("pages:admin.roles.actions.saving", "Saving...") : t("pages:admin.roles.actions.save", "Save")}
            </Button>
            <Button
              className={cn(
                HEADER_ACTION_BUTTON_CLASS,
                "border",
                deleteConfirmRoleId === selectedRole?.id
                  ? "border-[var(--color-error)] bg-[var(--color-error)] text-[var(--color-on-error)] hover:bg-[var(--color-on-error-container)] hover:text-[var(--color-on-error)]"
                  : "border-[var(--color-error)] bg-[var(--color-error-container)] text-[var(--color-on-error-container)] hover:bg-[var(--color-error-container)]/80",
              )}
              size="sm"
              type="button"
              variant="ghost"
              disabled={!selectedRole || selectedRole.kind === "System"}
              onClick={() => runDebouncedAction(
                `header-delete-role-${selectedRole?.id ?? "none"}-${deleteConfirmRoleId === selectedRole?.id ? "confirm" : "arm"}`,
                deleteSelectedRole,
              )}
            >
              <Trash2 size={14} aria-hidden="true" />
              {deleteConfirmRoleId === selectedRole?.id ? t("pages:admin.roles.actions.confirm", "Confirm") : t("pages:admin.roles.actions.delete", "Delete")}
            </Button>
          </>
        }
      />
      <SurfaceCard className="overflow-hidden p-0">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] p-5">
          <div>
            <h2 className="text-xl font-semibold">{t("pages:admin.roles.matrixTitle", "Platform Permissions")}</h2>
            <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{t("pages:admin.roles.matrixDescription", "Search roles, focus domains, and edit permission grants directly from the matrix.")}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <SearchField ariaLabel={t("pages:admin.roles.search", "Search roles")} placeholder={t("pages:admin.roles.searchPlaceholder", "Search roles, codes, or descriptions...")} value={query} onChange={setQuery} />
            <SelectInput aria-label={t("pages:admin.roles.roleTypeFilter", "Filter by role type")} className="h-9 min-w-36" value={roleType} onValueChange={(event) => setRoleType(event as RoleTypeFilter)}>
              {ROLE_TYPE_OPTIONS.map((option) => <SelectItem key={option}>{t(`pages:admin.roles.roleTypes.${option}`, option)}</SelectItem>)}
            </SelectInput>
            <SelectInput aria-label={t("pages:admin.roles.domainFilter", "Filter by permission domain")} className="h-9 min-w-40" value={domainFilter} onValueChange={(event) => setDomainFilter(event!)}>
              <SelectItem value={ALL_DOMAINS}>{t("pages:admin.roles.allDomains", ALL_DOMAINS)}</SelectItem>
              {allDomains.map((domain) => <SelectItem key={domain} value={domain}>{domain}</SelectItem>)}
            </SelectInput>
            <ToggleSwitch checked={showChangedOnly} label={t("pages:admin.roles.showChanged", "Show changed only")} onCheckedChange={setShowChangedOnly} />
          </div>
        </div>
        {filteredRoles.length ? (
          <div className="overflow-x-auto">
            <table className="w-full min-w-[1080px] border-collapse text-left text-sm">
              <thead>
                <tr className="border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)]">
                  <th className="sticky left-0 z-10 min-w-[280px] border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-[11px] uppercase text-[var(--color-on-surface-variant)]">{t("pages:admin.roles.role", "Role")}</th>
                  {visibleDomains.map((domain) => (
                    <th className="border-r border-[var(--color-outline-variant)] p-4 text-center text-[11px] uppercase text-[var(--color-on-surface-variant)]" key={domain}>
                      {domain}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {filteredRoles.map((role) => {
                  const roleMembers = membersByRoleCode.get(role.code) ?? [];
                  const selected = role.id === selectedRoleId;
                  return (
                    <tr className={cn("border-b border-[var(--color-outline-variant)]", selected ? "bg-[var(--color-surface-container-low)]" : "hover:bg-[var(--color-hover-surface)]")} key={role.id}>
                      <td
                        className={cn(
                          "sticky left-0 border-r border-[var(--color-outline-variant)] p-4 align-top",
                          selected ? "bg-[var(--color-surface-container-low)]" : "bg-white",
                        )}
                      >
                        <div className="flex flex-wrap items-center gap-2">
                          {selected && role.kind === "Custom" ? (
                            <TextInput
                              aria-label={`Edit role name for ${role.code}`}
                              autoFocus={renameFocusRoleId === role.id}
                              className="h-9 min-w-0 flex-1"
                              value={role.name}
                              onBlur={() => {
                                if (renameFocusRoleId === role.id) {
                                  setRenameFocusRoleId(null);
                                }
                              }}
                              onChange={(event) => renameRole(role.id, event.target.value)}
                              onClick={(event) => event.stopPropagation()}
                            />
                          ) : (
                            <button
                              className="min-w-0 text-left"
                              type="button"
                              onClick={() => runDebouncedAction(`matrix-select-${role.id}`, () => selectRole(role.id), 180)}
                            >
                              <span className="font-medium">{role.name}</span>
                            </button>
                          )}
                          <button
                            className="min-w-0 text-left"
                            type="button"
                            onClick={() => runDebouncedAction(`matrix-select-${role.id}`, () => selectRole(role.id), 180)}
                          >
                            <Badge tone={role.kind === "System" ? "info" : "outline"}>{role.kind}</Badge>
                          </button>
                          {dirtyIds.has(role.id) ? <Badge tone="warning">{t("pages:admin.roles.unsaved", "Unsaved")}</Badge> : null}
                        </div>
                        <button className="mt-2 w-full text-left" type="button" onClick={() => runDebouncedAction(`matrix-select-${role.id}`, () => selectRole(role.id), 180)}>
                          <div className="mt-1 font-mono text-[11px] uppercase text-[var(--color-on-surface-variant)]">{role.code}</div>
                          <div className="mt-2 text-[13px] text-[var(--color-on-surface-variant)]">{role.description}</div>
                          <div className="mt-3 flex flex-wrap items-center gap-3 text-[12px] text-[var(--color-on-surface-variant)]">
                            <span>{t("pages:admin.roles.usersLinked", "{{count}} users linked", { count: roleMembers.length })}</span>
                            <span>{t("pages:admin.roles.updated", "Updated {{value}}", { value: formatTimestamp(role.updatedAt) })}</span>
                          </div>
                        </button>
                      </td>
                      {visibleDomains.map((domain) => {
                        return (
                          <td className="border-r border-[var(--color-outline-variant)] p-4 align-top" key={`${role.id}-${domain}`}>
                            <div className="flex flex-wrap justify-center gap-1">
                              {adminPermissionCodes.map((code) => {
                                const active = role.permissions[domain]?.includes(code) ?? false;
                                return (
                                  <button
                                    aria-label={`Toggle ${code} for ${role.name} on ${domain}`}
                                    aria-pressed={active}
                                    className={cn(
                                      "flex size-8 items-center justify-center rounded-lg border text-[11px] font-semibold transition-colors",
                                      active
                                        ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-on-primary)]"
                                        : "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-high)] text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]",
                                    )}
                                    key={`${role.id}-${domain}-${code}`}
                                    type="button"
                                    onClick={() => runDebouncedAction(
                                      `matrix-toggle-${role.id}-${domain}-${code}`,
                                      () => togglePermission(role.id, domain, code),
                                    )}
                                  >
                                    {code}
                                  </button>
                                );
                              })}
                            </div>
                          </td>
                        );
                      })}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="p-10 text-center">
            <p className="text-sm text-[var(--color-on-surface-variant)]">{t("pages:admin.roles.empty", "No roles match the current filters.")}</p>
            <div className="mt-4 flex justify-center gap-2">
              <Button variant="secondary" onClick={() => runDebouncedAction("empty-clear-role-filters", clearFilters)}>
                {t("pages:admin.roles.actions.clear", "Clear Filters")}
              </Button>
              <Button onClick={() => runDebouncedAction("empty-add-role", addCustomRole)}>
                <Plus size={16} aria-hidden="true" />
                {t("pages:admin.roles.actions.new", "New")}
              </Button>
            </div>
          </div>
        )}
        <div className="flex flex-wrap items-center justify-between gap-4 border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-[13px] text-[var(--color-on-surface-variant)]">
          <div className="flex flex-wrap items-center gap-3">
            {adminPermissionLegend.map((item) => <span key={item}>{item}</span>)}
          </div>
          <span>{dataSource === "api" ? t("pages:admin.roles.persistence.api", "Saved drafts persist through the backend role catalog.") : t("pages:admin.roles.persistence.mock", "Saved drafts persist in this browser for mock mode.")}</span>
        </div>
      </SurfaceCard>
    </>
  );
}

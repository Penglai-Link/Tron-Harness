import {
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  ChevronDown,
  ChevronRight,
  FolderTree,
  Plus,
  RefreshCcw,
  RotateCcw,
  Save,
  Search,
  Trash2,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import { cn } from "@/shared/lib/utils";
import {
  Field,
  PageHeader,
  SelectInput,
  SegmentedTabs,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  ToggleSwitch,
  TronShell,
  useToast,
} from "@/shared";

import {
  type AdminPage,
  archiveWorkspace,
  createWorkspace,
  fetchWorkspaceDetail,
  fetchWorkspaceEffectiveUsers,
  fetchWorkspaceRoleAccess,
  fetchWorkspaceTree,
  fetchWorkspaceUserAccess,
  moveWorkspace,
  replaceWorkspaceRoleAccess,
  replaceWorkspaceUserAccess,
  restoreWorkspace,
  updateWorkspace,
} from "@/domain/admin/api";
import {
  adminUserStatusOptions,
  getAdminUserRoleLabel,
  getStatusTone,
} from "@/domain/admin/constants";
import {
  useRoleList,
  useUserList,
} from "@/domain/admin/hooks";
import {
  archiveWorkspaceRecord,
  createWorkspaceRecord,
  getWorkspaceDetail,
  getWorkspaceEffectiveUsers,
  getWorkspaceRoleAccess,
  getWorkspaceTree,
  getWorkspaceUserAccess,
  moveWorkspaceRecord,
  replaceWorkspaceRoleAccessRecords,
  replaceWorkspaceUserAccessRecords,
  restoreWorkspaceRecord,
  updateWorkspaceRecord,
} from "@/domain/admin/mock";
import type {
  UserRoleCode,
  WorkspaceDetailRecord,
  WorkspaceEffectiveUserRecord,
  WorkspaceRoleAccessRecord,
  WorkspaceTreeNodeRecord,
  WorkspaceUserAccessRecord,
  WorkspaceStatus,
} from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

type WorkspaceStatusFilter = WorkspaceStatus | "All";
type WorkspaceInspectorTab = "overview" | "access" | "create";

interface WorkspaceFormState {
  description: string;
  name: string;
}

interface WorkspaceCreateFormState {
  description: string;
  name: string;
  parentWorkspaceCode: string;
}

const ROOT_PARENT_VALUE = "__ROOT__";
const ACCESS_PAGE_SIZE = 100;
const workspaceInspectorTabs = [
  { label: "Overview", value: "overview" },
  { label: "Access", value: "access" },
  { label: "Create", value: "create" },
] satisfies ReadonlyArray<{ label: string; value: WorkspaceInspectorTab }>;

function SearchField({
  onChange,
  onClear,
  placeholder,
  value,
}: {
  onChange: (value: string) => void;
  onClear?: () => void;
  placeholder: string;
  value: string;
}) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput
        aria-label="Search workspaces"
        className={cn("h-9 w-full pl-9", value ? "pr-9" : undefined)}
        placeholder={placeholder}
        role="searchbox"
        type="text"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
      {value ? (
        <Button
          aria-label="Clear workspace search"
          className="absolute right-1 top-1/2 size-7 -translate-y-1/2"
          size="icon"
          type="button"
          variant="ghost"
          onClick={() => {
            onChange("");
            onClear?.();
          }}
        >
          <X size={14} aria-hidden="true" />
        </Button>
      ) : null}
    </div>
  );
}

function DetailField({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
      <p className="mt-1 break-words text-sm font-medium text-[var(--color-on-surface)]">{value}</p>
    </div>
  );
}

function InspectorStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl bg-[var(--color-surface-container-low)] px-4 py-3">
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
      <p className="mt-2 text-base font-semibold text-[var(--color-on-surface)]">{value}</p>
    </div>
  );
}

function emptyPage<T>(): AdminPage<T> {
  return {
    current: 1,
    size: ACCESS_PAGE_SIZE,
    total: 0,
    records: [],
  };
}

function pagedRecords<T>(records: T[]): AdminPage<T> {
  return {
    current: 1,
    size: ACCESS_PAGE_SIZE,
    total: records.length,
    records,
  };
}

function formatTimestamp(value: string | null) {
  if (!value) return "Never";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function rootValueToParent(value: string) {
  return value === ROOT_PARENT_VALUE ? null : value;
}

function parentToRootValue(value: string | null) {
  return value ?? ROOT_PARENT_VALUE;
}

function flattenWorkspaceTree(records: WorkspaceTreeNodeRecord[]): WorkspaceTreeNodeRecord[] {
  return records.flatMap((record) => [record, ...flattenWorkspaceTree(record.children)]);
}

function matchesWorkspaceSearch(record: WorkspaceTreeNodeRecord, search: string) {
  if (!search.trim()) return true;
  const searchable = [
    record.workspaceCode,
    record.name,
    record.description ?? "",
    record.status,
  ].join(" ").toLowerCase();
  return searchable.includes(search.trim().toLowerCase());
}

function filterWorkspaceTree(records: WorkspaceTreeNodeRecord[], search: string, status: WorkspaceStatusFilter): WorkspaceTreeNodeRecord[] {
  return records
    .map((record) => {
      const children = filterWorkspaceTree(record.children, search, status);
      const statusMatches = status === "All" || record.status === status;
      const searchMatches = matchesWorkspaceSearch(record, search);
      if (!(statusMatches && searchMatches) && !children.length) return null;

      return {
        ...record,
        children,
      };
    })
    .filter((record): record is WorkspaceTreeNodeRecord => record !== null);
}

function collectWorkspaceCodes(records: WorkspaceTreeNodeRecord[]): string[] {
  return flattenWorkspaceTree(records).filter((record) => record.children.length > 0).map((record) => record.workspaceCode);
}

function findWorkspaceNode(records: WorkspaceTreeNodeRecord[], workspaceCode: string | null) {
  if (!workspaceCode) return null;
  return flattenWorkspaceTree(records).find((record) => record.workspaceCode === workspaceCode) ?? null;
}

function workspaceSubtreeSpan(record: WorkspaceTreeNodeRecord): number {
  return record.children.length ? 1 + Math.max(...record.children.map(workspaceSubtreeSpan)) : 0;
}

function workspaceDescendantCodes(record: WorkspaceTreeNodeRecord) {
  const codes = new Set<string>([record.workspaceCode]);
  record.children.forEach((child) => {
    workspaceDescendantCodes(child).forEach((value) => codes.add(value));
  });
  return codes;
}

function sourceTone(source: WorkspaceEffectiveUserRecord["source"]) {
  if (source === "DirectAndRole") return "success";
  if (source === "Role") return "warning";
  return "accent";
}

function TreeNode({
  expandedCodes,
  onPrepareCreate,
  onSelect,
  onToggle,
  record,
  selectedWorkspaceCode,
}: {
  expandedCodes: Set<string>;
  onPrepareCreate: (parentWorkspaceCode: string | null) => void;
  onSelect: (workspaceCode: string) => void;
  onToggle: (workspaceCode: string) => void;
  record: WorkspaceTreeNodeRecord;
  selectedWorkspaceCode: string | null;
}) {
  const hasChildren = record.children.length > 0;
  const expanded = expandedCodes.has(record.workspaceCode);
  const selected = selectedWorkspaceCode === record.workspaceCode;

  return (
    <div className="relative">
      <div
        className={cn(
          "group flex min-h-12 items-center gap-1 rounded-lg px-1.5 py-1.5 transition-colors",
          selected
            ? "bg-[var(--color-primary)]/10 ring-1 ring-[var(--color-primary)]/35"
            : "hover:bg-[var(--color-hover-surface)]",
        )}
      >
        {hasChildren ? (
          <Button
            size="icon"
            variant="ghost"
            aria-label={`Toggle ${record.workspaceCode} children`}
            className="size-7 shrink-0 text-[var(--color-on-surface-variant)]"
            onClick={() => onToggle(record.workspaceCode)}
          >
            {expanded ? <ChevronDown size={15} aria-hidden="true" /> : <ChevronRight size={15} aria-hidden="true" />}
          </Button>
        ) : (
          <div className="h-7 w-1 shrink-0" aria-hidden="true" />
        )}
        <button
          className="min-w-0 flex-1 rounded-md px-1 py-0.5 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
          type="button"
          aria-current={selected ? "true" : undefined}
          aria-label={`Select workspace ${record.workspaceCode}`}
          onClick={() => onSelect(record.workspaceCode)}
        >
          <div className="flex min-w-0 items-center gap-2">
            <span className="truncate text-sm font-medium text-[var(--color-on-surface)]">{record.name}</span>
            <span className="inline-flex shrink-0 items-center gap-1 text-[11px] font-medium text-[var(--color-on-surface-variant)]">
              <span className={cn(
                "size-1.5 rounded-full",
                record.status === "Active" ? "bg-[var(--color-success)]" : "bg-[var(--color-warning)]",
              )} />
              {record.status}
            </span>
          </div>
          <div className="mt-0.5 flex min-w-0 items-center gap-2 text-[12px] text-[var(--color-on-surface-variant)]">
            <span className="truncate font-mono">{record.workspaceCode}</span>
            {record.childCount ? <span className="shrink-0">· {record.childCount} {record.childCount === 1 ? "child" : "children"}</span> : null}
          </div>
        </button>
        <Button
          size="icon"
          variant="ghost"
          aria-label={`Create child workspace under ${record.workspaceCode}`}
          className="size-7 shrink-0 text-[var(--color-on-surface-variant)] opacity-0 transition-opacity focus-visible:opacity-100 group-hover:opacity-100"
          onClick={() => onPrepareCreate(record.workspaceCode)}
        >
          <Plus size={15} aria-hidden="true" />
        </Button>
      </div>
      {hasChildren && expanded ? (
        <div className="ml-4 mt-1 space-y-1 border-l border-[var(--color-outline-variant)] pl-3">
          {record.children.map((child) => (
            <TreeNode
              expandedCodes={expandedCodes}
              key={child.workspaceCode}
              onPrepareCreate={onPrepareCreate}
              onSelect={onSelect}
              onToggle={onToggle}
              record={child}
              selectedWorkspaceCode={selectedWorkspaceCode}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function WorkspaceManagementPage() {
  const toast = useToast();
  const isApiMode = getServiceDataSource("admin") === "api";
  const users = useUserList({ current: 1, size: 200, status: "All" });
  const roleCatalog = useRoleList();
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<WorkspaceStatusFilter>("Active");
  const [catalogTree, setCatalogTree] = useState<WorkspaceTreeNodeRecord[]>([]);
  const [workspaceTree, setWorkspaceTree] = useState<WorkspaceTreeNodeRecord[]>([]);
  const [selectedWorkspaceCode, setSelectedWorkspaceCode] = useState<string | null>(null);
  const [searchClearVersion, setSearchClearVersion] = useState(0);
  const [inspectorTab, setInspectorTab] = useState<WorkspaceInspectorTab>("overview");
  const [detail, setDetail] = useState<WorkspaceDetailRecord | null>(null);
  const [detailForm, setDetailForm] = useState<WorkspaceFormState>({ name: "", description: "" });
  const [moveParentWorkspaceCode, setMoveParentWorkspaceCode] = useState(ROOT_PARENT_VALUE);
  const [createForm, setCreateForm] = useState<WorkspaceCreateFormState>({
    name: "",
    description: "",
    parentWorkspaceCode: ROOT_PARENT_VALUE,
  });
  const [userAccessPage, setUserAccessPage] = useState<AdminPage<WorkspaceUserAccessRecord>>(emptyPage());
  const [roleAccessRecords, setRoleAccessRecords] = useState<WorkspaceRoleAccessRecord[]>([]);
  const [effectiveUserPage, setEffectiveUserPage] = useState<AdminPage<WorkspaceEffectiveUserRecord>>(emptyPage());
  const [directUserDrafts, setDirectUserDrafts] = useState<WorkspaceUserAccessRecord[]>([]);
  const [roleDrafts, setRoleDrafts] = useState<WorkspaceRoleAccessRecord[]>([]);
  const [nextDirectUserCode, setNextDirectUserCode] = useState("");
  const [nextRoleCode, setNextRoleCode] = useState<UserRoleCode | "">("");
  const [expandedCodes, setExpandedCodes] = useState<string[]>([]);
  const [workspaceDataVersion, setWorkspaceDataVersion] = useState(0);
  const [treeLoading, setTreeLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [creatingWorkspace, setCreatingWorkspace] = useState(false);
  const [savingMetadata, setSavingMetadata] = useState(false);
  const [movingWorkspace, setMovingWorkspace] = useState(false);
  const [lifecycleLoading, setLifecycleLoading] = useState(false);
  const [savingUsers, setSavingUsers] = useState(false);
  const [savingRoles, setSavingRoles] = useState(false);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(searchInput.trim());
    }, 280);
    return () => window.clearTimeout(timer);
  }, [searchInput]);

  useEffect(() => {
    let cancelled = false;
    const controller = new AbortController();

    async function loadTree() {
      setTreeLoading(true);

      try {
        const workspaceTreeQuery = {
          includeArchived: statusFilter === "All" ? true : undefined,
          status: statusFilter === "All" ? undefined : statusFilter,
        };
        const nextCatalogTree = isApiMode
          ? await fetchWorkspaceTree(workspaceTreeQuery, controller.signal)
          : getWorkspaceTree();
        if (cancelled) return;

        const nextWorkspaceTree = isApiMode
          ? (
            debouncedSearch || statusFilter !== "All"
              ? await fetchWorkspaceTree({
                ...workspaceTreeQuery,
                search: debouncedSearch,
              }, controller.signal)
              : nextCatalogTree
          )
          : filterWorkspaceTree(nextCatalogTree, debouncedSearch, statusFilter);
        if (cancelled) return;

        setCatalogTree(nextCatalogTree);
        setWorkspaceTree(nextWorkspaceTree);
        setExpandedCodes(collectWorkspaceCodes(nextWorkspaceTree));
        if (!selectedWorkspaceCode) {
          setSelectedWorkspaceCode(nextCatalogTree[0]?.workspaceCode ?? null);
        } else if (!findWorkspaceNode(nextCatalogTree, selectedWorkspaceCode)) {
          setSelectedWorkspaceCode(nextCatalogTree[0]?.workspaceCode ?? null);
        }
      } catch (error) {
        if (controller.signal.aborted || cancelled) return;
        const message = error instanceof Error ? error.message : "Workspace tree could not be loaded";
        toast.error(message);
      } finally {
        if (!cancelled) setTreeLoading(false);
      }
    }

    void loadTree();
    return () => {
      cancelled = true;
      controller.abort();
    };
  }, [debouncedSearch, isApiMode, searchClearVersion, statusFilter, toast, workspaceDataVersion]);

  useEffect(() => {
    if (!selectedWorkspaceCode) {
      setDetail(null);
      setDetailForm({ name: "", description: "" });
      setMoveParentWorkspaceCode(ROOT_PARENT_VALUE);
      setUserAccessPage(emptyPage());
      setRoleAccessRecords([]);
      setEffectiveUserPage(emptyPage());
      setDirectUserDrafts([]);
      setRoleDrafts([]);
      return;
    }

    let cancelled = false;
    const controller = new AbortController();
    const workspaceCode = selectedWorkspaceCode;

    async function loadWorkspaceDetailAndAccess() {
      setDetailLoading(true);

      try {
        const [nextDetail, nextUserAccessPage, nextRoleAccessRecords, nextEffectiveUserPage] = await Promise.all([
          isApiMode ? fetchWorkspaceDetail(workspaceCode, controller.signal) : Promise.resolve(getWorkspaceDetail(workspaceCode)),
          isApiMode ? fetchWorkspaceUserAccess(workspaceCode, { current: 1, size: ACCESS_PAGE_SIZE }, controller.signal) : Promise.resolve(pagedRecords(getWorkspaceUserAccess(workspaceCode))),
          isApiMode ? fetchWorkspaceRoleAccess(workspaceCode, controller.signal) : Promise.resolve(getWorkspaceRoleAccess(workspaceCode)),
          isApiMode ? fetchWorkspaceEffectiveUsers(workspaceCode, { current: 1, size: ACCESS_PAGE_SIZE }, controller.signal) : Promise.resolve(pagedRecords(getWorkspaceEffectiveUsers(workspaceCode))),
        ]);
        if (cancelled || controller.signal.aborted) return;

        if (!nextDetail) {
          setDetail(null);
          setUserAccessPage(emptyPage());
          setRoleAccessRecords([]);
          setEffectiveUserPage(emptyPage());
          setDirectUserDrafts([]);
          setRoleDrafts([]);
          return;
        }

        setDetail(nextDetail);
        setDetailForm({
          name: nextDetail.name,
          description: nextDetail.description ?? "",
        });
        setMoveParentWorkspaceCode(parentToRootValue(nextDetail.parentWorkspaceCode));
        setUserAccessPage(nextUserAccessPage);
        setRoleAccessRecords(nextRoleAccessRecords);
        setEffectiveUserPage(nextEffectiveUserPage);
        setDirectUserDrafts(nextUserAccessPage.records);
        setRoleDrafts(nextRoleAccessRecords);
      } catch (error) {
        if (cancelled || controller.signal.aborted) return;
        const message = error instanceof Error ? error.message : "Workspace detail could not be loaded";
        toast.error(message);
      } finally {
        if (!cancelled) setDetailLoading(false);
      }
    }

    void loadWorkspaceDetailAndAccess();
    return () => {
      cancelled = true;
      controller.abort();
    };
  }, [isApiMode, selectedWorkspaceCode, toast, workspaceDataVersion]);

  const selectedTreeNode = useMemo(
    () => findWorkspaceNode(catalogTree, selectedWorkspaceCode),
    [catalogTree, selectedWorkspaceCode],
  );
  const expandedCodeSet = useMemo(() => new Set(expandedCodes), [expandedCodes]);
  const activeUsers = useMemo(
    () => [...users.records].filter((user) => user.status === "Active").sort((left, right) => left.displayName.localeCompare(right.displayName)),
    [users.records],
  );
  const availableDirectUsers = useMemo(() => {
    const selectedUserCodes = new Set(directUserDrafts.map((record) => record.userCode));
    return activeUsers.filter((user) => !selectedUserCodes.has(user.userCode));
  }, [activeUsers, directUserDrafts]);
  const availableRoleOptions = useMemo(() => {
    const selectedRoleCodes = new Set(roleDrafts.map((record) => record.roleCode));
    return roleCatalog
      .map((role) => role.code as UserRoleCode)
      .filter((roleCode) => !selectedRoleCodes.has(roleCode));
  }, [roleCatalog, roleDrafts]);
  const createParentOptions = useMemo(
    () => flattenWorkspaceTree(catalogTree).filter((record) => record.status === "Active" && record.depth < 3),
    [catalogTree],
  );
  const moveParentOptions = useMemo(() => {
    if (!selectedTreeNode) return [] as WorkspaceTreeNodeRecord[];
    const blockedCodes = workspaceDescendantCodes(selectedTreeNode);
    const subtreeSpan = workspaceSubtreeSpan(selectedTreeNode);
    return flattenWorkspaceTree(catalogTree).filter((record) => {
      if (record.status !== "Active") return false;
      if (blockedCodes.has(record.workspaceCode)) return false;
      return record.depth + 1 + subtreeSpan <= 3;
    });
  }, [catalogTree, selectedTreeNode]);
  const movePreviewDepth = useMemo(() => {
    const parentWorkspaceCode = rootValueToParent(moveParentWorkspaceCode);
    const parent = parentWorkspaceCode ? findWorkspaceNode(catalogTree, parentWorkspaceCode) : null;
    return (parent?.depth ?? 0) + 1;
  }, [catalogTree, moveParentWorkspaceCode]);

  function refreshWorkspaceData() {
    setWorkspaceDataVersion((value) => value + 1);
  }

  function clearWorkspaceSearch() {
    setDebouncedSearch("");
    setSearchClearVersion((value) => value + 1);
  }

  function handleSelectWorkspace(workspaceCode: string) {
    setSelectedWorkspaceCode(workspaceCode);
    setInspectorTab("overview");
  }

  function prepareCreate(parentWorkspaceCode: string | null) {
    setCreateForm({
      name: "",
      description: "",
      parentWorkspaceCode: parentToRootValue(parentWorkspaceCode),
    });
    setInspectorTab("create");
  }

  function toggleExpanded(workspaceCode: string) {
    setExpandedCodes((current) => (
      current.includes(workspaceCode)
        ? current.filter((value) => value !== workspaceCode)
        : [...current, workspaceCode]
    ));
  }

  async function handleCreateWorkspace() {
    if (creatingWorkspace) return;

    setCreatingWorkspace(true);
    try {
      const payload = {
        name: createForm.name.trim(),
        description: createForm.description.trim() || null,
        parentWorkspaceCode: rootValueToParent(createForm.parentWorkspaceCode),
      };
      const created = isApiMode
        ? await createWorkspace(payload)
        : createWorkspaceRecord(payload);

      setSelectedWorkspaceCode(created.workspaceCode);
      setCreateForm({
        name: "",
        description: "",
        parentWorkspaceCode: parentToRootValue(payload.parentWorkspaceCode),
      });
      setInspectorTab("overview");
      refreshWorkspaceData();
      toast.success(`Workspace ${created.workspaceCode} created`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Workspace could not be created";
      toast.error(message);
    } finally {
      setCreatingWorkspace(false);
    }
  }

  async function handleSaveMetadata() {
    if (!detail || savingMetadata) return;

    setSavingMetadata(true);
    try {
      const updated = isApiMode
        ? await updateWorkspace(detail.workspaceCode, {
          name: detailForm.name.trim(),
          description: detailForm.description.trim() || null,
        })
        : updateWorkspaceRecord(detail.workspaceCode, {
          name: detailForm.name.trim(),
          description: detailForm.description.trim() || null,
        });

      setDetail(updated);
      setDetailForm({
        name: updated.name,
        description: updated.description ?? "",
      });
      refreshWorkspaceData();
      toast.success(`Workspace ${updated.workspaceCode} updated`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Workspace metadata could not be saved";
      toast.error(message);
    } finally {
      setSavingMetadata(false);
    }
  }

  async function handleMoveWorkspace() {
    if (!detail || movingWorkspace) return;

    setMovingWorkspace(true);
    try {
      const moved = isApiMode
        ? await moveWorkspace(detail.workspaceCode, {
          parentWorkspaceCode: rootValueToParent(moveParentWorkspaceCode),
        })
        : moveWorkspaceRecord(detail.workspaceCode, rootValueToParent(moveParentWorkspaceCode));

      setDetail(moved);
      setMoveParentWorkspaceCode(parentToRootValue(moved.parentWorkspaceCode));
      refreshWorkspaceData();
      toast.success(`Workspace ${moved.workspaceCode} moved`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Workspace could not be moved";
      toast.error(message);
    } finally {
      setMovingWorkspace(false);
    }
  }

  async function handleWorkspaceLifecycle(action: "archive" | "restore") {
    if (!detail || lifecycleLoading) return;

    setLifecycleLoading(true);
    try {
      const result = action === "archive"
        ? (
          isApiMode
            ? await archiveWorkspace(detail.workspaceCode)
            : archiveWorkspaceRecord(detail.workspaceCode)
        )
        : (
          isApiMode
            ? await restoreWorkspace(detail.workspaceCode)
            : restoreWorkspaceRecord(detail.workspaceCode)
        );

      refreshWorkspaceData();
      toast.success(`${result.workspaceCode ?? detail.workspaceCode} ${action}d`);
    } catch (error) {
      const message = error instanceof Error ? error.message : `Workspace could not be ${action}d`;
      toast.error(message);
    } finally {
      setLifecycleLoading(false);
    }
  }

  function addDirectUserGrant() {
    if (!nextDirectUserCode) return;
    const user = activeUsers.find((record) => record.userCode === nextDirectUserCode);
    if (!user) return;

    setDirectUserDrafts((current) => [
      ...current,
      {
        userCode: user.userCode,
        displayName: user.displayName,
        email: user.email,
        isOwner: false,
        grantedAt: new Date().toISOString(),
      },
    ]);
    setNextDirectUserCode("");
  }

  function addRoleGrant() {
    if (!nextRoleCode) return;
    const roleName = roleCatalog.find((role) => role.code === nextRoleCode)?.name ?? getAdminUserRoleLabel(nextRoleCode);
    setRoleDrafts((current) => [
      ...current,
      {
        roleCode: nextRoleCode,
        roleName,
        matchedUserCount: activeUsers.filter((user) => user.roleCodes.includes(nextRoleCode)).length,
        grantedAt: new Date().toISOString(),
      },
    ]);
    setNextRoleCode("");
  }

  async function handleSaveDirectUsers() {
    if (!detail || savingUsers) return;

    setSavingUsers(true);
    try {
      const nextUsers = isApiMode
        ? await replaceWorkspaceUserAccess(detail.workspaceCode, directUserDrafts.map((record) => ({
          userCode: record.userCode,
          isOwner: record.isOwner,
        })))
        : pagedRecords(replaceWorkspaceUserAccessRecords(detail.workspaceCode, directUserDrafts.map((record) => ({
          userCode: record.userCode,
          isOwner: record.isOwner,
        }))));

      setUserAccessPage(nextUsers);
      setDirectUserDrafts(nextUsers.records);
      refreshWorkspaceData();
      toast.success("Direct user access updated");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Direct user access could not be saved";
      toast.error(message);
    } finally {
      setSavingUsers(false);
    }
  }

  async function handleSaveRoleAccess() {
    if (!detail || savingRoles) return;

    setSavingRoles(true);
    try {
      const nextRoles = isApiMode
        ? await replaceWorkspaceRoleAccess(detail.workspaceCode, roleDrafts.map((record) => ({
          roleCode: record.roleCode,
        })))
        : replaceWorkspaceRoleAccessRecords(detail.workspaceCode, roleDrafts.map((record) => ({
          roleCode: record.roleCode,
        })));

      setRoleAccessRecords(nextRoles);
      setRoleDrafts(nextRoles);
      refreshWorkspaceData();
      toast.success("Role access updated");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Role access could not be saved";
      toast.error(message);
    } finally {
      setSavingRoles(false);
    }
  }

  return (
    <TronShell activeItem="Workspace" searchPlaceholder="Search workspaces...">
      <PageHeader
        title="Workspace Management"
        actions={(
          <>
            <Button variant="secondary" disabled={treeLoading} aria-busy={treeLoading} onClick={refreshWorkspaceData}>
              <RefreshCcw size={16} aria-hidden="true" />
              {treeLoading ? "Refreshing..." : "Refresh"}
            </Button>
            <Button variant="secondary" onClick={() => prepareCreate(null)}>
              <Plus size={16} aria-hidden="true" />
              New
            </Button>
          </>
        )}
      />

      <div className="grid gap-5 xl:grid-cols-[minmax(18rem,24rem)_minmax(0,1fr)]">
        <SurfaceCard className="flex min-h-[28rem] flex-col space-y-4 xl:sticky xl:top-20 xl:h-[calc(100vh-6.5rem)]">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Workspace Tree</h2>
            </div>
            {treeLoading ? <Badge tone="accent">Loading</Badge> : null}
          </div>
          <div className="grid gap-3">
            <SearchField placeholder="Filter workspaces..." value={searchInput} onChange={setSearchInput} onClear={clearWorkspaceSearch} />
            <SelectInput aria-label="Filter workspaces by status" value={statusFilter} onChange={(event) => setStatusFilter(event.target.value as WorkspaceStatusFilter)}>
              <option value="All">All Status</option>
              {adminUserStatusOptions.map((status) => (
                <option key={status} value={status}>{status}</option>
              ))}
            </SelectInput>
          </div>
          <div className="min-h-0 flex-1 space-y-1 overflow-y-auto p-1">
            {workspaceTree.length ? workspaceTree.map((record) => (
              <TreeNode
                expandedCodes={expandedCodeSet}
                key={record.workspaceCode}
                onPrepareCreate={prepareCreate}
                onSelect={handleSelectWorkspace}
                onToggle={toggleExpanded}
                record={record}
                selectedWorkspaceCode={selectedWorkspaceCode}
              />
            )) : (
              <div className="rounded-xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                No workspaces match the current filters.
              </div>
            )}
          </div>
        </SurfaceCard>

        <div className="space-y-5">
          <SurfaceCard className="space-y-6">
            <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
              <div className="min-w-0">
                <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">Workspace Studio</p>
                <div className="mt-2 flex flex-wrap items-center gap-3">
                  <h2 className="text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">
                    {inspectorTab === "create" ? "Create Workspace" : detail ? detail.name : "Select a Workspace"}
                  </h2>
                  {detail && inspectorTab !== "create" ? <Badge tone={getStatusTone(detail.status)}>{detail.status}</Badge> : null}
                </div>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                {detail ? (
                  <Button variant="secondary" onClick={() => prepareCreate(detail.workspaceCode)}>
                    <Plus size={16} aria-hidden="true" />
                    New
                  </Button>
                ) : null}
              </div>
            </div>

            <div className="flex flex-col gap-4 border-t border-[var(--color-outline-variant)] pt-4 xl:flex-row xl:items-center xl:justify-between">
              <SegmentedTabs value={inspectorTab} onValueChange={(value) => setInspectorTab(value as WorkspaceInspectorTab)} items={workspaceInspectorTabs} />
              {detail && inspectorTab !== "create" ? (
                <div className="grid gap-3 sm:grid-cols-3 xl:max-w-[28rem] xl:grid-cols-3">
                  <InspectorStat label="Direct Users" value={String(detail.directUserCount)} />
                  <InspectorStat label="Role Grants" value={String(detail.roleGrantCount)} />
                  <InspectorStat label="Effective Users" value={String(detail.effectiveUserCount)} />
                </div>
              ) : null}
            </div>

            {inspectorTab === "create" ? (
              <section className="space-y-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <h3 className="text-base font-semibold text-[var(--color-on-surface)]">Create Workspace</h3>
                    <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">Use the selected tree context when you need a child, or keep it root-level for a new top branch.</p>
                  </div>
                  {rootValueToParent(createForm.parentWorkspaceCode)
                    ? <Badge tone="accent">Child of {rootValueToParent(createForm.parentWorkspaceCode)}</Badge>
                    : <Badge tone="neutral">Root workspace</Badge>}
                </div>
                <div className="grid gap-4 md:grid-cols-2">
                  <Field label="Parent workspace">
                    <SelectInput value={createForm.parentWorkspaceCode} onChange={(event) => setCreateForm((current) => ({ ...current, parentWorkspaceCode: event.target.value }))}>
                      <option value={ROOT_PARENT_VALUE}>Root workspace</option>
                      {createParentOptions.map((record) => (
                        <option key={record.workspaceCode} value={record.workspaceCode}>
                          {record.name} ({record.workspaceCode}) · Level {record.depth}
                        </option>
                      ))}
                    </SelectInput>
                  </Field>
                  <Field label="Workspace name">
                    <TextInput
                      aria-label="Workspace name"
                      value={createForm.name}
                      onChange={(event) => setCreateForm((current) => ({ ...current, name: event.target.value }))}
                    />
                  </Field>
                  <Field label="Description">
                    <TextArea
                      aria-label="Workspace description"
                      className="min-h-24"
                      value={createForm.description}
                      onChange={(event) => setCreateForm((current) => ({ ...current, description: event.target.value }))}
                    />
                  </Field>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Button disabled={creatingWorkspace} aria-busy={creatingWorkspace} onClick={() => void handleCreateWorkspace()}>
                    <Plus size={16} aria-hidden="true" />
                    {creatingWorkspace ? "Saving..." : "Save"}
                  </Button>
                  {selectedWorkspaceCode ? (
                    <Button variant="secondary" onClick={() => prepareCreate(selectedWorkspaceCode)}>
                      <Plus size={16} aria-hidden="true" />
                      Use
                    </Button>
                  ) : null}
                </div>
              </section>
            ) : detailLoading && !detail ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                Loading workspace detail...
              </div>
            ) : !detail ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                Select a workspace from the tree to open the inspector.
              </div>
            ) : inspectorTab === "overview" ? (
              <section className="space-y-5">
                <div className="grid gap-5 xl:grid-cols-[minmax(0,1.2fr)_minmax(18rem,0.8fr)]">
                  <div className="space-y-4">
                    <Field label="Name">
                      <TextInput value={detailForm.name} onChange={(event) => setDetailForm((current) => ({ ...current, name: event.target.value }))} />
                    </Field>
                    <Field label="Move parent" hint={`Resulting workspace depth: level ${movePreviewDepth}.`}>
                      <SelectInput value={moveParentWorkspaceCode} onChange={(event) => setMoveParentWorkspaceCode(event.target.value)}>
                        <option value={ROOT_PARENT_VALUE}>Root workspace</option>
                        {moveParentOptions.map((record) => (
                          <option key={record.workspaceCode} value={record.workspaceCode}>
                            {record.name} ({record.workspaceCode}) · Level {record.depth}
                          </option>
                        ))}
                      </SelectInput>
                    </Field>
                    <Field label="Description">
                      <TextArea
                        className="min-h-40"
                        value={detailForm.description}
                        onChange={(event) => setDetailForm((current) => ({ ...current, description: event.target.value }))}
                      />
                    </Field>
                  </div>
                  <div className="space-y-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                    <div className="grid gap-4 sm:grid-cols-2">
                      <DetailField label="Workspace Code" value={detail.workspaceCode} />
                      <DetailField label="Depth" value={`Level ${detail.depth}`} />
                      <DetailField label="Created" value={formatTimestamp(detail.createdAt)} />
                      <DetailField label="Archived" value={formatTimestamp(detail.archivedAt)} />
                    </div>
                    <DetailField label="Lineage" value={detail.lineage.map((item) => item.name).join(" / ")} />
                    <div>
                      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Child Workspaces</p>
                      <div className="mt-3 flex flex-wrap gap-2">
                        {detail.children.length ? detail.children.map((child) => (
                          <button
                            className="rounded-full border border-[var(--color-outline-variant)] px-3 py-1 text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
                            type="button"
                            key={child.workspaceCode}
                            onClick={() => handleSelectWorkspace(child.workspaceCode)}
                          >
                            {child.name} ({child.workspaceCode})
                          </button>
                        )) : <span className="text-sm text-[var(--color-on-surface-variant)]">No child workspaces.</span>}
                      </div>
                    </div>
                  </div>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Button disabled={savingMetadata || detail.status === "Archived"} aria-busy={savingMetadata} onClick={() => void handleSaveMetadata()}>
                    <Save size={16} aria-hidden="true" />
                    {savingMetadata ? "Saving..." : "Save"}
                  </Button>
                  <Button variant="secondary" disabled={movingWorkspace || detail.status === "Archived"} aria-busy={movingWorkspace} onClick={() => void handleMoveWorkspace()}>
                    <FolderTree size={16} aria-hidden="true" />
                    {movingWorkspace ? "Moving..." : "Move"}
                  </Button>
                  {detail.status === "Active" ? (
                    <Button variant="secondary" disabled={lifecycleLoading} aria-busy={lifecycleLoading} onClick={() => void handleWorkspaceLifecycle("archive")}>
                      <Trash2 size={16} aria-hidden="true" />
                      {lifecycleLoading ? "Archiving..." : "Archive"}
                    </Button>
                  ) : (
                    <Button variant="secondary" disabled={lifecycleLoading} aria-busy={lifecycleLoading} onClick={() => void handleWorkspaceLifecycle("restore")}>
                      <RotateCcw size={16} aria-hidden="true" />
                      {lifecycleLoading ? "Restoring..." : "Restore"}
                    </Button>
                  )}
                </div>
              </section>
            ) : (
              <section className="space-y-6">
                <div className="space-y-6">
                  <section className="space-y-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                    <div className="flex flex-wrap items-end justify-between gap-4">
                      <div className="min-w-[16rem] flex-1">
                        <Field label="Direct User Access">
                          <SelectInput aria-label="Add direct user access" value={nextDirectUserCode} onChange={(event) => setNextDirectUserCode(event.target.value)}>
                            <option value="">Select a user</option>
                            {availableDirectUsers.map((user) => (
                              <option key={user.userCode} value={user.userCode}>
                                {user.displayName} ({user.userCode})
                              </option>
                            ))}
                          </SelectInput>
                        </Field>
                      </div>
                      <Button className="shrink-0" variant="secondary" disabled={!nextDirectUserCode} onClick={addDirectUserGrant}>
                        <Plus size={16} aria-hidden="true" />
                        Add
                      </Button>
                    </div>

                    <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)] bg-white">
                      <table className="w-full min-w-[28rem] border-collapse text-left text-sm">
                        <thead>
                          <tr className="bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                            <th className="px-4 py-3">User</th>
                            <th className="px-4 py-3">Email</th>
                            <th className="px-4 py-3">Owner</th>
                            <th className="px-4 py-3">Granted</th>
                            <th className="px-4 py-3 text-right">Actions</th>
                          </tr>
                        </thead>
                        <tbody>
                          {directUserDrafts.length ? directUserDrafts.map((record) => (
                            <tr className="border-t border-[var(--color-outline-variant)]" key={record.userCode}>
                              <td className="px-4 py-3">
                                <div className="font-medium text-[var(--color-on-surface)]">{record.displayName}</div>
                                <div className="text-[12px] text-[var(--color-on-surface-variant)]">{record.userCode}</div>
                              </td>
                              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{record.email}</td>
                              <td className="px-4 py-3">
                                <ToggleSwitch
                                  checked={record.isOwner}
                                  label={record.isOwner ? "Owner" : "Member"}
                                  onCheckedChange={(checked) => setDirectUserDrafts((current) => current.map((item) => (
                                    item.userCode === record.userCode ? { ...item, isOwner: checked } : item
                                  )))}
                                />
                              </td>
                              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{formatTimestamp(record.grantedAt)}</td>
                              <td className="px-4 py-3 text-right">
                                <Button
                                  size="icon"
                                  variant="ghost"
                                  aria-label={`Remove direct grant for ${record.displayName}`}
                                  onClick={() => setDirectUserDrafts((current) => current.filter((item) => item.userCode !== record.userCode))}
                                >
                                  <Trash2 size={16} aria-hidden="true" />
                                </Button>
                              </td>
                            </tr>
                          )) : (
                            <tr>
                              <td className="px-4 py-6 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={5}>
                                No direct user grants configured.
                              </td>
                            </tr>
                          )}
                        </tbody>
                      </table>
                    </div>

                    <div className="flex justify-end">
                      <Button disabled={savingUsers} aria-busy={savingUsers} onClick={() => void handleSaveDirectUsers()}>
                        <Save size={16} aria-hidden="true" />
                        {savingUsers ? "Saving..." : "Save"}
                      </Button>
                    </div>
                  </section>

                  <section className="space-y-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                    <div className="flex flex-wrap items-end justify-between gap-4">
                      <div className="min-w-[16rem] flex-1">
                        <Field label="Role Access">
                          <SelectInput aria-label="Add workspace role access" value={nextRoleCode} onChange={(event) => setNextRoleCode(event.target.value as UserRoleCode | "")}>
                            <option value="">Select a role</option>
                            {availableRoleOptions.map((roleCode) => (
                              <option key={roleCode} value={roleCode}>
                                {getAdminUserRoleLabel(roleCode)} ({roleCode})
                              </option>
                            ))}
                          </SelectInput>
                        </Field>
                      </div>
                      <Button className="shrink-0" variant="secondary" disabled={!nextRoleCode} onClick={addRoleGrant}>
                        <Plus size={16} aria-hidden="true" />
                        Add
                      </Button>
                    </div>

                    <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)] bg-white">
                      <table className="w-full min-w-[28rem] border-collapse text-left text-sm">
                        <thead>
                          <tr className="bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                            <th className="px-4 py-3">Role</th>
                            <th className="px-4 py-3">Matched Users</th>
                            <th className="px-4 py-3">Granted</th>
                            <th className="px-4 py-3 text-right">Actions</th>
                          </tr>
                        </thead>
                        <tbody>
                          {roleDrafts.length ? roleDrafts.map((record) => (
                            <tr className="border-t border-[var(--color-outline-variant)]" key={record.roleCode}>
                              <td className="px-4 py-3">
                                <div className="font-medium text-[var(--color-on-surface)]">{record.roleName}</div>
                                <div className="text-[12px] text-[var(--color-on-surface-variant)]">{record.roleCode}</div>
                              </td>
                              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{record.matchedUserCount}</td>
                              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{formatTimestamp(record.grantedAt)}</td>
                              <td className="px-4 py-3 text-right">
                                <Button
                                  size="icon"
                                  variant="ghost"
                                  aria-label={`Remove role grant for ${record.roleCode}`}
                                  onClick={() => setRoleDrafts((current) => current.filter((item) => item.roleCode !== record.roleCode))}
                                >
                                  <Trash2 size={16} aria-hidden="true" />
                                </Button>
                              </td>
                            </tr>
                          )) : (
                            <tr>
                              <td className="px-4 py-6 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={4}>
                                No role grants configured.
                              </td>
                            </tr>
                          )}
                        </tbody>
                      </table>
                    </div>

                    <div className="flex justify-end">
                      <Button disabled={savingRoles} aria-busy={savingRoles} onClick={() => void handleSaveRoleAccess()}>
                        <Save size={16} aria-hidden="true" />
                        {savingRoles ? "Saving..." : "Save"}
                      </Button>
                    </div>
                  </section>
                </div>

                <section className="space-y-4 border-t border-[var(--color-outline-variant)] pt-5">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <h4 className="text-base font-semibold text-[var(--color-on-surface)]">Effective Access Users</h4>
                      <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">The derived union of direct-user and role-based access for the selected workspace.</p>
                    </div>
                    <Badge tone="accent">{effectiveUserPage.total} users</Badge>
                  </div>
                  <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)]">
                    <table className="w-full min-w-[52rem] border-collapse text-left text-sm">
                      <thead>
                        <tr className="bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                          <th className="px-4 py-3">User</th>
                          <th className="px-4 py-3">Email</th>
                          <th className="px-4 py-3">Access Source</th>
                          <th className="px-4 py-3">Matched Roles</th>
                          <th className="px-4 py-3">Owner</th>
                        </tr>
                      </thead>
                      <tbody>
                        {effectiveUserPage.records.length ? effectiveUserPage.records.map((row) => (
                          <tr className="border-t border-[var(--color-outline-variant)]" key={row.userCode}>
                            <td className="px-4 py-3">
                              <div className="font-medium text-[var(--color-on-surface)]">{row.displayName}</div>
                              <div className="text-[12px] text-[var(--color-on-surface-variant)]">{row.userCode}</div>
                            </td>
                            <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{row.email}</td>
                            <td className="px-4 py-3"><Badge tone={sourceTone(row.source)}>{row.source}</Badge></td>
                            <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{row.matchedRoleCodes.length ? row.matchedRoleCodes.map(getAdminUserRoleLabel).join(", ") : "Direct grant only"}</td>
                            <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{row.isOwner ? "Yes" : "No"}</td>
                          </tr>
                        )) : (
                          <tr>
                            <td className="px-4 py-8 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={5}>
                              No effective users match the current grants.
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </div>
                  <TablePagination
                    endItem={effectiveUserPage.total ? effectiveUserPage.total : 0}
                    itemLabel="users"
                    page={effectiveUserPage.current}
                    pageCount={1}
                    pageSize={effectiveUserPage.size}
                    startItem={effectiveUserPage.total ? 1 : 0}
                    totalItems={effectiveUserPage.total}
                  />
                </section>
              </section>
            )}
          </SurfaceCard>
        </div>
      </div>
    </TronShell>
  );
}

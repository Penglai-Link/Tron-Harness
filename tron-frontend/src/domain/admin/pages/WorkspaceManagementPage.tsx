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
  Trash2,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import { cn } from "@/shared/lib/utils";
import {
  DetailField,
  Field,
  PageHeader,
  SearchField,
  SelectInput,
  SegmentedTabs,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  ToggleSwitch,
  useLocaleFormatters,
  useLocaleText,
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
import { SelectItem } from "@/components/ui/select";

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

function InspectorStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl bg-[var(--color-surface-container-low)] px-4 py-3">
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
      <p className="mt-2 text-base font-semibold text-[var(--color-on-surface)]">{value}</p>
    </div>
  );
}

function emptyPage<T>(current = 1, size = ACCESS_PAGE_SIZE): AdminPage<T> {
  return {
    current,
    size,
    total: 0,
    records: [],
  };
}

function pagedRecords<T>(records: T[], current = 1, size = ACCESS_PAGE_SIZE): AdminPage<T> {
  const startIndex = Math.max(0, (current - 1) * size);
  return {
    current,
    size,
    total: records.length,
    records: records.slice(startIndex, startIndex + size),
  };
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
  return "info";
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
  const pt = useLocaleText();
  const { formatNumber } = useLocaleFormatters();
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
            aria-label={pt("Toggle {{code}} children", { code: record.workspaceCode })}
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
          aria-label={pt("Select workspace {{code}}", { code: record.workspaceCode })}
          onClick={() => onSelect(record.workspaceCode)}
        >
          <div className="flex min-w-0 items-center gap-[12px]">
            <span className="truncate text-sm font-medium text-[var(--color-on-surface)]">{record.name}</span>
            <span className="inline-flex shrink-0 items-center gap-1 text-[11px] font-medium text-[var(--color-on-surface-variant)]">
              <span className={cn(
                "size-1.5 rounded-full",
                record.status === "Active" ? "bg-[var(--color-success)]" : "bg-[var(--color-warning)]",
              )} />
              {pt(record.status)}
            </span>
          </div>
          <div className="mt-0.5 flex min-w-0 items-center gap-[12px] text-[12px] text-[var(--color-on-surface-variant)]">
            <span className="truncate font-mono">{record.workspaceCode}</span>
            {record.childCount ? <span className="shrink-0">· {record.childCount === 1 ? pt("{{count}} child", { count: formatNumber(record.childCount) }) : pt("{{count}} children", { count: formatNumber(record.childCount) })}</span> : null}
          </div>
        </button>
        <Button
          size="icon"
          variant="ghost"
          aria-label={pt("Create child workspace under {{code}}", { code: record.workspaceCode })}
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
  const pt = useLocaleText();
  const { formatDateTime, formatNumber } = useLocaleFormatters();
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
  const [effectiveUserCurrent, setEffectiveUserCurrent] = useState(1);
  const [effectiveUserSize, setEffectiveUserSize] = useState(ACCESS_PAGE_SIZE);
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
  const localizedInspectorTabs = useMemo(
    () => workspaceInspectorTabs.map((item) => ({ ...item, label: pt(item.label) })),
    [pt],
  );
  const formatTimestamp = (value: string | null) => (value ? formatDateTime(value, {
    dateStyle: "medium",
    timeStyle: "short",
  }) : pt("Never"));

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
        const message = error instanceof Error ? error.message : pt("Workspace tree could not be loaded");
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
    setEffectiveUserCurrent(1);
  }, [selectedWorkspaceCode]);

  useEffect(() => {
    if (!selectedWorkspaceCode) {
      setDetail(null);
      setDetailForm({ name: "", description: "" });
      setMoveParentWorkspaceCode(ROOT_PARENT_VALUE);
      setUserAccessPage(emptyPage());
      setRoleAccessRecords([]);
      setEffectiveUserPage(emptyPage(effectiveUserCurrent, effectiveUserSize));
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
          isApiMode
            ? fetchWorkspaceEffectiveUsers(workspaceCode, { current: effectiveUserCurrent, size: effectiveUserSize }, controller.signal)
            : Promise.resolve(pagedRecords(getWorkspaceEffectiveUsers(workspaceCode), effectiveUserCurrent, effectiveUserSize)),
        ]);
        if (cancelled || controller.signal.aborted) return;

        if (!nextDetail) {
          setDetail(null);
          setUserAccessPage(emptyPage());
          setRoleAccessRecords([]);
          setEffectiveUserPage(emptyPage(effectiveUserCurrent, effectiveUserSize));
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
        const message = error instanceof Error ? error.message : pt("Workspace detail could not be loaded");
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
  }, [effectiveUserCurrent, effectiveUserSize, isApiMode, selectedWorkspaceCode, toast, workspaceDataVersion]);

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
  const effectiveUserPageCount = Math.max(1, Math.ceil(effectiveUserPage.total / effectiveUserPage.size));
  const effectiveUserStartItem = effectiveUserPage.total ? (effectiveUserPage.current - 1) * effectiveUserPage.size + 1 : 0;
  const effectiveUserEndItem = effectiveUserStartItem
    ? effectiveUserStartItem + effectiveUserPage.records.length - 1
    : 0;
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
      toast.success(pt("Workspace {{code}} created", { code: created.workspaceCode }));
    } catch (error) {
      const message = error instanceof Error ? error.message : pt("Workspace could not be created");
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
      toast.success(pt("Workspace {{code}} updated", { code: updated.workspaceCode }));
    } catch (error) {
      const message = error instanceof Error ? error.message : pt("Workspace metadata could not be saved");
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
      toast.success(pt("Workspace {{code}} moved", { code: moved.workspaceCode }));
    } catch (error) {
      const message = error instanceof Error ? error.message : pt("Workspace could not be moved");
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
      toast.success(action === "archive"
        ? pt("Workspace {{code}} archived", { code: result.workspaceCode ?? detail.workspaceCode })
        : pt("Workspace {{code}} restored", { code: result.workspaceCode ?? detail.workspaceCode }));
    } catch (error) {
      const message = error instanceof Error ? error.message : action === "archive" ? pt("Workspace could not be archived") : pt("Workspace could not be restored");
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
      toast.success(pt("Direct user access updated"));
    } catch (error) {
      const message = error instanceof Error ? error.message : pt("Direct user access could not be saved");
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
      toast.success(pt("Role access updated"));
    } catch (error) {
      const message = error instanceof Error ? error.message : pt("Role access could not be saved");
      toast.error(message);
    } finally {
      setSavingRoles(false);
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        className="mb-5 shrink-0"
        title={pt("Workspace Management")}
        actions={(
          <>
            <Button variant="secondary" disabled={treeLoading} aria-busy={treeLoading} onClick={refreshWorkspaceData}>
              <RefreshCcw size={16} aria-hidden="true" />
              {treeLoading ? pt("Refreshing...") : pt("Refresh")}
            </Button>
            <Button variant="secondary" onClick={() => prepareCreate(null)}>
              <Plus size={16} aria-hidden="true" />
              {pt("Root")}
            </Button>
          </>
        )}
      />

      <div className="grid min-h-0 flex-1 gap-5 xl:grid-cols-[minmax(18rem,24rem)_minmax(0,1fr)]">
        <SurfaceCard className="flex min-h-[28rem] flex-col space-y-4 xl:min-h-0 xl:h-full">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pt("Workspace Tree")}</h2>
            </div>
            {treeLoading ? <Badge tone="info">{pt("Loading")}</Badge> : null}
          </div>
          <div className="grid gap-3">
            <SearchField ariaLabel={pt("Search workspaces")} clearLabel={pt("Clear workspace search")} placeholder={pt("Filter workspaces...")} value={searchInput} onChange={setSearchInput} onClear={clearWorkspaceSearch} />
            <SelectInput aria-label={pt("Filter workspaces by status")} value={statusFilter} onValueChange={(event) => setStatusFilter(event as WorkspaceStatusFilter)}>
              <SelectItem value="All">{pt("All Status")}</SelectItem>
              {adminUserStatusOptions.map((status) => (
                <SelectItem key={status} value={status}>{pt(status)}</SelectItem>
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
                {pt("No workspaces match the current filters.")}
              </div>
            )}
          </div>
        </SurfaceCard>

        <div className="min-h-0 space-y-5 xl:overflow-hidden">
          <SurfaceCard className="min-h-[28rem] space-y-6 xl:h-full xl:min-h-0 xl:overflow-y-auto">
            <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
              <div className="min-w-0">
                <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">{pt("Workspace Studio")}</p>
                <div className="mt-2 flex flex-wrap items-center gap-3">
                  <h2 className="text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">
                    {inspectorTab === "create" ? pt("Create Workspace") : detail ? detail.name : pt("Select a Workspace")}
                  </h2>
                  {detail && inspectorTab !== "create" ? <Badge tone={getStatusTone(detail.status)}>{pt(detail.status)}</Badge> : null}
                </div>
              </div>
              <div className="flex flex-wrap items-center gap-[12px]">
                {inspectorTab === "create" ? (
                  <Button disabled={creatingWorkspace} aria-busy={creatingWorkspace} onClick={() => void handleCreateWorkspace()}>
                    <Plus size={16} aria-hidden="true" />
                    {creatingWorkspace ? pt("Saving...") : pt("Save")}
                  </Button>
                ) : null}
                {detail && inspectorTab !== "create" ? (
                  <Button variant="secondary" onClick={() => prepareCreate(detail.workspaceCode)}>
                    <Plus size={16} aria-hidden="true" />
                    {pt("Child")}
                  </Button>
                ) : null}
              </div>
            </div>

            <div className="space-y-4 border-t border-[var(--color-outline-variant)] pt-4">
              <SegmentedTabs value={inspectorTab} onValueChange={(value) => setInspectorTab(value as WorkspaceInspectorTab)} items={localizedInspectorTabs} />
              {detail && inspectorTab !== "create" ? (
                <div className="grid gap-3 sm:grid-cols-3">
                  <InspectorStat label={pt("Direct Users")} value={formatNumber(detail.directUserCount)} />
                  <InspectorStat label={pt("Role Grants")} value={formatNumber(detail.roleGrantCount)} />
                  <InspectorStat label={pt("Effective Users")} value={formatNumber(detail.effectiveUserCount)} />
                </div>
              ) : null}
            </div>

            {inspectorTab === "create" ? (
              <section className="space-y-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <h3 className="text-base font-semibold text-[var(--color-on-surface)]">{pt("Create Workspace")}</h3>
                    <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{pt("Use the selected tree context when you need a child, or keep it root-level for a new top branch.")}</p>
                  </div>
                  {rootValueToParent(createForm.parentWorkspaceCode)
                    ? <Badge tone="info">{pt("Child of {{code}}", { code: rootValueToParent(createForm.parentWorkspaceCode) ?? "" })}</Badge>
                    : <Badge tone="info">{pt("Root workspace")}</Badge>}
                </div>
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="min-w-0">
                    <Field label={pt("Parent workspace")}>
                      <SelectInput
                        className="w-full min-w-0"
                        value={createForm.parentWorkspaceCode}
                        onValueChange={(event) => setCreateForm((current) => ({ ...current, parentWorkspaceCode: event! }))}
                      >
                        <SelectItem value={ROOT_PARENT_VALUE}>{pt("Root workspace")}</SelectItem>
                        {createParentOptions.map((record) => (
                          <SelectItem key={record.workspaceCode} value={record.workspaceCode}>
                            {pt("{{name}} ({{code}}) · Level {{depth}}", { code: record.workspaceCode, depth: formatNumber(record.depth), name: record.name })}
                          </SelectItem>
                        ))}
                      </SelectInput>
                    </Field>
                  </div>
                  <div className="min-w-0">
                    <Field label={pt("Workspace name")}>
                      <TextInput
                        aria-label={pt("Workspace name")}
                        className="w-full"
                        value={createForm.name}
                        onChange={(event) => setCreateForm((current) => ({ ...current, name: event.target.value }))}
                      />
                    </Field>
                  </div>
                  <div className="min-w-0">
                    <Field label={pt("Description")}>
                      <TextArea
                        aria-label={pt("Workspace description")}
                        className="min-h-24 w-full"
                        value={createForm.description}
                        onChange={(event) => setCreateForm((current) => ({ ...current, description: event.target.value }))}
                      />
                    </Field>
                  </div>
                </div>
              </section>
            ) : detailLoading && !detail ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                {pt("Loading workspace detail...")}
              </div>
            ) : !detail ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                {pt("Select a workspace from the tree to open the inspector.")}
              </div>
            ) : inspectorTab === "overview" ? (
              <section className="space-y-5">
                <div className="grid gap-5 xl:grid-cols-[minmax(0,1.2fr)_minmax(18rem,0.8fr)]">
                  <div className="space-y-4">
                    <Field label={pt("Name")}>
                      <TextInput className="w-full" value={detailForm.name} onChange={(event) => setDetailForm((current) => ({ ...current, name: event.target.value }))} />
                    </Field>
                    <Field label={pt("Move parent")} hint={pt("Resulting workspace depth: level {{depth}}.", { depth: formatNumber(movePreviewDepth) })}>
                      <SelectInput className="w-full min-w-0" value={moveParentWorkspaceCode} onValueChange={(event) => setMoveParentWorkspaceCode(event!)}>
                        <SelectItem value={ROOT_PARENT_VALUE}>{pt("Root workspace")}</SelectItem>
                        {moveParentOptions.map((record) => (
                          <SelectItem key={record.workspaceCode} value={record.workspaceCode}>
                            {pt("{{name}} ({{code}}) · Level {{depth}}", { code: record.workspaceCode, depth: formatNumber(record.depth), name: record.name })}
                          </SelectItem>
                        ))}
                      </SelectInput>
                    </Field>
                    <Field label={pt("Description")}>
                      <TextArea
                        className="min-h-40 w-full"
                        value={detailForm.description}
                        onChange={(event) => setDetailForm((current) => ({ ...current, description: event.target.value }))}
                      />
                    </Field>
                  </div>
                  <div className="space-y-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                    <div className="grid gap-4 sm:grid-cols-2">
                      <DetailField label={pt("Workspace Code")} value={detail.workspaceCode} />
                      <DetailField label={pt("Depth")} value={pt("Level {{depth}}", { depth: formatNumber(detail.depth) })} />
                      <DetailField label={pt("Created")} value={formatTimestamp(detail.createdAt)} />
                      <DetailField label={pt("Archived")} value={formatTimestamp(detail.archivedAt)} />
                    </div>
                    <DetailField label={pt("Path")} value={detail.path.map((item) => item.name).join(" / ")} />
                    <div>
                      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pt("Child Workspaces")}</p>
                      <div className="mt-3 flex flex-wrap gap-[12px]">
                        {detail.children.length ? detail.children.map((child) => (
                          <button
                            className="rounded-full border border-[var(--color-outline-variant)] px-3 py-1 text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
                            type="button"
                            key={child.workspaceCode}
                            onClick={() => handleSelectWorkspace(child.workspaceCode)}
                          >
                            {child.name} ({child.workspaceCode})
                          </button>
                        )) : <span className="text-sm text-[var(--color-on-surface-variant)]">{pt("No child workspaces.")}</span>}
                      </div>
                    </div>
                  </div>
                </div>
                <div className="flex flex-wrap items-center gap-[12px]">
                  <Button disabled={savingMetadata || detail.status === "Archived"} aria-busy={savingMetadata} onClick={() => void handleSaveMetadata()}>
                    <Save size={16} aria-hidden="true" />
                    {savingMetadata ? pt("Saving...") : pt("Save")}
                  </Button>
                  <Button variant="secondary" disabled={movingWorkspace || detail.status === "Archived"} aria-busy={movingWorkspace} onClick={() => void handleMoveWorkspace()}>
                    <FolderTree size={16} aria-hidden="true" />
                    {movingWorkspace ? pt("Moving...") : pt("Move")}
                  </Button>
                  {detail.status === "Active" ? (
                    <Button variant="secondary" disabled={lifecycleLoading} aria-busy={lifecycleLoading} onClick={() => void handleWorkspaceLifecycle("archive")}>
                      <Trash2 size={16} aria-hidden="true" />
                      {lifecycleLoading ? pt("Archiving...") : pt("Archive")}
                    </Button>
                  ) : (
                    <Button variant="secondary" disabled={lifecycleLoading} aria-busy={lifecycleLoading} onClick={() => void handleWorkspaceLifecycle("restore")}>
                      <RotateCcw size={16} aria-hidden="true" />
                      {lifecycleLoading ? pt("Restoring...") : pt("Restore")}
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
                        <Field label={pt("Direct User Access")}>
                          <SelectInput aria-label={pt("Add direct user access")} value={nextDirectUserCode} onValueChange={(event) => setNextDirectUserCode(event!)}>
                            <SelectItem value="">{pt("Select a user")}</SelectItem>
                            {availableDirectUsers.map((user) => (
                              <SelectItem key={user.userCode} value={user.userCode}>
                                {user.displayName} ({user.userCode})
                              </SelectItem>
                            ))}
                          </SelectInput>
                        </Field>
                      </div>
                      <Button className="shrink-0" variant="secondary" disabled={!nextDirectUserCode} onClick={addDirectUserGrant}>
                        <Plus size={16} aria-hidden="true" />
                        {pt("Add")}
                      </Button>
                    </div>

                    <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)] bg-white">
                      <table className="w-full min-w-[28rem] border-collapse text-left text-sm">
                        <thead>
                          <tr className="bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                            <th className="px-4 py-3">{pt("User")}</th>
                            <th className="px-4 py-3">{pt("Email")}</th>
                            <th className="px-4 py-3">{pt("Owner")}</th>
                            <th className="px-4 py-3">{pt("Granted")}</th>
                            <th className="px-4 py-3 text-right">{pt("Actions")}</th>
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
                                  label={record.isOwner ? pt("Owner") : pt("Member")}
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
                                  aria-label={pt("Remove direct grant for {{name}}", { name: record.displayName })}
                                  onClick={() => setDirectUserDrafts((current) => current.filter((item) => item.userCode !== record.userCode))}
                                >
                                  <Trash2 size={16} aria-hidden="true" />
                                </Button>
                              </td>
                            </tr>
                          )) : (
                            <tr>
                              <td className="px-4 py-6 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={5}>
                                {pt("No direct user grants configured.")}
                              </td>
                            </tr>
                          )}
                        </tbody>
                      </table>
                    </div>

                    <div className="flex justify-end">
                      <Button disabled={savingUsers} aria-busy={savingUsers} onClick={() => void handleSaveDirectUsers()}>
                        <Save size={16} aria-hidden="true" />
                        {savingUsers ? pt("Saving...") : pt("Save")}
                      </Button>
                    </div>
                  </section>

                  <section className="space-y-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                    <div className="flex flex-wrap items-end justify-between gap-4">
                      <div className="min-w-[16rem] flex-1">
                        <Field label={pt("Role Access")}>
                          <SelectInput aria-label={pt("Add workspace role access")} value={nextRoleCode} onValueChange={(value) => setNextRoleCode(value as UserRoleCode | "")}>
                            <SelectItem value="">{pt("Select a role")}</SelectItem>
                            {availableRoleOptions.map((roleCode) => (
                              <SelectItem key={roleCode} value={roleCode}>
                                {pt("{{name}} ({{code}})", { code: roleCode, name: pt(getAdminUserRoleLabel(roleCode)) })}
                              </SelectItem>
                            ))}
                          </SelectInput>
                        </Field>
                      </div>
                      <Button className="shrink-0" variant="secondary" disabled={!nextRoleCode} onClick={addRoleGrant}>
                        <Plus size={16} aria-hidden="true" />
                        {pt("Add")}
                      </Button>
                    </div>

                    <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)] bg-white">
                      <table className="w-full min-w-[28rem] border-collapse text-left text-sm">
                        <thead>
                          <tr className="bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                            <th className="px-4 py-3">{pt("Role")}</th>
                            <th className="px-4 py-3">{pt("Matched Users")}</th>
                            <th className="px-4 py-3">{pt("Granted")}</th>
                            <th className="px-4 py-3 text-right">{pt("Actions")}</th>
                          </tr>
                        </thead>
                        <tbody>
                          {roleDrafts.length ? roleDrafts.map((record) => (
                            <tr className="border-t border-[var(--color-outline-variant)]" key={record.roleCode}>
                              <td className="px-4 py-3">
                                <div className="font-medium text-[var(--color-on-surface)]">{pt(record.roleName)}</div>
                                <div className="text-[12px] text-[var(--color-on-surface-variant)]">{record.roleCode}</div>
                              </td>
                              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{formatNumber(record.matchedUserCount)}</td>
                              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{formatTimestamp(record.grantedAt)}</td>
                              <td className="px-4 py-3 text-right">
                                <Button
                                  size="icon"
                                  variant="ghost"
                                  aria-label={pt("Remove role grant for {{code}}", { code: record.roleCode })}
                                  onClick={() => setRoleDrafts((current) => current.filter((item) => item.roleCode !== record.roleCode))}
                                >
                                  <Trash2 size={16} aria-hidden="true" />
                                </Button>
                              </td>
                            </tr>
                          )) : (
                            <tr>
                              <td className="px-4 py-6 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={4}>
                                {pt("No role grants configured.")}
                              </td>
                            </tr>
                          )}
                        </tbody>
                      </table>
                    </div>

                    <div className="flex justify-end">
                      <Button disabled={savingRoles} aria-busy={savingRoles} onClick={() => void handleSaveRoleAccess()}>
                        <Save size={16} aria-hidden="true" />
                        {savingRoles ? pt("Saving...") : pt("Save")}
                      </Button>
                    </div>
                  </section>
                </div>

                <section className="space-y-4 border-t border-[var(--color-outline-variant)] pt-5">
                  <h4 className="text-base font-semibold text-[var(--color-on-surface)]">{pt("Effective Access Users")}</h4>
                  <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)]">
                    <table className="w-full min-w-[52rem] border-collapse text-left text-sm">
                      <thead>
                        <tr className="bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                          <th className="px-4 py-3">{pt("User")}</th>
                          <th className="px-4 py-3">{pt("Email")}</th>
                          <th className="px-4 py-3">{pt("Access Source")}</th>
                          <th className="px-4 py-3">{pt("Matched Roles")}</th>
                          <th className="px-4 py-3">{pt("Owner")}</th>
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
                            <td className="px-4 py-3"><Badge tone={sourceTone(row.source)}>{pt(row.source)}</Badge></td>
                            <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{row.matchedRoleCodes.length ? row.matchedRoleCodes.map((roleCode) => pt(getAdminUserRoleLabel(roleCode))).join(", ") : pt("Direct grant only")}</td>
                            <td className="px-4 py-3 text-[var(--color-on-surface-variant)]">{row.isOwner ? pt("Yes") : pt("No")}</td>
                          </tr>
                        )) : (
                          <tr>
                            <td className="px-4 py-8 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={5}>
                              {pt("No effective users match the current grants.")}
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </div>
                  <TablePagination
                    endItem={effectiveUserEndItem}
                    itemLabel={pt("users")}
                    page={effectiveUserPage.current}
                    pageCount={effectiveUserPageCount}
                    pageSize={effectiveUserPage.size}
                    startItem={effectiveUserStartItem}
                    totalItems={effectiveUserPage.total}
                    onPageChange={setEffectiveUserCurrent}
                    onPageSizeChange={(pageSize) => {
                      setEffectiveUserCurrent(1);
                      setEffectiveUserSize(pageSize);
                    }}
                  />
                </section>
              </section>
            )}
          </SurfaceCard>
        </div>
      </div>
    </div>
  );
}

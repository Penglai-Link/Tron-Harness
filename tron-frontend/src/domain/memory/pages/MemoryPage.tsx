import { useEffect, useMemo, useState } from "react";
import {
  Boxes,
  Check,
  ChevronDown,
  Database,
  MoreHorizontal,
  Plus,
  RefreshCcw,
  UserRound,
} from "lucide-react";
import { useNavigate } from "react-router";
import { getCoreRowModel, useReactTable } from "@tanstack/react-table";

import { useAuthSession } from "@/domain/auth";
import { MemoryRecordSummary } from "@/domain/memory/components";
import {
  defaultMemoryFilters,
  memoryInjectableOptions,
  memoryReviewOptions,
  memoryStatusOptions,
  type MemoryFilterState,
} from "@/domain/memory/constants";
import {
  archiveMemoryRecord,
  useMemoryFragmentList,
  useMemoryTargets,
} from "@/domain/memory";
import type {
  MemoryRecord,
  MemoryScopeKind,
  MemorySearchTarget,
} from "@/domain/memory/model";
import { archiveMemoryFragment } from "@/domain/memory/mock/data";
import {
  Badge,
  ConfirmDialog,
  Field,
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  useConsoleShellConfig,
} from "@/shared";
import { Button } from "@/components/ui/button";
import {
  DataTable,
  DataTableProps,
  type TableColumnDef,
} from "@/components/table";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { cn } from "@/shared/lib/utils";
import { SelectItem } from "@/components/ui/select";

const visibleTargetLimit = 4;

function MemoryTargetIcon({ scope }: { scope: MemoryScopeKind }) {
  const iconProps = {
    "aria-hidden": true,
    size: 18,
    strokeWidth: 2,
  };

  if (scope === "space") return <Database {...iconProps} />;
  if (scope === "user") return <UserRound {...iconProps} />;
  return <Boxes {...iconProps} />;
}

function getScopeLabel(scope: MemoryScopeKind) {
  if (scope === "space") return "Space";
  if (scope === "user") return "User";
  return "Domain";
}

function getTypeTone(memory: MemoryRecord) {
  if (memory.memoryStatus === "DISABLED") return "error";
  if (memory.scope === "space") return "success";
  if (memory.scope === "domain") return "warning";
  if (memory.scope === "user") return "info";
  return "outline";
}

function normalizeSearchValue(value: string) {
  return value.trim().toLowerCase();
}

function recordMatchesQuery(memory: MemoryRecord, query: string) {
  const normalizedQuery = normalizeSearchValue(query);
  if (!normalizedQuery) return true;

  return [
    memory.title,
    memory.text,
    memory.memoryCode,
    memory.memoryType,
    memory.memoryStatus,
    memory.reviewStatus,
    memory.semanticKey,
    memory.sourceLabel,
    memory.targetLabel,
    ...memory.tags,
  ].some((value) => value.toLowerCase().includes(normalizedQuery));
}

function recordMatchesFilters(
  memory: MemoryRecord,
  filters: MemoryFilterState,
) {
  if (filters.status !== "all" && memory.memoryStatus !== filters.status)
    return false;
  if (
    filters.reviewStatus !== "all" &&
    memory.reviewStatus !== filters.reviewStatus
  )
    return false;
  if (filters.injectable === "injectable" && !memory.injectable) return false;
  if (filters.injectable === "not-injectable" && memory.injectable)
    return false;
  return true;
}

function getVisibleTargets(
  targets: MemorySearchTarget[],
  activeTargetId: string,
  limit: number,
) {
  const pinnedTargets = targets.slice(0, limit);
  if (pinnedTargets.some((target) => target.id === activeTargetId)) {
    return pinnedTargets;
  }

  const activeTarget = targets.find((target) => target.id === activeTargetId);
  if (!activeTarget) return pinnedTargets;
  return [...pinnedTargets.slice(0, limit - 1), activeTarget];
}

function useCompactTargetNav() {
  const [compact, setCompact] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(max-width: 700px)");
    const syncCompact = () => setCompact(mediaQuery.matches);

    syncCompact();
    mediaQuery.addEventListener("change", syncCompact);
    return () => mediaQuery.removeEventListener("change", syncCompact);
  }, []);

  return compact;
}

function buildDisplayTargets(
  targets: MemorySearchTarget[],
  workspaceLabel?: string,
  workspaceCode?: string,
  userLabel?: string,
  userCode?: string,
) {
  return targets.map((target) => {
    if (target.id === "space:current") {
      return {
        ...target,
        code: workspaceCode || target.code,
        label: workspaceLabel || target.label,
      };
    }

    if (target.id === "user:current") {
      return {
        ...target,
        code: userCode || target.code,
        label: userLabel || target.label,
      };
    }

    return target;
  });
}

function MemoryFilterControls({
  filters,
  onFiltersChange,
  onSearchChange,
  query,
}: {
  filters: MemoryFilterState;
  onFiltersChange: (filters: MemoryFilterState) => void;
  onSearchChange: (value: string) => void;
  query: string;
}) {
  return (
    <>
      <SearchField
        ariaLabel="Search memories"
        className="w-full"
        placeholder="Search memories"
        value={query}
        onChange={onSearchChange}
      />
      <SelectInput
        aria-label="Filter memories by status"
        className="w-full"
        value={filters.status}
        onValueChange={(event) =>
          onFiltersChange({
            ...filters,
            status: event as MemoryFilterState["status"],
          })
        }
      >
        {memoryStatusOptions.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectInput>
      <SelectInput
        aria-label="Filter memories by review status"
        className="w-full"
        value={filters.reviewStatus}
        onValueChange={(event) =>
          onFiltersChange({
            ...filters,
            reviewStatus: event as MemoryFilterState["reviewStatus"],
          })
        }
      >
        {memoryReviewOptions.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectInput>
      <SelectInput
        aria-label="Filter memories by context use"
        className="w-full"
        value={filters.injectable}
        onValueChange={(event) =>
          onFiltersChange({
            ...filters,
            injectable: event as MemoryFilterState["injectable"],
          })
        }
      >
        {memoryInjectableOptions.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectInput>
    </>
  );
}

function EmptyResults({
  activeTarget,
  query,
}: {
  activeTarget: MemorySearchTarget;
  query: string;
}) {
  const trimmedQuery = query.trim();

  return (
    <section className="rounded-xl border border-dashed border-[var(--color-outline-variant)] px-5 py-8 text-sm leading-6 text-[var(--color-on-surface-variant)]">
      <p className="font-medium text-[var(--color-on-surface)]">
        No memories found
      </p>
      <p className="mt-1">
        {trimmedQuery
          ? `No memories in ${activeTarget.label} match "${trimmedQuery}".`
          : `No memories in ${activeTarget.label} match the current filters.`}
      </p>
    </section>
  );
}

function formatReviewStatus(reviewStatus: MemoryRecord["reviewStatus"]) {
  return reviewStatus.replace("_", " ");
}

function getUseSummary(memory: MemoryRecord) {
  if (memory.injectable) {
    return {
      detail: "added automatically",
      label: "Auto context",
      tone: "success" as const,
    };
  }

  return {
    detail: "manual lookup",
    label: "Search only",
    tone: "info" as const,
  };
}

function MemoryRecordsTable({
  activeTarget,
  memories,
  onArchive,
  onEdit,
  onOpen,
  query,
  title,
  toolbar,
}: {
  activeTarget: MemorySearchTarget;
  memories: MemoryRecord[];
  onArchive: (memory: MemoryRecord) => void;
  onEdit: (memory: MemoryRecord) => void;
  onOpen: (memory: MemoryRecord) => void;
  query: string;
} & Pick<DataTableProps<MemoryRecord>, "title" | "toolbar">) {
  const columns = useMemo<TableColumnDef<MemoryRecord>[]>(
    () => [
      {
        accessorKey: "title",
        cell: ({ row }) => (
          <MemoryRecordSummary
            memory={row.original}
            onOpen={() => onOpen(row.original)}
          />
        ),
        header: "Memory",
        id: "memory",
        meta: {
          cellClassName: "min-w-0 py-4",
          width: "330px",
        },
      },
      {
        cell: ({ row }) => (
          <Badge
            className="w-fit max-w-full font-bold"
            tone={getTypeTone(row.original)}
          >
            {row.original.memoryType}
          </Badge>
        ),
        header: "Type",
        id: "type",
        meta: {
          cellClassName: "py-4",
          width: "170px",
        },
      },
      {
        cell: ({ row }) => (
          <div className="flex gap-1">
            <Badge
              tone={row.original.memoryStatus === "ACTIVE" ? "success" : "info"}
            >
              {row.original.memoryStatus}
            </Badge>
            <span className="text-[12px] leading-4 text-[var(--color-on-surface-variant)]">
              {formatReviewStatus(row.original.reviewStatus)}
            </span>
          </div>
        ),
        header: "Status",
        id: "status",
        meta: {
          cellClassName: "py-4",
          width: "150px",
        },
      },
      {
        cell: ({ row }) => {
          const useSummary = getUseSummary(row.original);
          return (
            <div className="flex gap-1">
              <Badge tone={useSummary.tone}>{useSummary.label}</Badge>
              <span className="text-[12px] leading-4 text-[var(--color-on-surface-variant)]">
                {useSummary.detail}
              </span>
            </div>
          );
        },
        header: "Use",
        id: "use",
        meta: {
          cellClassName: "py-4",
          width: "150px",
        },
      },
      {
        cell: ({ row }) => row.original.updatedAt.split(" ")[0],
        header: "Updated",
        id: "updatedAt",
        meta: {
          width: "130px",
        },
      },
      {
        cell: ({ row }) => (
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  aria-label={`Open actions for ${row.original.title}`}
                  size="icon"
                  variant="ghost"
                  onClick={(event) => event.stopPropagation()}
                  onKeyDown={(event) => event.stopPropagation()}
                >
                  <MoreHorizontal size={16} aria-hidden="true" />
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-44">
              <DropdownMenuItem
                onClick={(event) => {
                  event.stopPropagation();
                  onOpen(row.original);
                }}
              >
                Open details
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={(event) => {
                  event.stopPropagation();
                  onEdit(row.original);
                }}
              >
                Edit
              </DropdownMenuItem>
              <DropdownMenuItem
                variant="destructive"
                onClick={(event) => {
                  event.stopPropagation();
                  onArchive(row.original);
                }}
              >
                Archive
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        ),
        header: "Actions",
        id: "actions",
        meta: {
          align: "right",
          width: "100px",
        },
      },
    ],
    [onArchive, onEdit, onOpen],
  );
  const table = useReactTable({
    columns,
    data: memories,
    getCoreRowModel: getCoreRowModel(),
    getRowId: (row) => row.id,
  });

  return (
    <DataTable<MemoryRecord>
      title={title}
      toolbar={toolbar}
      emptyState={<EmptyResults activeTarget={activeTarget} query={query} />}
      table={table}
      tableClassName="table-fixed min-w-[1018px]"
    />
  );
}

function MemoryRepository() {
  const navigate = useNavigate();
  const [refreshKey, setRefreshKey] = useState(0);
  const dataSource = getServiceDataSource("memory");
  const { session } = useAuthSession();
  const {
    selectedPrimarySpace,
    selectedPrimarySpaceId,
    selectedSecondarySpace,
    selectedSecondarySpaceId,
  } = useWorkspaceSpaces();
  const [activeTargetId, setActiveTargetId] = useState("space:current");
  const [query, setQuery] = useState("");
  const [filters, setFilters] =
    useState<MemoryFilterState>(defaultMemoryFilters);
  const [notice, setNotice] = useState("");
  const [archiveTarget, setArchiveTarget] = useState<MemoryRecord | null>(null);

  useConsoleShellConfig({ searchPlaceholder: "Search memories..." });

  const workspaceLabel =
    selectedSecondarySpace?.name ?? selectedPrimarySpace?.name;
  const workspaceCode = selectedSecondarySpaceId || selectedPrimarySpaceId;
  const userLabel = session?.user.username || session?.user.displayName;
  const userCode = session?.user.userCode;
  const rawTargets = useMemoryTargets(workspaceCode, refreshKey);
  const safeTargets = Array.isArray(rawTargets) ? rawTargets : [];
  const targets = useMemo(
    () =>
      buildDisplayTargets(
        safeTargets,
        workspaceLabel,
        workspaceCode,
        userLabel,
        userCode,
      ),
    [safeTargets, userCode, userLabel, workspaceCode, workspaceLabel],
  );
  const fallbackTarget = useMemo<MemorySearchTarget>(
    () => ({
      code: workspaceCode || "space:current",
      description: "Current space",
      id: "space:current",
      label: workspaceLabel || "Current space",
      resultCount: 0,
      scope: "space",
    }),
    [workspaceCode, workspaceLabel],
  );
  const displayTargets = targets.length ? targets : [fallbackTarget];
  const activeTarget =
    displayTargets.find((target) => target.id === activeTargetId) ??
    displayTargets[0];
  const memories = useMemoryFragmentList({
    activeTarget,
    filters,
    query,
    refreshKey,
    workspaceCode,
  });
  const safeMemories = Array.isArray(memories) ? memories : [];
  const visibleMemories = useMemo(
    () =>
      activeTarget
        ? safeMemories.filter(
            (memory) =>
              memory.targetId === activeTarget.id &&
              recordMatchesQuery(memory, query) &&
              recordMatchesFilters(memory, filters),
          )
        : [],
    [activeTarget, filters, query, safeMemories],
  );

  function openMemory(memory: MemoryRecord) {
    navigate(`/memory/details?memoryId=${encodeURIComponent(memory.id)}`);
  }

  function editMemory(memory: MemoryRecord) {
    navigate(
      `/memory/details?memoryId=${encodeURIComponent(memory.id)}&mode=edit`,
    );
  }

  async function confirmArchive() {
    if (!archiveTarget) return;
    try {
      const archived =
        dataSource === "api" && workspaceCode
          ? await archiveMemoryRecord(workspaceCode, archiveTarget.memoryCode)
          : archiveMemoryFragment(archiveTarget.id);
      setNotice(`${archived.title} archived.`);
      setRefreshKey((value) => value + 1);
    } catch (error) {
      setNotice(
        error instanceof Error
          ? error.message
          : "Memory could not be archived.",
      );
    } finally {
      setArchiveTarget(null);
    }
  }

  const activeTargets = getVisibleTargets(
    targets,
    activeTargetId,
    useCompactTargetNav() ? 1 : visibleTargetLimit,
  );

  return (
    <main className="text-[var(--color-on-surface)]">
      <div className="space-y-5">
        <MemoryRecordsTable
          activeTarget={activeTarget}
          memories={visibleMemories}
          query={query}
          onArchive={setArchiveTarget}
          onEdit={editMemory}
          onOpen={openMemory}
          title="Memory"
          toolbar={
            <>
              <SelectInput
                aria-label="Filter memories by status"
                className="w-full"
                value={activeTargetId}
                onValueChange={(event) => {
                  setActiveTargetId(event!);
                }}
              >
                {targets.map((option) => (
                  <SelectItem key={option.id} value={option.label}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectInput>
              <MemoryFilterControls
                filters={filters}
                query={query}
                onFiltersChange={setFilters}
                onSearchChange={setQuery}
              />
              <Button
                size="lg"
                variant="outline"
                onClick={() => {
                  setRefreshKey((value) => value + 1);
                  setNotice("Memory refreshed.");
                }}
              >
                Refresh
              </Button>
              <Button size="lg" onClick={() => navigate("/memory/new")}>
                New
              </Button>
            </>
          }
        />
      </div>

      <ConfirmDialog
        open={archiveTarget !== null}
        title="Archive memory?"
        description={
          archiveTarget
            ? `${archiveTarget.title} will stay available for later review.`
            : undefined
        }
        confirmText="Archive"
        onCancel={() => setArchiveTarget(null)}
        onConfirm={confirmArchive}
      />
    </main>
  );
}

export function MemoryPage() {
  return <MemoryRepository />;
}

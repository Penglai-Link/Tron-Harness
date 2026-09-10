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
import { Field, SelectInput } from "@/components/forms";
import { SearchField } from "@/components/search-field";
import {
  PageHeader,
  SurfaceCard,
  useTabPanelConfig,
} from "@/shared";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Badge } from "@/components/ui/badge";
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
import { useTranslation } from "react-i18next";

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
  const { t } = useTranslation(["pages"]);
  return (
    <>
      <SearchField
        ariaLabel={t("pages:memory.search", "Search memories")}
        className="w-full"
        placeholder={t("pages:memory.search", "Search memories")}
        value={query}
        onChange={onSearchChange}
      />
      <SelectInput
        aria-label={t("pages:memory.filters.status", "Filter memories by status")}
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
            {t(`pages:memory.filters.statusOptions.${option.value}`, option.label)}
          </SelectItem>
        ))}
      </SelectInput>
      <SelectInput
        aria-label={t("pages:memory.filters.review", "Filter memories by review status")}
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
            {t(`pages:memory.filters.reviewOptions.${option.value}`, option.label)}
          </SelectItem>
        ))}
      </SelectInput>
      <SelectInput
        aria-label={t("pages:memory.filters.injectable", "Filter memories by context use")}
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
            {t(`pages:memory.filters.useOptions.${option.value}`, option.label)}
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
  const { t } = useTranslation(["pages"]);
  const trimmedQuery = query.trim();

  return (
    <section className="rounded-xl border border-dashed border-[var(--color-outline-variant)] px-5 py-8 text-sm leading-6 text-[var(--color-on-surface-variant)]">
      <p className="font-medium text-[var(--color-on-surface)]">
        {t("pages:memory.empty.title", "No memories found")}
      </p>
      <p className="mt-1">
        {trimmedQuery
          ? t("pages:memory.empty.withQuery", "No memories in {{target}} match \"{{query}}\".", { target: activeTarget.label, query: trimmedQuery })
          : t("pages:memory.empty.withFilters", "No memories in {{target}} match the current filters.", { target: activeTarget.label })}
      </p>
    </section>
  );
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
  const { t } = useTranslation(["pages"]);
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
        header: t("pages:memory.columns.memory", "Memory"),
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
            variant={getTypeTone(row.original)}
          >
            {row.original.memoryType}
          </Badge>
        ),
        header: t("pages:memory.columns.type", "Type"),
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
              variant={row.original.memoryStatus === "ACTIVE" ? "success" : "info"}
            >
              {t(`pages:memory.statusValue.${row.original.memoryStatus}`, row.original.memoryStatus)}
            </Badge>
            <span className="text-[12px] leading-4 text-[var(--color-on-surface-variant)]">
              {t(`pages:memory.reviewValue.${row.original.reviewStatus}`, row.original.reviewStatus.replace("_", " "))}
            </span>
          </div>
        ),
        header: t("pages:memory.columns.status", "Status"),
        id: "status",
        meta: {
          cellClassName: "py-4",
          width: "150px",
        },
      },
      {
        cell: ({ row }) => {
          const injectable = row.original.injectable;
          return (
            <div className="flex gap-1">
              <Badge variant={injectable ? "success" : "info"}>
                {injectable
                  ? t("pages:memory.use.autoContext", "Auto context")
                  : t("pages:memory.use.searchOnly", "Search only")}
              </Badge>
              <span className="text-[12px] leading-4 text-[var(--color-on-surface-variant)]">
                {injectable
                  ? t("pages:memory.use.autoContextDetail", "added automatically")
                  : t("pages:memory.use.searchOnlyDetail", "manual lookup")}
              </span>
            </div>
          );
        },
        header: t("pages:memory.columns.use", "Use"),
        id: "use",
        meta: {
          cellClassName: "py-4",
          width: "150px",
        },
      },
      {
        cell: ({ row }) => row.original.updatedAt.split(" ")[0],
        header: t("pages:memory.columns.updated", "Updated"),
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
                  aria-label={t("pages:memory.aria.actionsFor", "Open actions for {{title}}", { title: row.original.title })}
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
                {t("pages:memory.actions.open", "Open details")}
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={(event) => {
                  event.stopPropagation();
                  onEdit(row.original);
                }}
              >
                {t("pages:memory.actions.edit", "Edit")}
              </DropdownMenuItem>
              <DropdownMenuItem
                variant="destructive"
                onClick={(event) => {
                  event.stopPropagation();
                  onArchive(row.original);
                }}
              >
                {t("pages:memory.actions.archive", "Archive")}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        ),
        header: t("pages:memory.columns.actions", "Actions"),
        id: "actions",
        meta: {
          align: "right",
          width: "100px",
        },
      },
    ],
    [onArchive, onEdit, onOpen, t],
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
  const { t } = useTranslation(["pages"]);
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

  useTabPanelConfig({ searchPlaceholder: t("pages:memory.searchPlaceholder", "Search memories...") });

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
      description: t("pages:memory.form.currentSpace", "Current space"),
      id: "space:current",
      label: workspaceLabel || t("pages:memory.form.currentSpace", "Current space"),
      resultCount: 0,
      scope: "space",
    }),
    [workspaceCode, workspaceLabel, t],
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
      setNotice(t("pages:memory.messages.archived", "{{title}} archived.", { title: archived.title }));
      setRefreshKey((value) => value + 1);
    } catch (error) {
      setNotice(
        error instanceof Error
          ? error.message
          : t("pages:memory.errors.archive", "Memory could not be archived."),
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
          title={t("pages:memory.title", "Memory")}
          toolbar={
            <div className="flex gap-[12px]">
              <SelectInput
                aria-label={t("pages:memory.filters.target", "Filter memory target")}
                className="w-full"
                value={activeTargetId}
                onValueChange={(event) => {
                  setActiveTargetId(event!);
                }}
              >
                {targets.map((option) => (
                  <SelectItem key={option.id} value={option.id}>
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
                  setNotice(t("pages:memory.messages.refreshed", "Memory refreshed."));
                }}
              >
                {t("pages:memory.actions.refresh", "Refresh")}
              </Button>
              <Button size="lg" onClick={() => navigate("/memory/new")}>
                {t("pages:memory.actions.new", "New")}
              </Button>
            </div>
          }
        />
      </div>

      <ConfirmDialog
        open={archiveTarget !== null}
        title={t("pages:memory.confirm.title", "Archive memory?")}
        description={
          archiveTarget
            ? t("pages:memory.confirm.description", "{{title}} will stay available for later review.", { title: archiveTarget.title })
            : undefined
        }
        confirmText={t("pages:memory.actions.archive", "Archive")}
        onCancel={() => setArchiveTarget(null)}
        onConfirm={confirmArchive}
      />
    </main>
  );
}

export function MemoryPage() {
  return <MemoryRepository />;
}

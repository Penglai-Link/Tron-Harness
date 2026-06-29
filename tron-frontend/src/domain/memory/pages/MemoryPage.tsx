import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
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

import { useAuthSession } from "@/domain/auth";
import {
  defaultMemoryFilters,
  memoryInjectableOptions,
  memoryReviewOptions,
  memoryStatusOptions,
  type MemoryFilterState,
} from "@/domain/memory/constants";
import { archiveMemoryRecord, useMemoryFragmentList, useMemoryTargets } from "@/domain/memory";
import type {
  MemoryRecord,
  MemoryScopeKind,
  MemorySearchTarget,
} from "@/domain/memory/model";
import { archiveMemoryFragment } from "@/domain/memory/mock/data";
import {
  Badge,
  Button,
  ConfirmDialog,
  Field,
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  useConsoleShellConfig,
  useDismissibleLayer,
} from "@/shared";
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

function getTypeTone(memory: MemoryRecord): "accent" | "success" | "warning" | "neutral" | "error" {
  if (memory.memoryStatus === "DISABLED") return "error";
  if (memory.scope === "space") return "success";
  if (memory.scope === "domain") return "warning";
  if (memory.scope === "user") return "accent";
  return "neutral";
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

function recordMatchesFilters(memory: MemoryRecord, filters: MemoryFilterState) {
  if (filters.status !== "all" && memory.memoryStatus !== filters.status) return false;
  if (filters.reviewStatus !== "all" && memory.reviewStatus !== filters.reviewStatus) return false;
  if (filters.injectable === "injectable" && !memory.injectable) return false;
  if (filters.injectable === "not-injectable" && memory.injectable) return false;
  return true;
}

function getVisibleTargets(targets: MemorySearchTarget[], activeTargetId: string, limit: number) {
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

function SearchTargetButton({
  active,
  onClick,
  target,
}: {
  active: boolean;
  onClick: () => void;
  target: MemorySearchTarget;
}) {
  return (
    <button
      aria-current={active ? "page" : undefined}
      className={cn(
        "inline-flex h-[46px] max-w-[220px] flex-none items-center gap-2 border-b-[3px] px-0 text-sm tracking-normal transition-colors",
        active
          ? "border-[var(--color-on-surface)] font-semibold text-[var(--color-on-surface)]"
          : "border-transparent text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]",
      )}
      title={`${getScopeLabel(target.scope)}: ${target.label}`}
      type="button"
      onClick={onClick}
    >
      <MemoryTargetIcon scope={target.scope} />
      <span className="min-w-0 truncate">{target.label}</span>
    </button>
  );
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
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <div className="min-w-0">
        <Field label="Search">
          <SearchField
            ariaLabel="Search memories"
            className="w-full"
            placeholder="Search memories"
            value={query}
            onChange={onSearchChange}
          />
        </Field>
      </div>
      <Field label="Status">
        <SelectInput
          aria-label="Filter memories by status"
          className="w-full"
          value={filters.status}
          onValueChange={(event) => onFiltersChange({ ...filters, status: event as MemoryFilterState["status"] })}
        >
          {memoryStatusOptions.map((option) => (
            <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
          ))}
        </SelectInput>
      </Field>
      <Field label="Review">
        <SelectInput
          aria-label="Filter memories by review status"
          className="w-full"
          value={filters.reviewStatus}
          onValueChange={(event) => onFiltersChange({ ...filters, reviewStatus: event as MemoryFilterState["reviewStatus"] })}
        >
          {memoryReviewOptions.map((option) => (
            <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
          ))}
        </SelectInput>
      </Field>
      <Field label="Use">
        <SelectInput
          aria-label="Filter memories by context use"
          className="w-full"
          value={filters.injectable}
          onValueChange={(event) => onFiltersChange({ ...filters, injectable: event as MemoryFilterState["injectable"] })}
        >
          {memoryInjectableOptions.map((option) => (
            <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
          ))}
        </SelectInput>
      </Field>
    </div>
  );
}

function SearchTargetNav({
  activeTargetId,
  onTargetChange,
  targets,
}: {
  activeTargetId: string;
  onTargetChange: (targetId: string) => void;
  targets: MemorySearchTarget[];
}) {
  const [moreOpen, setMoreOpen] = useState(false);
  const compactTargetNav = useCompactTargetNav();
  const visibleTargets = getVisibleTargets(targets, activeTargetId, compactTargetNav ? 1 : visibleTargetLimit);
  const visibleTargetIds = new Set(visibleTargets.map((target) => target.id));
  const overflowTargets = targets.filter((target) => !visibleTargetIds.has(target.id));

  return (
    <div className="mt-2 flex min-h-[46px] items-end gap-5 border-b border-[var(--color-outline-variant)]">
      <nav
        aria-label="Search targets"
        className="flex min-w-0 flex-1 items-end gap-5 overflow-x-auto overflow-y-hidden"
      >
        {visibleTargets.map((target) => (
          <SearchTargetButton
            key={target.id}
            active={target.id === activeTargetId}
            target={target}
            onClick={() => {
              setMoreOpen(false);
              onTargetChange(target.id);
            }}
          />
        ))}
      </nav>
      <div className="flex flex-none items-end gap-4">
        <div className="relative">
          <button
            className="inline-flex h-[46px] items-center gap-1 border-b-[3px] border-transparent text-sm text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]"
            type="button"
            onClick={() => {
              setMoreOpen((open) => !open);
            }}
          >
            More
            <ChevronDown size={14} aria-hidden="true" />
          </button>
          {moreOpen ? (
            <div className="absolute right-0 top-full z-20 mt-2 w-56 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] py-2 text-left shadow-lg">
              {overflowTargets.length > 0 ? overflowTargets.map((target) => (
                <button
                  key={target.id}
                  className="flex w-full items-center gap-2 px-3 py-2 text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
                  title={`${getScopeLabel(target.scope)}: ${target.label}`}
                  type="button"
                  onClick={() => {
                    onTargetChange(target.id);
                    setMoreOpen(false);
                  }}
                >
                  <MemoryTargetIcon scope={target.scope} />
                  <span className="min-w-0 flex-1 truncate">{target.label}</span>
                  {target.id === activeTargetId ? <Check size={15} aria-hidden="true" /> : null}
                </button>
              )) : (
                <p className="px-3 py-2 text-sm text-[var(--color-on-surface-variant)]">No more targets</p>
              )}
            </div>
          ) : null}
        </div>
      </div>
    </div>
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
      <p className="font-medium text-[var(--color-on-surface)]">No memories found</p>
      <p className="mt-1">
        {trimmedQuery
          ? `No memories in ${activeTarget.label} match "${trimmedQuery}".`
          : `No memories in ${activeTarget.label} match the current filters.`}
      </p>
    </section>
  );
}

interface MemoryRowMenuState {
  left: number;
  memoryId: string;
  top: number;
  triggerElement: HTMLButtonElement;
}

function MobileLabel({ children }: { children: string }) {
  return (
    <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)] lg:hidden">
      {children}
    </span>
  );
}

function MemoryCell({
  children,
  label,
}: {
  children: ReactNode;
  label: string;
}) {
  return (
    <div className="mt-3 flex min-w-0 items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] pt-3 text-[13px] text-[var(--color-on-surface)] lg:mt-0 lg:block lg:border-t-0 lg:px-5 lg:py-4 lg:pt-4">
      <MobileLabel>{label}</MobileLabel>
      <div className="min-w-0 text-right lg:text-left">{children}</div>
    </div>
  );
}

function formatReviewStatus(reviewStatus: MemoryRecord["reviewStatus"]) {
  return reviewStatus.replace("_", " ");
}

function getUseSummary(memory: MemoryRecord) {
  if (memory.injectable) {
    return {
      detail: "available to agent",
      label: "Auto context",
      tone: "success" as const,
    };
  }

  return {
    detail: "not injected",
    label: "Search only",
    tone: "neutral" as const,
  };
}

function MemoryRecordsTable({
  activeTarget,
  memories,
  onArchive,
  onEdit,
  onOpen,
  query,
}: {
  activeTarget: MemorySearchTarget;
  memories: MemoryRecord[];
  onArchive: (memory: MemoryRecord) => void;
  onEdit: (memory: MemoryRecord) => void;
  onOpen: (memory: MemoryRecord) => void;
  query: string;
}) {
  const [rowMenu, setRowMenu] = useState<MemoryRowMenuState | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);
  const openMenuMemory = rowMenu
    ? memories.find((memory) => memory.id === rowMenu.memoryId) ?? null
    : null;

  useDismissibleLayer({
    open: rowMenu !== null,
    onDismiss: () => setRowMenu(null),
    popupRef: rowMenuRef,
    triggerElement: rowMenu?.triggerElement,
  });

  function toggleRowMenu(memory: MemoryRecord, triggerElement: HTMLButtonElement) {
    if (rowMenu?.memoryId === memory.id) {
      setRowMenu(null);
      return;
    }

    const menuWidth = 176;
    const estimatedMenuHeight = 152;
    const rect = triggerElement.getBoundingClientRect();
    setRowMenu({
      memoryId: memory.id,
      triggerElement,
      left: Math.min(
        Math.max(12, rect.right - menuWidth),
        window.innerWidth - menuWidth - 12,
      ),
      top: Math.min(
        Math.max(12, rect.bottom + 8),
        window.innerHeight - estimatedMenuHeight - 12,
      ),
    });
  }

  function handleMenuAction(memory: MemoryRecord, action: string) {
    setRowMenu(null);
    if (action === "open") {
      onOpen(memory);
      return;
    }
    if (action === "edit") {
      onEdit(memory);
      return;
    }
    if (action === "archive") {
      onArchive(memory);
    }
  }

  return (
    <section className="overflow-hidden rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]">
      {memories.length > 0 ? (
        <>
          <div className="overflow-x-visible lg:overflow-x-auto">
            <div className="hidden min-w-[1018px] grid-cols-[minmax(330px,1fr)_170px_150px_150px_118px_100px] border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)] lg:grid">
              <span className="px-5 py-3">Memory</span>
              <span className="px-5 py-3">Type</span>
              <span className="px-5 py-3">Status</span>
              <span className="px-5 py-3">Use</span>
              <span className="px-5 py-3">Updated</span>
              <span className="sticky right-0 z-20 border-l border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-5 py-3 text-right shadow-[-8px_0_12px_rgba(0,0,0,0.04)]">Actions</span>
            </div>

            <div className="divide-y divide-[var(--color-outline-variant)]">
              {memories.map((memory) => {
                const useSummary = getUseSummary(memory);

                return (
                  <article
                    className="group grid p-4 hover:bg-[var(--color-hover-surface)] lg:min-w-[1018px] lg:grid-cols-[minmax(330px,1fr)_170px_150px_150px_118px_100px] lg:p-0"
                    key={memory.id}
                  >
                    <div className="min-w-0 lg:px-5 lg:py-4">
                      <button
                        className="text-left text-base font-medium leading-6 tracking-normal text-[#1a0dab] hover:underline"
                        type="button"
                        onClick={() => onOpen(memory)}
                      >
                        {memory.title}
                      </button>
                      <p className="mt-1 text-[13px] leading-5 text-[var(--color-on-surface)]">
                        {memory.text}
                      </p>
                      <div className="mt-1 max-w-full overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[12px] leading-5 text-[var(--color-on-surface-variant)]">
                        {memory.memoryCode}
                      </div>
                    </div>
                    <MemoryCell label="Type">
                      <Badge className="w-fit max-w-full font-bold" tone={getTypeTone(memory)}>{memory.memoryType}</Badge>
                    </MemoryCell>
                    <MemoryCell label="Status">
                      <div className="grid justify-items-end gap-1 lg:justify-items-start">
                        <Badge className="w-fit max-w-full font-bold" tone={memory.memoryStatus === "ACTIVE" ? "success" : "neutral"}>
                          {memory.memoryStatus}
                        </Badge>
                        <span className="text-[12px] leading-4 text-[var(--color-on-surface-variant)]">
                          {formatReviewStatus(memory.reviewStatus)}
                        </span>
                      </div>
                    </MemoryCell>
                    <MemoryCell label="Use">
                      <div className="grid justify-items-end gap-1 lg:justify-items-start">
                        <Badge className="w-fit max-w-full font-bold" tone={useSummary.tone}>
                          {useSummary.label}
                        </Badge>
                        <span className="text-[12px] leading-4 text-[var(--color-on-surface-variant)]">
                          {useSummary.detail}
                        </span>
                      </div>
                    </MemoryCell>
                    <MemoryCell label="Updated">{memory.updatedAt.split(" ")[0]}</MemoryCell>
                    <div className="relative mt-3 flex min-w-0 items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] pt-3 lg:sticky lg:right-0 lg:z-10 lg:mt-0 lg:justify-end lg:border-l lg:border-t-0 lg:bg-[var(--color-elevated-surface)] lg:px-5 lg:py-3 lg:pt-3 lg:shadow-[-8px_0_12px_rgba(0,0,0,0.04)] lg:group-hover:bg-[var(--color-hover-surface)]">
                      <MobileLabel>Actions</MobileLabel>
                      <Button
                        aria-expanded={rowMenu?.memoryId === memory.id}
                        aria-label={`Open actions for ${memory.title}`}
                        size="icon"
                        variant="ghost"
                        onClick={(event) => {
                          event.stopPropagation();
                          toggleRowMenu(memory, event.currentTarget);
                        }}
                      >
                        <MoreHorizontal size={16} aria-hidden="true" />
                      </Button>
                    </div>
                  </article>
                );
              })}
            </div>
          </div>
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] px-5 py-3 text-[13px] text-[var(--color-on-surface-variant)]">
            <span>{memories.length} memories shown</span>
            <span className="truncate">Target: {activeTarget.label}</span>
          </div>
        </>
      ) : (
        <div className="p-5">
          <EmptyResults activeTarget={activeTarget} query={query} />
        </div>
      )}

      {rowMenu && openMenuMemory ? createPortal(
        <div
          ref={rowMenuRef}
          className="fixed z-[80] w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
          role="menu"
          aria-label={`Actions for ${openMenuMemory.title}`}
          style={{ left: rowMenu.left, top: rowMenu.top }}
        >
          <button
            className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
            role="menuitem"
            type="button"
            onClick={() => handleMenuAction(openMenuMemory, "open")}
          >
            Open details
          </button>
          <button
            className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
            role="menuitem"
            type="button"
            onClick={() => handleMenuAction(openMenuMemory, "edit")}
          >
            Edit
          </button>
          <button
            className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)]"
            role="menuitem"
            type="button"
            onClick={() => handleMenuAction(openMenuMemory, "archive")}
          >
            Archive
          </button>
        </div>,
        document.body,
      ) : null}
    </section>
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
  const [filters, setFilters] = useState<MemoryFilterState>(defaultMemoryFilters);
  const [notice, setNotice] = useState("");
  const [archiveTarget, setArchiveTarget] = useState<MemoryRecord | null>(null);

  useConsoleShellConfig({ searchPlaceholder: "Search memories..." });

  const workspaceLabel = selectedSecondarySpace?.name ?? selectedPrimarySpace?.name;
  const workspaceCode = selectedSecondarySpaceId || selectedPrimarySpaceId;
  const userLabel = session?.user.username || session?.user.displayName;
  const userCode = session?.user.userCode;
  const rawTargets = useMemoryTargets(workspaceCode, refreshKey);
  const safeTargets = Array.isArray(rawTargets) ? rawTargets : [];
  const targets = useMemo(
    () => buildDisplayTargets(safeTargets, workspaceLabel, workspaceCode, userLabel, userCode),
    [safeTargets, userCode, userLabel, workspaceCode, workspaceLabel],
  );
  const fallbackTarget = useMemo<MemorySearchTarget>(() => ({
    code: workspaceCode || "space:current",
    description: "Current space",
    id: "space:current",
    label: workspaceLabel || "Current space",
    resultCount: 0,
    scope: "space",
  }), [workspaceCode, workspaceLabel]);
  const displayTargets = targets.length ? targets : [fallbackTarget];
  const activeTarget = displayTargets.find((target) => target.id === activeTargetId) ?? displayTargets[0];
  const memories = useMemoryFragmentList({
    activeTarget,
    filters,
    query,
    refreshKey,
    workspaceCode,
  });
  const safeMemories = Array.isArray(memories) ? memories : [];
  const visibleMemories = useMemo(
    () => activeTarget ? safeMemories.filter((memory) => (
      memory.targetId === activeTarget.id
      && recordMatchesQuery(memory, query)
      && recordMatchesFilters(memory, filters)
    )) : [],
    [activeTarget, filters, query, safeMemories],
  );

  function openMemory(memory: MemoryRecord) {
    navigate(`/memory/details?memoryId=${encodeURIComponent(memory.id)}`);
  }

  function editMemory(memory: MemoryRecord) {
    navigate(`/memory/details?memoryId=${encodeURIComponent(memory.id)}&mode=edit`);
  }

  async function confirmArchive() {
    if (!archiveTarget) return;
    try {
      const archived = dataSource === "api" && workspaceCode
        ? await archiveMemoryRecord(workspaceCode, archiveTarget.memoryCode)
        : archiveMemoryFragment(archiveTarget.id);
      setNotice(`${archived.title} archived.`);
      setRefreshKey((value) => value + 1);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Memory could not be archived.");
    } finally {
      setArchiveTarget(null);
    }
  }

  return (
    <main className="text-[var(--color-on-surface)]">
      <PageHeader
        title="Memory"
        actions={(
          <>
            <Button
              variant="secondary"
              onClick={() => {
                setRefreshKey((value) => value + 1);
                setNotice("Memory list refreshed.");
              }}
            >
              <RefreshCcw size={16} aria-hidden="true" />
              Refresh
            </Button>
            <Button onClick={() => navigate("/memory/new")}>
              <Plus size={16} aria-hidden="true" />
              New
            </Button>
          </>
        )}
      />
      {notice ? <p className="sr-only" aria-live="polite">{notice}</p> : null}

      <div className="space-y-5">
        <SurfaceCard className="space-y-3">
          <MemoryFilterControls
            filters={filters}
            query={query}
            onFiltersChange={setFilters}
            onSearchChange={setQuery}
          />
          <SearchTargetNav
            activeTargetId={activeTarget.id}
            targets={displayTargets}
            onTargetChange={setActiveTargetId}
          />
        </SurfaceCard>

        <MemoryRecordsTable
          activeTarget={activeTarget}
          memories={visibleMemories}
          query={query}
          onArchive={setArchiveTarget}
          onEdit={editMemory}
          onOpen={openMemory}
        />
      </div>

      <ConfirmDialog
        open={archiveTarget !== null}
        title="Archive memory?"
        description={archiveTarget ? `${archiveTarget.title} will stay available for later review.` : undefined}
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

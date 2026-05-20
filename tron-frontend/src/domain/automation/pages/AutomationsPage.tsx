import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import {
  MoreHorizontal,
  Plus,
  RefreshCcw,
  Search,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextInput,
  TronShell,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import {
  archiveAutomation,
  executeAutomation,
  restoreAutomation,
  updateAutomationStatus,
} from "@/domain/automation/api";
import {
  getAutomationExecutionStatusTone,
  getAutomationStatusTone,
} from "@/domain/automation/constants";
import { useAutomationList } from "@/domain/automation/hooks";
import {
  archiveMockAutomation,
  executeMockAutomation,
  restoreMockAutomation,
  updateMockAutomationStatus,
} from "@/domain/automation/mock/data";
import type {
  AutomationListItem,
  AutomationOperationalStatus,
  AutomationStatus,
} from "@/domain/automation/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function SearchField({
  className,
  inputClassName,
  onChange,
  value,
}: {
  className?: string;
  inputClassName?: string;
  onChange: (value: string) => void;
  value: string;
}) {
  return (
    <div className={cn("relative", className)}>
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput
        aria-label="Search automations"
        className={cn("h-9 w-full pl-9", value ? "pr-9" : undefined, inputClassName)}
        placeholder="Filter by code, name, description, or latest result..."
        type="search"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
      {value ? (
        <Button
          aria-label="Clear automation search"
          className="absolute right-1 top-1/2 size-7 -translate-y-1/2"
          size="icon"
          type="button"
          variant="ghost"
          onClick={() => onChange("")}
        >
          <X size={14} aria-hidden="true" />
        </Button>
      ) : null}
    </div>
  );
}

function formatTimestamp(value?: string | null, fallback = "Not available") {
  if (!value) return fallback;
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function pageBounds(page: number, pageSize: number, total: number, visible: number) {
  if (!total || !visible) return { end: 0, start: 0 };
  const start = (page - 1) * pageSize + 1;
  return {
    end: start + visible - 1,
    start,
  };
}

function StatisticCard({
  label,
  value,
}: {
  label: string;
  value: number;
}) {
  return (
    <SurfaceCard className="p-4">
      <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">
        {label}
      </p>
      <p className="mt-2 text-3xl font-semibold leading-none text-[var(--color-on-surface)]">
        {value}
      </p>
    </SurfaceCard>
  );
}

interface RowMenuState {
  automationCode: string;
  left: number;
  top: number;
}

export function AutomationsPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("automation");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<AutomationStatus | "">("");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const [rowBusyKey, setRowBusyKey] = useState("");
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);
  const query = useMemo(() => ({
    current,
    search: debouncedSearch,
    size,
    status: statusFilter || undefined,
    workspaceCode: activeWorkspaceCode,
  }), [activeWorkspaceCode, current, debouncedSearch, size, statusFilter]);
  const { page, refresh } = useAutomationList(query);
  const totalPages = Math.max(1, Math.ceil(page.total / page.size));
  const bounds = pageBounds(page.current, page.size, page.total, page.records.length);
  const activeCount = page.records.filter((item) => item.status === "Active").length;
  const pausedCount = page.records.filter((item) => item.status === "Paused").length;
  const openMenuRow = openRowMenu
    ? page.records.find((item) => item.automationCode === openRowMenu.automationCode) ?? null
    : null;

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(searchInput.trim());
      setCurrent(1);
    }, 220);

    return () => window.clearTimeout(timer);
  }, [searchInput]);

  useEffect(() => {
    setCurrent(1);
  }, [activeWorkspaceCode, size, statusFilter]);

  useEffect(() => {
    setOpenRowMenu(null);
  }, [current, debouncedSearch, size, statusFilter, activeWorkspaceCode]);

  useEffect(() => {
    if (!openRowMenu) return;

    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (rowMenuRef.current?.contains(target)) return;
      const trigger = document.querySelector<HTMLButtonElement>(
        `[data-automation-menu-trigger="${openRowMenu.automationCode}"]`,
      );
      if (trigger?.contains(target)) return;
      setOpenRowMenu(null);
    };

    const handleViewportChange = () => setOpenRowMenu(null);
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenRowMenu(null);
    };

    document.addEventListener("mousedown", handlePointerDown);
    window.addEventListener("resize", handleViewportChange);
    window.addEventListener("scroll", handleViewportChange, true);
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      window.removeEventListener("resize", handleViewportChange);
      window.removeEventListener("scroll", handleViewportChange, true);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [openRowMenu]);

  function toggleRowMenu(row: AutomationListItem, trigger: HTMLButtonElement) {
    if (rowBusyKey.includes(`:${row.automationCode}`)) return;
    if (openRowMenu?.automationCode === row.automationCode) {
      setOpenRowMenu(null);
      return;
    }

    const menuWidth = 176;
    const estimatedMenuHeight = row.status === "Archived" ? 112 : 160;
    const rect = trigger.getBoundingClientRect();
    const left = Math.min(
      Math.max(12, rect.right - menuWidth),
      window.innerWidth - menuWidth - 12,
    );
    const top = Math.min(
      Math.max(12, rect.bottom + 8),
      window.innerHeight - estimatedMenuHeight - 12,
    );

    setOpenRowMenu({
      automationCode: row.automationCode,
      left,
      top,
    });
  }

  async function handleExecute(row: AutomationListItem) {
    const busyKey = `execute:${row.automationCode}`;
    if (rowBusyKey === busyKey || row.status === "Archived") return;
    setRowBusyKey(busyKey);
    setOpenRowMenu(null);

    try {
      const updated = dataSource === "api"
        ? await executeAutomation(row.automationCode)
        : executeMockAutomation(row.automationCode);
      toast.success(`Manual execution recorded for ${updated.automationCode}`);
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation execution could not be recorded");
    } finally {
      setRowBusyKey("");
    }
  }

  async function handleStatusToggle(row: AutomationListItem) {
    const busyKey = `status:${row.automationCode}`;
    if (rowBusyKey === busyKey || row.status === "Archived") return;
    setRowBusyKey(busyKey);
    setOpenRowMenu(null);

    try {
      const status: AutomationOperationalStatus = row.status === "Active" ? "Paused" : "Active";
      const updated = dataSource === "api"
        ? await updateAutomationStatus(row.automationCode, { status })
        : updateMockAutomationStatus(row.automationCode, { status });
      toast.success(`Automation ${updated.automationCode} ${status === "Active" ? "activated" : "paused"}`);
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation status could not be updated");
    } finally {
      setRowBusyKey("");
    }
  }

  async function handleArchive(row: AutomationListItem) {
    const busyKey = `archive:${row.automationCode}`;
    if (rowBusyKey === busyKey) return;
    setRowBusyKey(busyKey);
    setOpenRowMenu(null);

    try {
      if (row.status === "Archived") {
        const result = dataSource === "api"
          ? await restoreAutomation(row.automationCode)
          : restoreMockAutomation(row.automationCode);
        toast.success(`Automation ${result.automationCode} restored to ${result.status}`);
      } else {
        const result = dataSource === "api"
          ? await archiveAutomation(row.automationCode)
          : archiveMockAutomation(row.automationCode);
        toast.success(`Automation ${result.automationCode} archived`);
      }
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation lifecycle update failed");
    } finally {
      setRowBusyKey("");
    }
  }

  return (
    <TronShell activeItem="Automations" searchPlaceholder="Search automations...">
      <PageHeader
        title="Automations"
        actions={
          <>
            <Button variant="secondary" onClick={() => {
              refresh();
              toast.success("Automation list refreshed");
            }}
            >
              <RefreshCcw size={15} aria-hidden="true" />
              Refresh
            </Button>
            <Button disabled={!activeWorkspaceCode} onClick={() => navigate("/automations/new")}>
              <Plus size={15} aria-hidden="true" />
              New
            </Button>
          </>
        }
      />
      <section className="mb-4 grid gap-3 md:grid-cols-3">
        <StatisticCard label="In Scope" value={page.total} />
        <StatisticCard label="Active" value={activeCount} />
        <StatisticCard label="Paused" value={pausedCount} />
      </section>
      <DataTable<AutomationListItem>
        emptyState={activeWorkspaceCode ? "No automations match the current filters." : "Choose a workspace in the shell header before managing automations."}
        onRowClick={(row) => navigate(`/automations/detail?automationCode=${encodeURIComponent(row.automationCode)}`)}
        rows={page.records}
        rowKey={(row) => row.automationCode}
        toolbar={
          <>
            <SearchField
              className="min-w-0 grow basis-full sm:basis-0"
              value={searchInput}
              onChange={setSearchInput}
            />
            <SelectInput
              className="h-9 w-full sm:w-44"
              aria-label="Automation status filter"
              value={statusFilter}
              onChange={(event) => setStatusFilter(event.target.value as AutomationStatus | "")}
            >
              <option value="">All active states</option>
              <option value="Active">Active</option>
              <option value="Paused">Paused</option>
              <option value="Archived">Archived</option>
            </SelectInput>
          </>
        }
        pagination={(
          <TablePagination
            endItem={bounds.end}
            itemLabel="automations"
            page={page.current}
            pageCount={totalPages}
            pageSize={page.size}
            startItem={bounds.start}
            totalItems={page.total}
            onPageChange={(nextPage) => setCurrent(Math.min(totalPages, Math.max(1, nextPage)))}
            onPageSizeChange={(nextSize) => {
              setCurrent(1);
              setSize(nextSize);
            }}
          />
        )}
        columns={[
          {
            header: "Automation",
            id: "name",
            render: (row) => (
              <div className="space-y-1">
                <p className="font-medium text-[var(--color-on-surface)]">{row.name}</p>
                <p className="max-w-[28rem] text-sm leading-6 text-[var(--color-on-surface-variant)]">{row.description ?? "No description"}</p>
              </div>
            ),
          },
          {
            header: "Schedule",
            id: "schedule",
            render: (row) => (
              <div className="space-y-1">
                <p className="font-mono text-[13px] text-[var(--color-on-surface)]">{row.cronExpression}</p>
                <p className="text-sm text-[var(--color-on-surface-variant)]">Next: {formatTimestamp(row.nextRunAt, "Not scheduled")}</p>
              </div>
            ),
          },
          {
            header: "Latest result",
            id: "latest",
            render: (row) => (
              <div>
                {row.lastExecutionStatus ? (
                  <Badge tone={getAutomationExecutionStatusTone(row.lastExecutionStatus)}>
                    {row.lastExecutionStatus}
                  </Badge>
                ) : (
                  <Badge tone="neutral">Never run</Badge>
                )}
              </div>
            ),
          },
          {
            header: "Status",
            id: "status",
            render: (row) => (
              <Badge tone={getAutomationStatusTone(row.status)}>{row.status}</Badge>
            ),
          },
          {
            header: "Updated",
            id: "updated",
            render: (row) => (
              <span className="text-sm text-[var(--color-on-surface-variant)]">{formatTimestamp(row.updatedAt)}</span>
            ),
          },
          {
            align: "right",
            header: "Actions",
            id: "actions",
            render: (row) => {
              const isMenuOpen = openRowMenu?.automationCode === row.automationCode;
              const isBusy = rowBusyKey.includes(`:${row.automationCode}`);

              return (
                <div className="flex justify-end">
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label={`Open actions for ${row.name}`}
                    aria-expanded={isMenuOpen}
                    data-automation-menu-trigger={row.automationCode}
                    disabled={isBusy}
                    onClick={(event) => {
                      event.stopPropagation();
                      toggleRowMenu(row, event.currentTarget);
                    }}
                  >
                    <MoreHorizontal size={16} aria-hidden="true" />
                  </Button>
                </div>
              );
            },
          },
        ]}
      />
      {openRowMenu && openMenuRow
        ? createPortal(
          <div
            ref={rowMenuRef}
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
            className="fixed z-[80] w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={`Actions for ${openMenuRow.name}`}
          >
            {openMenuRow.status !== "Archived" ? (
              <button
                type="button"
                role="menuitem"
                className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                disabled={rowBusyKey === `execute:${openMenuRow.automationCode}`}
                onClick={() => {
                  void handleExecute(openMenuRow);
                }}
              >
                Run
              </button>
            ) : null}
            {openMenuRow.status !== "Archived" ? (
              <button
                type="button"
                role="menuitem"
                className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                disabled={rowBusyKey === `status:${openMenuRow.automationCode}`}
                onClick={() => {
                  void handleStatusToggle(openMenuRow);
                }}
              >
                {openMenuRow.status === "Active" ? "Pause" : "Activate"}
              </button>
            ) : null}
            {openMenuRow.status !== "Archived" ? (
              <button
                type="button"
                role="menuitem"
                className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                disabled={rowBusyKey === `archive:${openMenuRow.automationCode}`}
                onClick={() => {
                  void handleArchive(openMenuRow);
                }}
              >
                Archive
              </button>
            ) : null}
            {openMenuRow.status === "Archived" ? (
              <button
                type="button"
                role="menuitem"
                className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                disabled={rowBusyKey === `archive:${openMenuRow.automationCode}`}
                onClick={() => {
                  void handleArchive(openMenuRow);
                }}
              >
                Restore
              </button>
            ) : null}
          </div>,
          document.body,
        )
        : null}
    </TronShell>
  );
}

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
} from "lucide-react";

import {
  deletePlatformVariable,
  fetchPlatformVariables,
} from "@/domain/variable/api";
import {
  deletePlatformVariableRecord,
  listPlatformVariableRecords,
} from "@/domain/variable/mock/data";
import type { PlatformVariableRecord } from "@/domain/variable/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  SearchField,
  TablePagination,
  useTablePagination,
  useToast,
} from "@/shared";

import {
  formatPlatformVariableTimestamp,
  matchesPlatformVariableSearch,
} from "./platformVariablePageHelpers";

interface RowMenuState {
  left: number;
  top: number;
  triggerElement: HTMLButtonElement;
  variableKey: string;
}

export function PlatformVariablesPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const [records, setRecords] = useState<PlatformVariableRecord[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [rowBusyKey, setRowBusyKey] = useState("");
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);

  const filteredRecords = useMemo(
    () => records.filter((record) => matchesPlatformVariableSearch(record, query)),
    [query, records],
  );
  const pagination = useTablePagination(filteredRecords);
  const openMenuRow = openRowMenu
    ? records.find((record) => record.key === openRowMenu.variableKey) ?? null
    : null;

  async function refreshPlatformVariables() {
    setLoading(true);
    try {
      const nextRecords = dataSource === "api"
        ? await fetchPlatformVariables()
        : listPlatformVariableRecords();
      setRecords(nextRecords);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform variables could not be loaded");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    let active = true;

    async function loadPlatformVariables() {
      setLoading(true);
      try {
        const nextRecords = dataSource === "api"
          ? await fetchPlatformVariables()
          : listPlatformVariableRecords();
        if (active) setRecords(nextRecords);
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : "Platform variables could not be loaded");
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadPlatformVariables();

    return () => {
      active = false;
    };
  }, [dataSource, toast]);

  useEffect(() => {
    if (!openRowMenu) return undefined;

    const handleDocumentClick = (event: MouseEvent) => {
      const target = event.target as Node;
      if (rowMenuRef.current?.contains(target)) return;
      if (openRowMenu.triggerElement.contains(target)) return;
      setOpenRowMenu(null);
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenRowMenu(null);
    };

    document.addEventListener("click", handleDocumentClick);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("click", handleDocumentClick);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [openRowMenu]);

  function toggleRowMenu(row: PlatformVariableRecord, trigger: HTMLButtonElement) {
    if (rowBusyKey === row.key) return;
    if (openRowMenu?.variableKey === row.key) {
      setOpenRowMenu(null);
      return;
    }

    const menuWidth = 144;
    const estimatedMenuHeight = 104;
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
      left,
      top,
      triggerElement: trigger,
      variableKey: row.key,
    });
  }

  async function handleDelete(row: PlatformVariableRecord) {
    if (rowBusyKey === row.key) return;
    setRowBusyKey(row.key);
    setOpenRowMenu(null);

    try {
      if (dataSource === "api") {
        await deletePlatformVariable(row.key);
      } else {
        deletePlatformVariableRecord(row.key);
      }
      toast.success(`${row.key} deleted`);
      await refreshPlatformVariables();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform variable could not be deleted");
    } finally {
      setRowBusyKey("");
    }
  }

  return (
    <>
      <PageHeader
        title="Platform Variables"
        actions={(
          <>
            <Button variant="secondary" disabled={loading} aria-busy={loading} onClick={() => void refreshPlatformVariables()}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? "Refreshing..." : "Refresh"}
            </Button>
            <Button onClick={() => navigate("/settings/variables/new")}>
              <Plus size={16} aria-hidden="true" />
              New
            </Button>
          </>
        )}
      />

      <DataTable<PlatformVariableRecord>
        rows={pagination.visibleItems}
        rowKey={(row) => row.key}
        onRowClick={(row) => navigate(`/settings/variables/detail?variableKey=${encodeURIComponent(row.key)}`)}
        pagination={<TablePagination itemLabel="platform variables" {...pagination.paginationProps} />}
        emptyState="No platform variables match the current filters."
        toolbar={(
          <SearchField
            ariaLabel="Search platform variables"
            className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
            placeholder="Search platform variables..."
            value={query}
            onChange={setQuery}
          />
        )}
        columns={[
          {
            id: "key",
            header: "Key",
            width: "30%",
            render: (row) => (
              <div>
                <p className="font-mono text-[13px] font-medium text-[var(--color-on-surface)]">{row.key}</p>
                {row.description ? (
                  <p className="mt-1 line-clamp-1 text-[13px] text-[var(--color-on-surface-variant)]">{row.description}</p>
                ) : null}
              </div>
            ),
          },
          {
            id: "value",
            header: "Value",
            width: "30%",
            render: (row) => <span className="block max-w-[28rem] truncate font-mono text-[13px]">{row.value}</span>,
          },
          {
            id: "updatedAt",
            header: "Updated",
            width: "30%",
            render: (row) => formatPlatformVariableTimestamp(row.updatedAt),
          },
          {
            align: "right",
            cellClassName: "w-16",
            header: "Actions",
            headerClassName: "w-16",
            id: "actions",
            width: "10%",
            render: (row) => {
              const isMenuOpen = openRowMenu?.variableKey === row.key;
              const isBusy = rowBusyKey === row.key;

              return (
                <div className="flex justify-end">
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label={`Open actions for ${row.key}`}
                    aria-expanded={isMenuOpen}
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
            className="fixed z-[80] w-36 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={`Actions for ${openMenuRow.key}`}
            onClick={(event) => event.stopPropagation()}
          >
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              onClick={() => {
                setOpenRowMenu(null);
                navigate(`/settings/variables/edit?variableKey=${encodeURIComponent(openMenuRow.key)}`);
              }}
            >
              Edit
            </button>
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyKey === openMenuRow.key}
              onClick={() => {
                void handleDelete(openMenuRow);
              }}
            >
              Delete
            </button>
          </div>,
          document.body,
        )
        : null}
    </>
  );
}

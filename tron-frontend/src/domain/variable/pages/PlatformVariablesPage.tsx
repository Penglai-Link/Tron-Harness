import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
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
  revealPlatformVariable,
  verifyStepUpPassword,
} from "@/domain/variable/api";
import {
  deletePlatformVariableRecord,
  listPlatformVariableRecords,
  revealPlatformVariableRecord,
  verifyStepUpPasswordRecord,
} from "@/domain/variable/mock/data";
import type { PlatformVariableRecord } from "@/domain/variable/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  Field,
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  TablePagination,
  useTablePagination,
  useToast,
} from "@/shared";

import {
  SensitiveDataBadge,
  SensitiveRevealIconButton,
  SensitiveValueCode,
  StepUpRevealDialog,
  formatPlatformVariableTimestamp,
  getSensitiveDisplayValue,
  getSensitiveState,
  isSensitiveRecord,
  matchesPlatformVariableSearch,
} from "./platformVariablePageHelpers";
import { SelectItem } from "@/components/ui/select";

interface RowMenuState {
  left: number;
  top: number;
  triggerElement: HTMLButtonElement;
  variableKey: string;
}

type PlatformSensitiveFilter = "All" | "Sensitive" | "Plain";

const platformSensitiveFilters = ["All", "Sensitive", "Plain"] as const satisfies readonly PlatformSensitiveFilter[];

export function PlatformVariablesPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`variablesPlatform.${key}`, { defaultValue, ...options })
  );
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const [records, setRecords] = useState<PlatformVariableRecord[]>([]);
  const [query, setQuery] = useState("");
  const [sensitiveFilter, setSensitiveFilter] = useState<PlatformSensitiveFilter>("All");
  const [loading, setLoading] = useState(false);
  const [rowBusyKey, setRowBusyKey] = useState("");
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const [revealedValues, setRevealedValues] = useState<Record<string, string>>({});
  const [revealTargetKey, setRevealTargetKey] = useState("");
  const [revealPending, setRevealPending] = useState(false);
  const [revealError, setRevealError] = useState("");
  const rowMenuRef = useRef<HTMLDivElement | null>(null);

  const filteredRecords = useMemo(
    () => records.filter((record) => {
      const matchesSensitive = sensitiveFilter === "All"
        || (sensitiveFilter === "Sensitive" && isSensitiveRecord(record))
        || (sensitiveFilter === "Plain" && !isSensitiveRecord(record));
      return matchesSensitive && matchesPlatformVariableSearch(record, query);
    }),
    [query, records, sensitiveFilter],
  );
  const pagination = useTablePagination(filteredRecords);
  const openMenuRow = openRowMenu
    ? records.find((record) => record.key === openRowMenu.variableKey) ?? null
    : null;
  const revealTarget = revealTargetKey
    ? records.find((record) => record.key === revealTargetKey) ?? null
    : null;

  async function refreshPlatformVariables() {
    setLoading(true);
    try {
      const nextRecords = dataSource === "api"
        ? await fetchPlatformVariables()
        : listPlatformVariableRecords();
      setRecords(nextRecords);
      setRevealedValues({});
      setRevealTargetKey("");
      setRevealError("");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform variables could not be loaded"));
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
        if (active) {
          setRevealedValues({});
          setRevealTargetKey("");
          setRevealError("");
        }
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform variables could not be loaded"));
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
      setRevealedValues((current) => {
        const next = { ...current };
        delete next[row.key];
        return next;
      });
      toast.success(pageT("messages.deleted", "{{key}} deleted", { key: row.key }));
      await refreshPlatformVariables();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.delete", "Platform variable could not be deleted"));
    } finally {
      setRowBusyKey("");
    }
  }

  function sensitiveFilterLabel(filter: PlatformSensitiveFilter) {
    if (filter === "Sensitive") return pageT("filters.sensitiveData", "Sensitive Data");
    if (filter === "Plain") return pageT("filters.plainData", "Plain Data");
    return pageT("filters.allData", "All Data");
  }

  function sensitiveFilterValue(filter: string): PlatformSensitiveFilter {
    return platformSensitiveFilters.includes(filter as PlatformSensitiveFilter)
      ? filter as PlatformSensitiveFilter
      : "All";
  }

  function hideSensitiveValue(record: PlatformVariableRecord) {
    setRevealedValues((current) => {
      const next = { ...current };
      delete next[record.key];
      return next;
    });
  }

  function openSensitiveReveal(record: PlatformVariableRecord) {
    if (!isSensitiveRecord(record)) return;
    setRevealTargetKey(record.key);
    setRevealError("");
  }

  async function revealSensitiveValue(password: string) {
    if (!revealTarget) return;

    setRevealPending(true);
    setRevealError("");
    try {
      const stepUp = dataSource === "api"
        ? await verifyStepUpPassword({
          password,
          target: {
            scope: "platform",
            variableKey: revealTarget.key,
          },
        })
        : verifyStepUpPasswordRecord({
          password,
          target: {
            scope: "platform",
            variableKey: revealTarget.key,
          },
        });
      const revealed = dataSource === "api"
        ? await revealPlatformVariable({
          variableKey: revealTarget.key,
          stepUpToken: stepUp.stepUpToken,
        })
        : revealPlatformVariableRecord(revealTarget.key, stepUp.stepUpToken);

      setRevealedValues((current) => ({
        ...current,
        [revealTarget.key]: revealed.value,
      }));
      setRevealTargetKey("");
      setRevealError("");
    } catch (error) {
      setRevealError(error instanceof Error ? error.message : pageT("errors.reveal", "Sensitive value could not be revealed"));
    } finally {
      setRevealPending(false);
    }
  }

  return (
    <>
      <PageHeader
        title={pageT("title", "Platform Variables")}
        actions={(
          <>
            <Button variant="secondary" disabled={loading} aria-busy={loading} onClick={() => void refreshPlatformVariables()}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? pageT("actions.refreshing", "Refreshing...") : pageT("actions.refresh", "Refresh")}
            </Button>
            <Button onClick={() => navigate("/settings/variables/new")}>
              <Plus size={16} aria-hidden="true" />
              {pageT("actions.new", "New")}
            </Button>
          </>
        )}
      />

      <div className="space-y-5">
        <SurfaceCard className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <Field label={pageT("fields.search", "Search")}>
              <SearchField
                ariaLabel={pageT("aria.search", "Search platform variables")}
                className="w-full"
                placeholder={pageT("filters.searchPlaceholder", "Search platform variables...")}
                value={query}
                onChange={setQuery}
              />
            </Field>
            <Field label={pageT("fields.sensitive", "Sensitive Data")}>
              <SelectInput
                aria-label={pageT("aria.sensitiveFilter", "Filter platform variables by sensitive data")}
                className="w-full"
                value={sensitiveFilter}
                onValueChange={(event) => setSensitiveFilter(sensitiveFilterValue(event!))}
              >
                {platformSensitiveFilters.map((filter) => (
                  <SelectItem key={filter} value={filter}>
                    {sensitiveFilterLabel(filter)}
                  </SelectItem>
                ))}
              </SelectInput>
            </Field>
          </div>
        </SurfaceCard>

        <DataTable<PlatformVariableRecord>
          rows={pagination.visibleItems}
          rowKey={(row) => row.key}
          tableClassName="table-fixed min-w-[1080px]"
          onRowClick={(row) => navigate(`/settings/variables/detail?variableKey=${encodeURIComponent(row.key)}`)}
          pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "platform variables")} {...pagination.paginationProps} />}
          emptyState={pageT("empty.filtered", "No platform variables match the current filters.")}
          columns={[
            {
              id: "key",
              header: pageT("columns.key", "Key"),
              width: "310px",
              cellClassName: "min-w-0",
              render: (row) => (
                <div className="min-w-0 max-h-11 overflow-hidden">
                  <p className="truncate font-mono text-[13px] font-medium text-[var(--color-on-surface)]" title={row.key}>{row.key}</p>
                  {row.description ? (
                    <p className="mt-1 truncate text-[13px] text-[var(--color-on-surface-variant)]" title={row.description}>{row.description}</p>
                  ) : null}
                </div>
              ),
            },
            {
              id: "value",
              header: pageT("columns.value", "Value"),
              width: "310px",
              cellClassName: "min-w-0",
              render: (row) => {
                const revealedValue = revealedValues[row.key];
                const sensitive = isSensitiveRecord(row);
                return (
                  <div className="flex min-w-0 max-w-full items-center gap-2">
                    <span className="min-w-0 flex-1 truncate font-mono text-[13px]">
                      <SensitiveValueCode
                        className="!block !truncate !whitespace-nowrap !text-[13px] !leading-5"
                        state={getSensitiveState(sensitive, revealedValue)}
                        testId={`platform-list-sensitive-value-${row.key}`}
                      >
                        {getSensitiveDisplayValue(row, revealedValue)}
                      </SensitiveValueCode>
                    </span>
                    {sensitive ? (
                      <SensitiveRevealIconButton
                        revealed={Boolean(revealedValue)}
                        variableKey={row.key}
                        onClick={(event) => {
                          event.stopPropagation();
                          if (revealedValue) {
                            hideSensitiveValue(row);
                          } else {
                            openSensitiveReveal(row);
                          }
                        }}
                      />
                    ) : null}
                  </div>
                );
              },
            },
            {
              id: "sensitive",
              header: pageT("columns.sensitive", "Sensitive Data"),
              width: "150px",
              render: (row) => (
                <SensitiveDataBadge sensitive={isSensitiveRecord(row)} />
              ),
            },
            {
              id: "updatedAt",
              header: pageT("columns.updated", "Updated"),
              width: "210px",
              render: (row) => (
                <span className="whitespace-nowrap text-[13px] text-[var(--color-on-surface)]">
                  {formatPlatformVariableTimestamp(row.updatedAt)}
                </span>
              ),
            },
            {
              align: "right",
              cellClassName: "sticky right-0 z-10 w-24 border-l border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[-8px_0_12px_rgba(0,0,0,0.04)] group-hover:bg-[var(--color-hover-surface)]",
              header: "Actions",
              headerClassName: "sticky right-0 z-20 w-24 border-l border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] shadow-[-8px_0_12px_rgba(0,0,0,0.04)]",
              id: "actions",
              width: "100px",
              render: (row) => {
                const isMenuOpen = openRowMenu?.variableKey === row.key;
                const isBusy = rowBusyKey === row.key;

                return (
                  <div className="flex justify-end">
                    <Button
                      size="icon"
                      variant="ghost"
                      aria-label={pageT("aria.actionsFor", "Open actions for {{key}}", { key: row.key })}
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
      </div>
      {openRowMenu && openMenuRow
        ? createPortal(
          <div
            ref={rowMenuRef}
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
            className="fixed z-[80] w-36 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={pageT("aria.menu", "Actions for {{key}}", { key: openMenuRow.key })}
            onClick={(event) => event.stopPropagation()}
          >
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              onClick={() => {
                setOpenRowMenu(null);
                navigate(`/settings/variables/detail?variableKey=${encodeURIComponent(openMenuRow.key)}&mode=edit`);
              }}
            >
              {pageT("actions.edit", "Edit")}
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
              {pageT("actions.delete", "Delete")}
            </button>
          </div>,
          document.body,
        )
        : null}
      {revealTarget ? (
        <StepUpRevealDialog
          error={revealError}
          onCancel={() => {
            setRevealTargetKey("");
            setRevealError("");
          }}
          onSubmit={(password) => void revealSensitiveValue(password)}
          open={Boolean(revealTarget)}
          pending={revealPending}
          variableKey={revealTarget.key}
        />
      ) : null}
    </>
  );
}

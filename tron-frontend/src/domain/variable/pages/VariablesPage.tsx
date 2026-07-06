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
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/components/ui/button";
import {
  DataTable,
  Field,
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  TablePagination,
  useDismissibleLayer,
  useTablePagination,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import {
  deleteVariable,
  revealWorkspaceVariable,
  verifyStepUpPassword,
} from "@/domain/variable/api";
import { getVariableSourceTone } from "@/domain/variable/constants";
import { useVariableCatalog } from "@/domain/variable/hooks";
import {
  deleteVariableRecord,
  revealWorkspaceVariableRecord,
  verifyStepUpPasswordRecord,
} from "@/domain/variable/mock/data";
import type { VariableRecord } from "@/domain/variable/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  SensitiveDataBadge,
  SensitiveRevealIconButton,
  SensitiveValueCode,
  StepUpRevealDialog,
  getSensitiveDisplayValue,
  getSensitiveState,
  isSensitiveRecord,
} from "./platformVariablePageHelpers";
import {
  formatWorkspaceVariableTimestamp,
  getWorkspaceVariableInheritedFrom,
  isWorkspaceVariableRemovable,
  matchesWorkspaceVariableSearch,
} from "./workspaceVariablePageHelpers";
import { SelectItem } from "@/components/ui/select";

type VariableSourceFilter = VariableRecord["sourceKind"] | "All";
type VariableSensitiveFilter = "All" | "Sensitive" | "Plain";

const variableSourceFilters = ["All", "Local", "Inherited", "Override"] as const satisfies readonly VariableSourceFilter[];
const variableSensitiveFilters = ["All", "Sensitive", "Plain"] as const satisfies readonly VariableSensitiveFilter[];

interface RowMenuState {
  key: string;
  left: number;
  top: number;
  triggerElement: HTMLButtonElement;
}

export function VariablesPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const { loading, refresh, variables } = useVariableCatalog(activeWorkspaceCode);
  const [search, setSearch] = useState("");
  const [sourceFilter, setSourceFilter] = useState<VariableSourceFilter>("All");
  const [sensitiveFilter, setSensitiveFilter] = useState<VariableSensitiveFilter>("All");
  const [rowBusyKey, setRowBusyKey] = useState("");
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const [revealedValues, setRevealedValues] = useState<Record<string, string>>({});
  const [revealTargetKey, setRevealTargetKey] = useState("");
  const [revealPending, setRevealPending] = useState(false);
  const [revealError, setRevealError] = useState("");
  const rowMenuRef = useRef<HTMLDivElement | null>(null);

  const openMenuVariable = openRowMenu
    ? variables.find((record) => record.key === openRowMenu.key) ?? null
    : null;
  const revealTarget = revealTargetKey
    ? variables.find((record) => record.key === revealTargetKey) ?? null
    : null;

  const filteredVariables = useMemo(() => variables.filter((record) => {
    const matchesSource = sourceFilter === "All" || record.sourceKind === sourceFilter;
    const matchesSensitive = sensitiveFilter === "All"
      || (sensitiveFilter === "Sensitive" && isSensitiveRecord(record))
      || (sensitiveFilter === "Plain" && !isSensitiveRecord(record));
    return matchesSource && matchesSensitive && matchesWorkspaceVariableSearch(record, search);
  }), [search, sensitiveFilter, sourceFilter, variables]);
  const pagination = useTablePagination(filteredVariables);

  useEffect(() => {
    setRevealedValues({});
    setRevealTargetKey("");
    setRevealError("");
  }, [activeWorkspaceCode]);

  useDismissibleLayer({
    onDismiss: () => setOpenRowMenu(null),
    open: openRowMenu !== null,
    popupRef: rowMenuRef,
    triggerElement: openRowMenu?.triggerElement,
  });

  function navigateToDetail(record: VariableRecord, mode?: "edit" | "override") {
    const params = new URLSearchParams({ variableKey: record.key });
    if (mode) params.set("mode", mode);
    navigate(`/variables/detail?${params.toString()}`);
  }

  function refreshAndClearRevealedValues() {
    setRevealedValues({});
    refresh();
  }

  function toggleRowMenu(row: VariableRecord, trigger: HTMLButtonElement) {
    if (rowBusyKey === row.key) return;
    if (openRowMenu?.key === row.key) {
      setOpenRowMenu(null);
      return;
    }

    const menuWidth = 144;
    const estimatedMenuHeight = 112;
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
      key: row.key,
      left,
      top,
      triggerElement: trigger,
    });
  }

  async function removeVariableRow(record: VariableRecord) {
    if (!activeWorkspaceCode || rowBusyKey === record.key || !isWorkspaceVariableRemovable(record)) return;
    setRowBusyKey(record.key);
    setOpenRowMenu(null);
    try {
      if (dataSource === "api") {
        await deleteVariable(activeWorkspaceCode, record.key);
      } else {
        deleteVariableRecord(activeWorkspaceCode, record.key);
      }
      setRevealedValues((current) => {
        const next = { ...current };
        delete next[record.key];
        return next;
      });
      toast.success(record.sourceKind === "Override" ? `${record.key} reverted to inherited value` : `${record.key} deleted`);
      refresh();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Variable could not be removed";
      toast.error(message);
    } finally {
      setRowBusyKey("");
    }
  }

  function sourceFilterLabel(source: VariableSourceFilter) {
    return source === "All" ? "All Sources" : source;
  }

  function sourceFilterValue(source: string): VariableSourceFilter {
    return variableSourceFilters.includes(source as VariableSourceFilter)
      ? source as VariableSourceFilter
      : "All";
  }

  function sensitiveFilterLabel(filter: VariableSensitiveFilter) {
    if (filter === "Sensitive") return "Sensitive Data";
    if (filter === "Plain") return "Plain Data";
    return "All Data";
  }

  function sensitiveFilterValue(filter: string): VariableSensitiveFilter {
    return variableSensitiveFilters.includes(filter as VariableSensitiveFilter)
      ? filter as VariableSensitiveFilter
      : "All";
  }

  function hideSensitiveValue(record: VariableRecord) {
    setRevealedValues((current) => {
      const next = { ...current };
      delete next[record.key];
      return next;
    });
  }

  function openSensitiveReveal(record: VariableRecord) {
    if (!activeWorkspaceCode || !isSensitiveRecord(record)) return;
    setRevealTargetKey(record.key);
    setRevealError("");
  }

  async function revealSensitiveValue(password: string) {
    if (!activeWorkspaceCode || !revealTarget) return;

    setRevealPending(true);
    setRevealError("");
    try {
      const stepUp = dataSource === "api"
        ? await verifyStepUpPassword({
          password,
          target: {
            scope: "workspace",
            workspaceCode: activeWorkspaceCode,
            variableKey: revealTarget.key,
          },
        })
        : verifyStepUpPasswordRecord({
          password,
          target: {
            scope: "workspace",
            workspaceCode: activeWorkspaceCode,
            variableKey: revealTarget.key,
          },
        });
      const revealed = dataSource === "api"
        ? await revealWorkspaceVariable({
          workspaceCode: activeWorkspaceCode,
          variableKey: revealTarget.key,
          stepUpToken: stepUp.stepUpToken,
        })
        : revealWorkspaceVariableRecord(activeWorkspaceCode, revealTarget.key, stepUp.stepUpToken);

      setRevealedValues((current) => ({
        ...current,
        [revealTarget.key]: revealed.value,
      }));
      setRevealTargetKey("");
      setRevealError("");
    } catch (error) {
      setRevealError(error instanceof Error ? error.message : "Sensitive value could not be revealed");
    } finally {
      setRevealPending(false);
    }
  }

  return (
    <>
      <PageHeader
        title="Variables"
        actions={(
          <>
            <Button size="lg" variant="outline" disabled={loading} aria-busy={loading} onClick={refreshAndClearRevealedValues}>
              {loading ? "Refreshing..." : "Refresh"}
            </Button>
            <Button size="lg" disabled={!activeWorkspaceCode} onClick={() => navigate("/variables/new")}>
              New
            </Button>
          </>
        )}
      />

      <div className="space-y-5">
        <SurfaceCard className="space-y-4">
          <div className="grid gap-4 md:grid-cols-3">
            <Field label="Search">
              <SearchField
                ariaLabel="Search variables"
                className="w-full"
                placeholder="Search variables"
                value={search}
                onChange={setSearch}
              />
            </Field>
            <Field label="Source">
              <SelectInput
                aria-label="Filter variables by source"
                className="w-full"
                value={sourceFilter}
                onValueChange={(event) => setSourceFilter(sourceFilterValue(event!))}
              >
                {variableSourceFilters.map((source) => (
                  <SelectItem key={source} value={source}>
                    {sourceFilterLabel(source)}
                  </SelectItem>
                ))}
              </SelectInput>
            </Field>
            <Field label="Sensitive Data">
              <SelectInput
                aria-label="Filter variables by sensitive data"
                className="w-full"
                value={sensitiveFilter}
                onValueChange={(event) => setSensitiveFilter(sensitiveFilterValue(event!))}
              >
                {variableSensitiveFilters.map((filter) => (
                  <SelectItem key={filter} value={filter}>
                    {sensitiveFilterLabel(filter)}
                  </SelectItem>
                ))}
              </SelectInput>
            </Field>
          </div>
        </SurfaceCard>

        <DataTable<VariableRecord>
          rows={pagination.visibleItems}
          rowKey={(row) => row.key}
          tableClassName="table-fixed min-w-[1360px]"
          onRowClick={(row) => navigateToDetail(row)}
          pagination={<TablePagination itemLabel="variables" {...pagination.paginationProps} />}
          emptyState={activeWorkspaceCode ? "No effective variables match the current filters." : "Select a workspace in the shell to load variables."}
          columns={[
            {
              id: "key",
              header: "Key",
              width: "260px",
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
              header: "Value",
              width: "300px",
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
                        testId={`workspace-list-sensitive-value-${row.key}`}
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
              header: "Sensitive Data",
              width: "150px",
              render: (row) => (
                <SensitiveDataBadge sensitive={isSensitiveRecord(row)} />
              ),
            },
            {
              id: "sourceKind",
              header: "Source",
              width: "130px",
              render: (row) => (
                <Badge tone={getVariableSourceTone(row.sourceKind)}>{row.sourceKind}</Badge>
              ),
            },
            {
              id: "inheritedFrom",
              header: "Inherited From",
              width: "230px",
              cellClassName: "min-w-0",
              render: (row) => {
                const inheritedWorkspace = getWorkspaceVariableInheritedFrom(row);
                if (!inheritedWorkspace) {
                  return (
                    <span className="text-[13px] text-[var(--color-on-surface-variant)]">-</span>
                  );
                }

                return (
                  <div className="min-w-0 max-h-11 overflow-hidden">
                    <p className="truncate font-medium text-[var(--color-on-surface)]">
                      {inheritedWorkspace.label}
                    </p>
                    {inheritedWorkspace.code ? (
                      <p className="mt-1 truncate text-[13px] text-[var(--color-on-surface-variant)]">
                        {inheritedWorkspace.code}
                      </p>
                    ) : null}
                  </div>
                );
              },
            },
            {
              id: "updatedAt",
              header: "Updated",
              width: "190px",
              render: (row) => (
                <span className="whitespace-nowrap text-[13px] text-[var(--color-on-surface)]">
                  {formatWorkspaceVariableTimestamp(row.updatedAt)}
                </span>
              ),
            },
            {
              align: "right",
              header: "Actions",
              headerClassName: "sticky right-0 z-20 w-24 border-l border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] shadow-[-8px_0_12px_rgba(0,0,0,0.04)]",
              id: "actions",
              cellClassName: "sticky right-0 z-10 w-24 border-l border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[-8px_0_12px_rgba(0,0,0,0.04)] group-hover:bg-[var(--color-hover-surface)]",
              width: "100px",
              render: (row) => {
                const isMenuOpen = openRowMenu?.key === row.key;
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
      </div>

      {openRowMenu && openMenuVariable
        ? createPortal(
          <div
            ref={rowMenuRef}
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
            className="fixed z-[80] w-36 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={`Actions for ${openMenuVariable.key}`}
          >
            {openMenuVariable.sourceKind === "Inherited" ? (
              <button
                type="button"
                role="menuitem"
                className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
                onClick={() => {
                  setOpenRowMenu(null);
                  navigateToDetail(openMenuVariable, "override");
                }}
              >
                Override
              </button>
            ) : (
              <>
                <button
                  type="button"
                  role="menuitem"
                  className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
                  onClick={() => {
                    setOpenRowMenu(null);
                    navigateToDetail(openMenuVariable, "edit");
                  }}
                >
                  Edit
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                  disabled={rowBusyKey === openMenuVariable.key}
                  onClick={() => {
                    void removeVariableRow(openMenuVariable);
                  }}
                >
                  {openMenuVariable.sourceKind === "Override" ? "Revert" : "Delete"}
                </button>
              </>
            )}
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

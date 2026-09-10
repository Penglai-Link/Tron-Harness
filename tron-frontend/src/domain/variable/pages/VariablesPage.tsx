import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  getCoreRowModel,
  getPaginationRowModel,
  useReactTable,
} from "@tanstack/react-table";
import { MoreHorizontal } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SelectInput } from "@/components/forms";
import { SearchField } from "@/components/search-field";
import {
  useWorkspaceSpaces,
} from "@/shared";
import { useToast } from "@/components/ui/toast";
import {
  DataTable,
  TablePagination,
  type TableColumnDef,
} from "@/components/table";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

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

const variableSourceFilters = [
  "All",
  "Local",
  "Inherited",
  "Override",
] as const satisfies readonly VariableSourceFilter[];
const variableSensitiveFilters = [
  "All",
  "Sensitive",
  "Plain",
] as const satisfies readonly VariableSensitiveFilter[];

export function VariablesPage() {
  const navigate = useNavigate();
  const { t, i18n } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`variables.${key}`, { defaultValue, ...options })
  );
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const { loading, refresh, variables } =
    useVariableCatalog(activeWorkspaceCode);
  const [search, setSearch] = useState("");
  const [sourceFilter, setSourceFilter] = useState<VariableSourceFilter>("All");
  const [sensitiveFilter, setSensitiveFilter] =
    useState<VariableSensitiveFilter>("All");
  const [rowBusyKey, setRowBusyKey] = useState("");
  const [revealedValues, setRevealedValues] = useState<Record<string, string>>(
    {},
  );
  const [revealTargetKey, setRevealTargetKey] = useState("");
  const [revealPending, setRevealPending] = useState(false);
  const [revealError, setRevealError] = useState("");
  const revealTarget = revealTargetKey
    ? variables.find((record) => record.key === revealTargetKey) ?? null
    : null;

  const filteredVariables = useMemo(
    () =>
      variables.filter((record) => {
        const matchesSource =
          sourceFilter === "All" || record.sourceKind === sourceFilter;
        const matchesSensitive =
          sensitiveFilter === "All" ||
          (sensitiveFilter === "Sensitive" && isSensitiveRecord(record)) ||
          (sensitiveFilter === "Plain" && !isSensitiveRecord(record));
        return (
          matchesSource &&
          matchesSensitive &&
          matchesWorkspaceVariableSearch(record, search)
        );
      }),
    [search, sensitiveFilter, sourceFilter, variables],
  );

  useEffect(() => {
    setRevealedValues({});
    setRevealTargetKey("");
    setRevealError("");
  }, [activeWorkspaceCode]);

  function navigateToDetail(
    record: VariableRecord,
    mode?: "edit" | "override",
  ) {
    const params = new URLSearchParams({ variableKey: record.key });
    if (mode) params.set("mode", mode);
    navigate(`/variables/detail?${params.toString()}`);
  }


  async function removeVariableRow(record: VariableRecord) {
    if (
      !activeWorkspaceCode ||
      rowBusyKey === record.key ||
      !isWorkspaceVariableRemovable(record)
    )
      return;
    setRowBusyKey(record.key);
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
      toast.success(
        record.sourceKind === "Override"
          ? pageT("messages.reverted", "{{key}} reverted to inherited value", { key: record.key })
          : pageT("messages.deleted", "{{key}} deleted", { key: record.key }),
      );
      refresh();
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : pageT("errors.remove", "Variable could not be removed");
      toast.error(message);
    } finally {
      setRowBusyKey("");
    }
  }

  function sourceFilterLabel(source: VariableSourceFilter) {
    return source === "All" ? pageT("filters.allSources", "All Sources") : pageT(`sources.${source}`, source);
  }

  function sourceFilterValue(source: string): VariableSourceFilter {
    return variableSourceFilters.includes(source as VariableSourceFilter)
      ? (source as VariableSourceFilter)
      : "All";
  }

  function sensitiveFilterLabel(filter: VariableSensitiveFilter) {
    if (filter === "Sensitive") return pageT("filters.sensitiveData", "Sensitive Data");
    if (filter === "Plain") return pageT("filters.plainData", "Plain Data");
    return pageT("filters.allData", "All Data");
  }

  function sensitiveFilterValue(filter: string): VariableSensitiveFilter {
    return variableSensitiveFilters.includes(filter as VariableSensitiveFilter)
      ? (filter as VariableSensitiveFilter)
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
      const stepUp =
        dataSource === "api"
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
      const revealed =
        dataSource === "api"
          ? await revealWorkspaceVariable({
              workspaceCode: activeWorkspaceCode,
              variableKey: revealTarget.key,
              stepUpToken: stepUp.stepUpToken,
            })
          : revealWorkspaceVariableRecord(
              activeWorkspaceCode,
              revealTarget.key,
              stepUp.stepUpToken,
            );

      setRevealedValues((current) => ({
        ...current,
        [revealTarget.key]: revealed.value,
      }));
      setRevealTargetKey("");
      setRevealError("");
    } catch (error) {
      setRevealError(
        error instanceof Error
          ? error.message
          : pageT("errors.reveal", "Sensitive value could not be revealed"),
      );
    } finally {
      setRevealPending(false);
    }
  }

  const columns = useMemo<TableColumnDef<VariableRecord>[]>(
    () => [
      {
        accessorKey: "key",
        cell: ({ row }) => (
          <div className="min-w-0 max-h-11 overflow-hidden">
            <p
              className="truncate font-mono text-[13px] font-medium text-[var(--color-on-surface)]"
              title={row.original.key}
            >
              {row.original.key}
            </p>
            {row.original.description ? (
              <p
                className="mt-1 truncate text-[13px] text-[var(--color-on-surface-variant)]"
                title={row.original.description}
              >
                {row.original.description}
              </p>
            ) : null}
          </div>
        ),
        header: pageT("columns.key", "Key"),
        id: "key",
        meta: {
          cellClassName: "min-w-0",
          width: "260px",
        },
      },
      {
        cell: ({ row }) => {
          const revealedValue = revealedValues[row.original.key];
          const sensitive = isSensitiveRecord(row.original);
          return (
            <div className="flex min-w-0 max-w-full items-center gap-[12px]">
              <span className="min-w-0 flex-1 truncate font-mono text-[13px]">
                <SensitiveValueCode
                  className="!block !truncate !whitespace-nowrap !text-[13px] !leading-5"
                  state={getSensitiveState(sensitive, revealedValue)}
                  testId={`workspace-list-sensitive-value-${row.original.key}`}
                >
                  {getSensitiveDisplayValue(row.original, revealedValue)}
                </SensitiveValueCode>
              </span>
              {sensitive ? (
                <SensitiveRevealIconButton
                  revealed={Boolean(revealedValue)}
                  variableKey={row.original.key}
                  onClick={(event) => {
                    event.stopPropagation();
                    if (revealedValue) {
                      hideSensitiveValue(row.original);
                    } else {
                      openSensitiveReveal(row.original);
                    }
                  }}
                />
              ) : null}
            </div>
          );
        },
        header: pageT("columns.value", "Value"),
        id: "value",
        meta: {
          cellClassName: "min-w-0",
          width: "300px",
        },
      },
      {
        cell: ({ row }) => (
          <SensitiveDataBadge sensitive={isSensitiveRecord(row.original)} />
        ),
        header: pageT("columns.sensitive", "Sensitive Data"),
        id: "sensitive",
        meta: { width: "150px" },
      },
      {
        cell: ({ row }) => (
          <Badge variant={getVariableSourceTone(row.original.sourceKind)}>
            {row.original.sourceKind}
          </Badge>
        ),
        header: pageT("columns.source", "Source"),
        id: "sourceKind",
        meta: { width: "130px" },
      },
      {
        cell: ({ row }) => {
          const inheritedWorkspace = getWorkspaceVariableInheritedFrom(
            row.original,
          );
          if (!inheritedWorkspace) {
            return (
              <span className="text-[13px] text-[var(--color-on-surface-variant)]">
                -
              </span>
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
        header: pageT("columns.inheritedFrom", "Inherited From"),
        id: "inheritedFrom",
        meta: {
          cellClassName: "min-w-0",
          width: "230px",
        },
      },
      {
        cell: ({ row }) => (
          <span className="whitespace-nowrap text-[13px] text-[var(--color-on-surface)]">
            {formatWorkspaceVariableTimestamp(row.original.updatedAt)}
          </span>
        ),
        header: pageT("columns.updated", "Updated"),
        id: "updatedAt",
        meta: { width: "190px" },
      },
      {
        cell: ({ row }) => {
          const isBusy = rowBusyKey === row.original.key;

          return (
            <div className="flex justify-end">
              <DropdownMenu>
                <DropdownMenuTrigger
                  render={(
                    <Button
                      size="icon"
                      variant="ghost"
                      aria-label={pageT("aria.actionsFor", "Open actions for {{key}}", { key: row.original.key })}
                      disabled={isBusy}
                      onClick={(event) => event.stopPropagation()}
                      onKeyDown={(event) => event.stopPropagation()}
                    >
                      <MoreHorizontal size={16} aria-hidden="true" />
                    </Button>
                  )}
                />
                <DropdownMenuContent align="end" className="w-36">
                  {row.original.sourceKind === "Inherited" ? (
                    <DropdownMenuItem
                      onClick={(event) => {
                        event.stopPropagation();
                        navigateToDetail(row.original, "override");
                      }}
                    >
                      {pageT("actions.override", "Override")}
                    </DropdownMenuItem>
                  ) : (
                    <>
                      <DropdownMenuItem
                        onClick={(event) => {
                          event.stopPropagation();
                          navigateToDetail(row.original, "edit");
                        }}
                      >
                        {pageT("actions.edit", "Edit")}
                      </DropdownMenuItem>
                      <DropdownMenuItem
                        variant="destructive"
                        disabled={isBusy}
                        onClick={(event) => {
                          event.stopPropagation();
                          void removeVariableRow(row.original);
                        }}
                      >
                        {row.original.sourceKind === "Override"
                          ? pageT("actions.revert", "Revert")
                          : pageT("actions.delete", "Delete")}
                      </DropdownMenuItem>
                    </>
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          );
        },
        header: pageT("columns.actions", "Actions"),
        id: "actions",
        meta: {
          align: "right",
          cellClassName:
            "border-l border-[var(--color-outline-variant)] group-hover:bg-[var(--color-hover-surface)]",
          fixed: "right",
          headerClassName: "border-l border-[var(--color-outline-variant)]",
          width: "100px",
        },
      },
    ],
    [activeWorkspaceCode, dataSource, i18n.language, revealedValues, rowBusyKey],
  );
  const table = useReactTable({
    columns,
    data: filteredVariables,
    getCoreRowModel: getCoreRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    getRowId: (row) => row.key,
    initialState: {
      pagination: { pageSize: 15 },
    },
  });

  return (
    <>
      <>
        <DataTable<VariableRecord>
          title={pageT("title", "Variables")}
          toolbar={
            <div className="flex gap-[12px]">
              <SearchField
                ariaLabel={pageT("aria.search", "Search variables")}
                className="w-full"
                placeholder={pageT("filters.searchPlaceholder", "Search variables")}
                value={search}
                onChange={setSearch}
              />
              <SelectInput
                aria-label={pageT("aria.sourceFilter", "Filter variables by source")}
                className="w-full"
                value={sourceFilter}
                onValueChange={(event) =>
                  setSourceFilter(sourceFilterValue(event!))
                }
              >
                {variableSourceFilters.map((source) => (
                  <SelectItem key={source} value={source}>
                    {sourceFilterLabel(source)}
                  </SelectItem>
                ))}
              </SelectInput>
              <SelectInput
                aria-label={pageT("aria.sensitiveFilter", "Filter variables by sensitive data")}
                className="w-full"
                value={sensitiveFilter}
                onValueChange={(event) =>
                  setSensitiveFilter(sensitiveFilterValue(event!))
                }
              >
                {variableSensitiveFilters.map((filter) => (
                  <SelectItem key={filter} value={filter}>
                    {sensitiveFilterLabel(filter)}
                  </SelectItem>
                ))}
              </SelectInput>
              <Button
                size="lg"
                disabled={!activeWorkspaceCode}
                onClick={() => navigate("/variables/new")}
              >
                {pageT("actions.new", "New")}
              </Button>
            </div>
          }
          table={table}
          loading={loading}
          tableClassName="table-fixed min-w-[1360px]"
          onRowClick={(row) => navigateToDetail(row)}
          pagination={
            <TablePagination
              itemLabel={pageT("pagination.itemLabel", "variables")}
              table={table}
              totalItems={filteredVariables.length}
            />
          }
          emptyState={
            activeWorkspaceCode
              ? pageT("empty.filtered", "No effective variables match the current filters.")
              : pageT("empty.workspace", "Select a workspace in the shell to load variables.")
          }
        />
      </>

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

import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  MoreHorizontal
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/components/ui/button";
import {
  DataTable,
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextInput,
  useTablePagination,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

import {
  deleteWorkspaceModel,
  disableWorkspaceModel,
  duplicateWorkspaceModel,
  enableWorkspaceModel,
  fetchWorkspaceModelCatalog,
  setWorkspaceDefaultModel,
  verifyWorkspaceModel,
} from "@/domain/model/api";
import {
  ModelSourceBadge,
} from "@/domain/model/components";
import {
  formatModelCapability,
  formatModelProtocol,
  normalizeModelCapabilities,
} from "@/domain/model/constants/modelPageOptions";
import {
  deleteWorkspaceModelRecord,
  duplicateWorkspaceModelRecord,
  getWorkspaceModelCatalog,
  setWorkspaceDefaultRecord,
  setWorkspaceModelEnabled,
  verifyWorkspaceModelRecord,
} from "@/domain/model/mock";
import type {
  ModelRecord,
  WorkspaceModelCatalog,
} from "@/domain/model/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { SelectItem } from "@/components/ui/select";

export function ModelRegistryPage() {
  const navigate = useNavigate();
  const { t, i18n } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const dataSource = getServiceDataSource("model");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const [catalog, setCatalog] = useState<WorkspaceModelCatalog | null>(null);
  const [loading, setLoading] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("ALL");
  const [protocolFilter, setProtocolFilter] = useState("ALL");
  const [typeFilter, setTypeFilter] = useState("ALL");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:modelRegistry.${key}`, { defaultValue, ...options })
  );

  const records = catalog?.records ?? [];

  function getProtocolFilterValue(record: ModelRecord) {
    return record.protocol;
  }

  function getProtocolLabel(protocol: ModelRecord["protocol"], modelType?: ModelRecord["modelType"]) {
    switch (protocol) {
      case "OPENAI_COMPATIBLE":
        return pageT(
          `protocol.openaiCompatible.${modelType === "EMBEDDING" ? "embedding" : "chat"}`,
          modelType === "EMBEDDING" ? "OpenAI Compatible - Embedding" : "OpenAI Compatible - Chat",
        );
      case "RESPONSES":
        return pageT("protocol.responses", "OpenAI Compatible - Response");
      case "RERANK":
        return pageT("protocol.rerank", "OpenAI Compatible - Rerank");
      default:
        return protocol;
    }
  }

  function getProtocolFilterLabel(protocol: ModelRecord["protocol"]) {
    if (protocol === "OPENAI_COMPATIBLE") {
      return pageT("protocol.openaiCompatible.label", "OpenAI Compatible");
    }
    return getProtocolLabel(protocol);
  }

  function getTypeFilterValue(record: ModelRecord) {
    return `${record.modelType}::${record.modelCapabilities.join(",")}`;
  }

  function getTypeFilterLabel(record: ModelRecord) {
    return normalizeModelCapabilities(record.modelCapabilities, record.modelType)
      .map((capability) => pageT(`capabilities.${capability}`, formatModelCapability(capability)))
      .join(", ");
  }

  function getWorkspaceSourceLabel(record: ModelRecord) {
    switch (record.sourceKind) {
      case "LOCAL":
        return pageT("source.local", "Local");
      case "INHERITED":
        return pageT("source.inherited", "Inherited");
      case "OVERRIDE":
        return pageT("source.override", "Override");
      default:
        return pageT("source.platform", "Platform");
    }
  }

  function getWorkspaceInheritedFrom(record: ModelRecord) {
    if (record.sourceKind === "LOCAL") {
      return null;
    }
    if (record.sourceWorkspacePath.length > 0) {
      return {
        code: record.sourceWorkspacePath.map((entry) => entry.workspaceCode).join(" / "),
        label: record.sourceWorkspacePath.map((entry) => entry.name).join(" / "),
      };
    }
    if (record.sourceWorkspaceCode || record.sourceWorkspaceName) {
      return {
        code: record.sourceWorkspaceCode,
        label: record.sourceWorkspaceName?.trim() || record.sourceWorkspaceCode || pageT("source.workspace", "Workspace"),
      };
    }
    return {
      code: null,
      label: pageT("source.platform", "Platform"),
    };
  }

  const protocolOptions = useMemo(() => {
    const seen = new Set<string>();
    return records.flatMap((record) => {
      const value = getProtocolFilterValue(record);
      if (seen.has(value)) return [];
      seen.add(value);
      return [{ value, label: getProtocolFilterLabel(record.protocol) }];
    });
  }, [i18n.language, records]);

  const typeOptions = useMemo(() => {
    const seen = new Set<string>();
    return records.flatMap((record) => {
      const value = getTypeFilterValue(record);
      if (seen.has(value)) return [];
      seen.add(value);
      return [{ value, label: getTypeFilterLabel(record) }];
    });
  }, [i18n.language, records]);

  const filteredRecords = useMemo(() => {
    const normalizedSearch = searchQuery.trim().toLowerCase();
    return records.filter((record) => {
      if (statusFilter === "ENABLED" && !record.enabled) return false;
      if (statusFilter === "DISABLED" && record.enabled) return false;
      if (protocolFilter !== "ALL" && getProtocolFilterValue(record) !== protocolFilter) return false;
      if (typeFilter !== "ALL" && getTypeFilterValue(record) !== typeFilter) return false;
      if (!normalizedSearch) return true;
      const inheritedFrom = getWorkspaceInheritedFrom(record);

      const haystack = [
        record.displayName,
        record.modelName,
        record.vendor,
        getWorkspaceSourceLabel(record),
        inheritedFrom?.label ?? "",
        inheritedFrom?.code ?? "",
        formatModelProtocol(record.protocol, record.modelType),
        getProtocolLabel(record.protocol, record.modelType),
        getTypeFilterLabel(record),
      ].join(" ").toLowerCase();
      return haystack.includes(normalizedSearch);
    });
  }, [i18n.language, protocolFilter, records, searchQuery, statusFilter, typeFilter]);
  const sortedRecords = useMemo(
    () => [...filteredRecords].sort((left, right) => Number(right.isDefault) - Number(left.isDefault)),
    [filteredRecords],
  );
  const pagination = useTablePagination(sortedRecords);
  async function refreshWorkspaceModels() {
    if (!activeWorkspaceCode) {
      setCatalog(null);
      return;
    }

    setLoading(true);
    try {
      const nextCatalog = dataSource === "api"
        ? await fetchWorkspaceModelCatalog(activeWorkspaceCode)
        : getWorkspaceModelCatalog(activeWorkspaceCode);
      setCatalog(nextCatalog);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Workspace models could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshWorkspaceModels();
  }, [activeWorkspaceCode]);

  function hasRowActions(row: ModelRecord) {
    return row.workspaceCode === activeWorkspaceCode;
  }

  async function handleSetDefault(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode || row.isDefault) return;
    setRowBusyCode(row.modelCode);

    try {
      await (dataSource === "api"
        ? setWorkspaceDefaultModel(activeWorkspaceCode, row.modelCode)
        : setWorkspaceDefaultRecord(activeWorkspaceCode, row.modelCode));
      toast.success(pageT("messages.default", "{{name}} set as the {{type}} default", {
        name: row.displayName,
        type: pageT(`types.${row.modelType}`, row.modelType.toLowerCase()),
      }));
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.default", "Default model could not be updated"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleToggleEnabled(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);

    try {
      const updated = dataSource === "api"
        ? (row.enabled
          ? await disableWorkspaceModel(activeWorkspaceCode, row.modelCode)
          : await enableWorkspaceModel(activeWorkspaceCode, row.modelCode))
        : setWorkspaceModelEnabled(activeWorkspaceCode, row.modelCode, !row.enabled);
      toast.success(pageT("messages.toggle", "{{name}} {{status}}", {
        name: updated.displayName,
        status: updated.enabled ? pageT("status.enabledLower", "enabled") : pageT("status.disabledLower", "disabled"),
      }));
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.status", "Status update failed"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleVerify(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    const verifyingToastId = toast.info(pageT("messages.verifying", "Verifying connectivity for {{name}}...", { name: row.displayName }), {
      title: pageT("actions.verify", "Verify"),
      durationMs: 0,
    });

    try {
      const verified = dataSource === "api"
        ? await verifyWorkspaceModel(activeWorkspaceCode, row.modelCode)
        : verifyWorkspaceModelRecord(activeWorkspaceCode, row.modelCode);
      toast.success(pageT("messages.verified", "{{name}} verified", { name: verified.displayName }));
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.verify", "Verify failed"));
    } finally {
      toast.dismiss(verifyingToastId);
      setRowBusyCode("");
    }
  }

  async function handleDuplicate(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);

    try {
      const duplicated = dataSource === "api"
        ? await duplicateWorkspaceModel(activeWorkspaceCode, row.modelCode)
        : duplicateWorkspaceModelRecord(activeWorkspaceCode, row.modelCode);
      toast.success(pageT("messages.created", "{{name}} created", { name: duplicated.displayName }));
      navigate(`/models/detail?modelCode=${encodeURIComponent(duplicated.modelCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.duplicate", "Workspace model could not be duplicated"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleDelete(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);

    try {
      if (dataSource === "api") {
        await deleteWorkspaceModel(activeWorkspaceCode, row.modelCode);
      } else {
        deleteWorkspaceModelRecord(activeWorkspaceCode, row.modelCode);
      }
      toast.success(row.sourceKind === "OVERRIDE"
        ? pageT("messages.reverted", "{{name}} reverted to inherited", { name: row.displayName })
        : pageT("messages.deleted", "{{name}} deleted", { name: row.displayName }));
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.delete", "Workspace model could not be removed"));
    } finally {
      setRowBusyCode("");
    }
  }

  const headerActions = useMemo(() => (
    <>
      <Button size="lg" variant="outline" disabled={loading} onClick={() => void refreshWorkspaceModels()}>
        {loading ? pageT("actions.refreshing", "Refreshing...") : pageT("actions.refresh", "Refresh")}
      </Button>
      <Button size="lg" onClick={() => navigate("/models/new")}>
        {pageT("actions.new", "New")}
      </Button>
    </>
  ), [i18n.language, loading, navigate]);

  return (
    <>
      <PageHeader title={pageT("title", "Models")} actions={headerActions} />

      <div className="space-y-5">
        <SurfaceCard className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <div className="min-w-0">
              <Field label={pageT("filters.search", "Search")}>
                <TextInput
                  className="w-full"
                  value={searchQuery}
                  placeholder={pageT("filters.searchPlaceholder", "Search models")}
                  onChange={(event) => setSearchQuery(event.target.value)}
                />
              </Field>
            </div>
            <div className="min-w-0">
              <Field label={pageT("filters.status", "Status")}>
                <SelectInput className="w-full" value={statusFilter} onValueChange={(event) => setStatusFilter(event!)}>
                  <SelectItem value="ALL">{pageT("filters.all", "All")}</SelectItem>
                  <SelectItem value="ENABLED">{pageT("status.enabled", "Enabled")}</SelectItem>
                  <SelectItem value="DISABLED">{pageT("status.disabled", "Disabled")}</SelectItem>
                </SelectInput>
              </Field>
            </div>
            <div className="min-w-0">
              <Field label={pageT("filters.protocol", "Protocol")}>
                <SelectInput
                  aria-label={pageT("filters.protocol", "Protocol")}
                  className="w-full"
                  value={protocolFilter}
                  onValueChange={(event) => setProtocolFilter(event!)}
                >
                  <SelectItem value="ALL">{pageT("filters.all", "All")}</SelectItem>
                  {protocolOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
            <div className="min-w-0">
              <Field label={pageT("filters.type", "Type")}>
                <SelectInput
                  aria-label={pageT("filters.type", "Type")}
                  className="w-full"
                  value={typeFilter}
                  onValueChange={(event) => setTypeFilter(event!)}
                >
                  <SelectItem value="ALL">{pageT("filters.all", "All")}</SelectItem>
                  {typeOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
          </div>
        </SurfaceCard>

        <DataTable<ModelRecord>
          rows={pagination.visibleItems}
          pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "models")} {...pagination.paginationProps} />}
          rowKey={(row) => row.modelCode}
          onRowClick={(row) => navigate(`/models/detail?modelCode=${encodeURIComponent(row.modelCode)}`)}
          columns={[
            {
              id: "displayName",
              header: pageT("columns.displayName", "Display Name"),
              render: (row) => (
                <div>
                  <p className="font-medium text-[var(--color-on-surface)]">{row.displayName}</p>
                  <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{row.modelName}</p>
                </div>
              ),
            },
            {
              id: "source",
              header: pageT("columns.source", "Source"),
              render: (row) => (
                <ModelSourceBadge
                  sourceKind={row.sourceKind}
                  label={getWorkspaceSourceLabel(row)}
                />
              ),
            },
            {
              id: "inheritedFrom",
              header: pageT("columns.inheritedFrom", "Inherited From"),
              render: (row) => {
                const inheritedFrom = getWorkspaceInheritedFrom(row);
                if (!inheritedFrom) {
                  return (
                    <span className="text-[13px] text-[var(--color-on-surface-variant)]">-</span>
                  );
                }
                return (
                  <div>
                    <p className="font-medium text-[var(--color-on-surface)]">
                      {inheritedFrom.label}
                    </p>
                  </div>
                );
              },
            },
            {
              id: "protocol",
              header: pageT("columns.protocol", "Protocol"),
              render: (row) => getProtocolLabel(row.protocol, row.modelType),
            },
            {
              id: "type",
              header: pageT("columns.type", "Type"),
              render: (row) => getTypeFilterLabel(row),
            },
            {
              id: "status",
              header: pageT("columns.status", "Status"),
              render: (row) => (
                <div className="flex flex-wrap items-center gap-2">
                  <Badge tone={row.enabled ? "success" : "info"}>{row.enabled ? pageT("status.enabled", "Enabled") : pageT("status.disabled", "Disabled")}</Badge>
                  {row.isDefault ? <Badge tone="info">{pageT("status.default", "Default")}</Badge> : null}
                </div>
              ),
            },
            {
              align: "right",
              header: pageT("columns.actions", "Actions"),
              headerClassName: "w-24",
              id: "actions",
              cellClassName: "w-24",
              render: (row) => {
                const hasActionsAvailable = hasRowActions(row);
                const isBusy = rowBusyCode === row.modelCode;

                if (!hasActionsAvailable) {
                  return null;
                }

                return (
                  <div className="flex justify-end">
                    <DropdownMenu>
                      <DropdownMenuTrigger disabled={isBusy}>
                        <MoreHorizontal onClick={(event) => {
                          event.stopPropagation();
                        }} size={16} aria-hidden="true" />
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end" className="w-44">
                        {row.enabled && !row.isDefault ? (
                          <DropdownMenuItem
                            disabled={isBusy}
                            onClick={(event) => {
                              event.stopPropagation();
                              void handleSetDefault(row);
                            }}
                          >
                            {pageT("actions.setDefault", "Set Default")}
                          </DropdownMenuItem>
                        ) : null}
                        <DropdownMenuItem
                          disabled={isBusy}
                          onClick={(event) => {
                            event.stopPropagation();
                            void handleDuplicate(row);
                          }}
                        >
                          {pageT("actions.copy", "Copy")}
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={(event) => {
                            event.stopPropagation();
                            navigate(`/models/detail?modelCode=${encodeURIComponent(row.modelCode)}&mode=edit`);
                          }}
                        >
                          {pageT("actions.edit", "Edit")}
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          disabled={isBusy}
                          onClick={(event) => {
                            event.stopPropagation();
                            void handleToggleEnabled(row);
                          }}
                        >
                          {row.enabled ? pageT("actions.disable", "Disable") : pageT("actions.enable", "Enable")}
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          disabled={isBusy}
                          onClick={(event) => {
                            event.stopPropagation();
                            void handleVerify(row);
                          }}
                        >
                          {pageT("actions.verify", "Verify")}
                        </DropdownMenuItem>
                        {row.sourceKind === "LOCAL" || row.sourceKind === "OVERRIDE" ? (
                          <DropdownMenuItem
                            variant="destructive"
                            disabled={isBusy}
                            onClick={(event) => {
                              event.stopPropagation();
                              void handleDelete(row);
                            }}
                          >
                            {row.sourceKind === "OVERRIDE" ? pageT("actions.revert", "Revert") : pageT("actions.delete", "Delete")}
                          </DropdownMenuItem>
                        ) : null}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                );
              },
            },
          ]}
          emptyState={activeWorkspaceCode ? pageT("empty.filtered", "No effective models match the current filters.") : pageT("empty.workspace", "Select a workspace in the shell to load models.")}
        />
      </div>
    </>
  );
}

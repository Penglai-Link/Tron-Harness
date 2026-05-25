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

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextInput,
  useDismissibleLayer,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

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
  formatModelCapabilities,
  formatModelProtocol,
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

interface RowMenuState {
  left: number;
  modelCode: string;
  top: number;
  triggerElement: HTMLButtonElement;
}

export function ModelRegistryPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("model");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const [catalog, setCatalog] = useState<WorkspaceModelCatalog | null>(null);
  const [loading, setLoading] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("ALL");
  const [protocolFilter, setProtocolFilter] = useState("ALL");
  const [typeFilter, setTypeFilter] = useState("ALL");
  const rowMenuRef = useRef<HTMLDivElement | null>(null);

  const records = catalog?.records ?? [];
  const openMenuRow = openRowMenu
    ? records.find((record) => record.modelCode === openRowMenu.modelCode) ?? null
    : null;

  function getProtocolFilterValue(record: ModelRecord) {
    return record.protocol;
  }

  function getProtocolFilterLabel(protocol: ModelRecord["protocol"]) {
    switch (protocol) {
      case "OPENAI_COMPATIBLE":
        return "OpenAI Compatible";
      case "RESPONSES":
        return "OpenAI Compatible - Response";
      case "RERANK":
        return "OpenAI Compatible - Rerank";
      default:
        return protocol;
    }
  }

  function getTypeFilterValue(record: ModelRecord) {
    return `${record.modelType}::${record.modelCapabilities.join(",")}`;
  }

  function getTypeFilterLabel(record: ModelRecord) {
    return formatModelCapabilities(record.modelCapabilities, record.modelType);
  }

  function getWorkspaceSourceLabel(record: ModelRecord) {
    switch (record.sourceKind) {
      case "LOCAL":
        return "Local";
      case "INHERITED":
        return "Inherited";
      case "OVERRIDE":
        return "Override";
      default:
        return "Platform";
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
        label: record.sourceWorkspaceName?.trim() || record.sourceWorkspaceCode || "Workspace",
      };
    }
    return {
      code: null,
      label: "Platform",
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
  }, [records]);

  const typeOptions = useMemo(() => {
    const seen = new Set<string>();
    return records.flatMap((record) => {
      const value = getTypeFilterValue(record);
      if (seen.has(value)) return [];
      seen.add(value);
      return [{ value, label: getTypeFilterLabel(record) }];
    });
  }, [records]);

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
        getProtocolFilterLabel(record.protocol),
        getTypeFilterLabel(record),
      ].join(" ").toLowerCase();
      return haystack.includes(normalizedSearch);
    });
  }, [protocolFilter, records, searchQuery, statusFilter, typeFilter]);

  async function refreshWorkspaceModels() {
    if (!activeWorkspaceCode) {
      setCatalog(null);
      setOpenRowMenu(null);
      return;
    }

    setLoading(true);
    try {
      const nextCatalog = dataSource === "api"
        ? await fetchWorkspaceModelCatalog(activeWorkspaceCode)
        : getWorkspaceModelCatalog(activeWorkspaceCode);
      setCatalog(nextCatalog);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Workspace models could not be loaded");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshWorkspaceModels();
  }, [activeWorkspaceCode]);

  useDismissibleLayer({
    onDismiss: () => setOpenRowMenu(null),
    open: openRowMenu !== null,
    popupRef: rowMenuRef,
    triggerElement: openRowMenu?.triggerElement,
  });

  function hasRowActions(row: ModelRecord) {
    return row.workspaceCode === activeWorkspaceCode;
  }

  function toggleRowMenu(row: ModelRecord, trigger: HTMLButtonElement) {
    if (!activeWorkspaceCode || !hasRowActions(row) || rowBusyCode === row.modelCode) {
      return;
    }
    if (openRowMenu?.modelCode === row.modelCode) {
      setOpenRowMenu(null);
      return;
    }

    const menuWidth = 176;
    const estimatedMenuHeight = 272;
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
      modelCode: row.modelCode,
      left,
      top,
      triggerElement: trigger,
    });
  }

  async function handleSetDefault(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode || row.isDefault) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);

    try {
      await (dataSource === "api"
        ? setWorkspaceDefaultModel(activeWorkspaceCode, row.modelCode)
        : setWorkspaceDefaultRecord(activeWorkspaceCode, row.modelCode));
      toast.success(`${row.displayName} set as the ${row.modelType.toLowerCase()} default`);
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Default model could not be updated");
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleToggleEnabled(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);

    try {
      const updated = dataSource === "api"
        ? (row.enabled
          ? await disableWorkspaceModel(activeWorkspaceCode, row.modelCode)
          : await enableWorkspaceModel(activeWorkspaceCode, row.modelCode))
        : setWorkspaceModelEnabled(activeWorkspaceCode, row.modelCode, !row.enabled);
      toast.success(`${updated.displayName} ${updated.enabled ? "enabled" : "disabled"}`);
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Status update failed");
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleVerify(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);
    const verifyingToastId = toast.info(`Verifying connectivity for ${row.displayName}...`, {
      title: "Verify",
      durationMs: 0,
    });

    try {
      const verified = dataSource === "api"
        ? await verifyWorkspaceModel(activeWorkspaceCode, row.modelCode)
        : verifyWorkspaceModelRecord(activeWorkspaceCode, row.modelCode);
      toast.success(`${verified.displayName} verified`);
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Verify failed");
    } finally {
      toast.dismiss(verifyingToastId);
      setRowBusyCode("");
    }
  }

  async function handleDuplicate(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);

    try {
      const duplicated = dataSource === "api"
        ? await duplicateWorkspaceModel(activeWorkspaceCode, row.modelCode)
        : duplicateWorkspaceModelRecord(activeWorkspaceCode, row.modelCode);
      toast.success(`${duplicated.displayName} created`);
      navigate(`/models/detail?modelCode=${encodeURIComponent(duplicated.modelCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Workspace model could not be duplicated");
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleDelete(row: ModelRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);

    try {
      if (dataSource === "api") {
        await deleteWorkspaceModel(activeWorkspaceCode, row.modelCode);
      } else {
        deleteWorkspaceModelRecord(activeWorkspaceCode, row.modelCode);
      }
      toast.success(
        row.sourceKind === "OVERRIDE"
          ? `${row.displayName} reverted to inherited`
          : `${row.displayName} deleted`,
      );
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Workspace model could not be removed");
    } finally {
      setRowBusyCode("");
    }
  }

  const headerActions = useMemo(() => (
    <>
      <Button variant="secondary" disabled={loading} onClick={() => void refreshWorkspaceModels()}>
        <RefreshCcw size={16} aria-hidden="true" />
        {loading ? "Refreshing..." : "Refresh"}
      </Button>
      <Button onClick={() => navigate("/models/new")}>
        <Plus size={16} aria-hidden="true" />
        New
      </Button>
    </>
  ), [loading, navigate]);

  return (
    <>
      <PageHeader title="Models" actions={headerActions} />

      <div className="space-y-5">
        <SurfaceCard className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <div className="min-w-0">
              <Field label="Search">
                <TextInput
                  className="w-full"
                  value={searchQuery}
                  placeholder="Search models"
                  onChange={(event) => setSearchQuery(event.target.value)}
                />
              </Field>
            </div>
            <div className="min-w-0">
              <Field label="Status">
                <SelectInput className="w-full" value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>
                  <option value="ALL">All</option>
                  <option value="ENABLED">Enabled</option>
                  <option value="DISABLED">Disabled</option>
                </SelectInput>
              </Field>
            </div>
            <div className="min-w-0">
              <Field label="Protocol">
                <SelectInput
                  aria-label="Protocol"
                  className="w-full"
                  value={protocolFilter}
                  onChange={(event) => setProtocolFilter(event.target.value)}
                >
                  <option value="ALL">All</option>
                  {protocolOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </SelectInput>
              </Field>
            </div>
            <div className="min-w-0">
              <Field label="Type">
                <SelectInput
                  aria-label="Type"
                  className="w-full"
                  value={typeFilter}
                  onChange={(event) => setTypeFilter(event.target.value)}
                >
                  <option value="ALL">All</option>
                  {typeOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </SelectInput>
              </Field>
            </div>
          </div>
        </SurfaceCard>

        <DataTable<ModelRecord>
          rows={filteredRecords}
          rowKey={(row) => row.modelCode}
          onRowClick={(row) => navigate(`/models/detail?modelCode=${encodeURIComponent(row.modelCode)}`)}
          columns={[
            {
              id: "displayName",
              header: "Display Name",
              render: (row) => (
                <div>
                  <p className="font-medium text-[var(--color-on-surface)]">{row.displayName}</p>
                  <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{row.modelName}</p>
                </div>
              ),
            },
            {
              id: "source",
              header: "Source",
              render: (row) => (
                <ModelSourceBadge
                  sourceKind={row.sourceKind}
                  label={row.sourceLabel || getWorkspaceSourceLabel(row)}
                />
              ),
            },
            {
              id: "inheritedFrom",
              header: "Inherited From",
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
              header: "Protocol",
              render: (row) => formatModelProtocol(row.protocol, row.modelType),
            },
            {
              id: "type",
              header: "Type",
              render: (row) => formatModelCapabilities(row.modelCapabilities, row.modelType),
            },
            {
              id: "status",
              header: "Status",
              render: (row) => (
                <div className="flex flex-wrap items-center gap-2">
                  <Badge tone={row.enabled ? "success" : "neutral"}>{row.enabled ? "Enabled" : "Disabled"}</Badge>
                  {row.isDefault ? <Badge tone="accent">Default</Badge> : null}
                </div>
              ),
            },
            {
              align: "right",
              header: "Actions",
              headerClassName: "w-24",
              id: "actions",
              cellClassName: "w-24",
              render: (row) => {
                const hasActionsAvailable = hasRowActions(row);
                const isMenuOpen = openRowMenu?.modelCode === row.modelCode;
                const isBusy = rowBusyCode === row.modelCode;

                if (!hasActionsAvailable) {
                  return null;
                }

                return (
                  <div className="flex justify-end">
                    <Button
                      size="icon"
                      variant="ghost"
                      aria-label={`Open actions for ${row.displayName}`}
                      aria-expanded={isMenuOpen}
                      data-workspace-model-menu-trigger={row.modelCode}
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
          emptyState={activeWorkspaceCode ? "No effective models match the current filters." : "Select a workspace in the shell to load models."}
        />
      </div>

      {openRowMenu && openMenuRow
        ? createPortal(
          <div
            ref={rowMenuRef}
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
            className="fixed z-[80] w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={`Actions for ${openMenuRow.displayName}`}
          >
            {openMenuRow.enabled && !openMenuRow.isDefault ? (
              <button
                type="button"
                role="menuitem"
                className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                disabled={rowBusyCode === openMenuRow.modelCode}
                onClick={() => {
                  void handleSetDefault(openMenuRow);
                }}
              >
                Set Default
              </button>
            ) : null}
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuRow.modelCode}
              onClick={() => {
                void handleDuplicate(openMenuRow);
              }}
            >
              Copy
            </button>
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              onClick={() => {
                setOpenRowMenu(null);
                navigate(`/models/detail?modelCode=${encodeURIComponent(openMenuRow.modelCode)}`);
              }}
            >
              Edit
            </button>
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuRow.modelCode}
              onClick={() => {
                void handleToggleEnabled(openMenuRow);
              }}
            >
              {openMenuRow.enabled ? "Disable" : "Enable"}
            </button>
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuRow.modelCode}
              onClick={() => {
                void handleVerify(openMenuRow);
              }}
            >
              Verify
            </button>
            {openMenuRow.sourceKind === "LOCAL" || openMenuRow.sourceKind === "OVERRIDE" ? (
              <button
                type="button"
                role="menuitem"
                className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                disabled={rowBusyCode === openMenuRow.modelCode}
                onClick={() => {
                  void handleDelete(openMenuRow);
                }}
              >
                {openMenuRow.sourceKind === "OVERRIDE" ? "Revert" : "Delete"}
              </button>
            ) : null}
          </div>,
          document.body,
        )
        : null}
    </>
  );
}

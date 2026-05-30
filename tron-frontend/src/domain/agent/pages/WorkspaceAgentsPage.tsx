import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import {
  disableWorkspaceSystemAgent,
  enableWorkspaceSystemAgent,
  fetchWorkspaceSystemAgents,
} from "@/domain/agent/api";
import {
  AgentPolicySwitch,
  AgentStatusBadge,
} from "@/domain/agent/components";
import {
  listWorkspaceSystemAgentsMock,
  setWorkspaceSystemAgentEnabledMock,
} from "@/domain/agent/mock";
import type { WorkspaceSystemAgentRecord } from "@/domain/agent/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  DataTable,
  PageHeader,
  SearchField,
  SurfaceCard,
  TablePagination,
  useTablePagination,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

function resolveWorkspacePolicyLabel(
  row: WorkspaceSystemAgentRecord,
  label: (key: string, defaultValue: string) => string,
) {
  if (!row.available) {
    return label("table.policyUnavailable", "Unavailable");
  }
  if (!row.platformEnabled) {
    return label("table.policyPlatformDisabled", "Platform disabled");
  }
  if (row.workspaceOverrideDisabled) {
    return label("table.policyLocalDisable", "Local disable");
  }
  return label("table.policyInherited", "Platform baseline");
}

function canToggleWorkspaceAgent(row: WorkspaceSystemAgentRecord) {
  if (!row.available) {
    return false;
  }

  return row.workspaceOverrideDisabled || row.platformEnabled;
}

export function WorkspaceAgentsPage() {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const dataSource = getServiceDataSource("agent");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode =
    selectedSecondarySpace?.workspaceCode
    ?? selectedPrimarySpace?.workspaceCode
    ?? "";
  const activeWorkspaceLabel =
    selectedSecondarySpace?.label
    ?? selectedPrimarySpace?.name
    ?? t("pages:workspaceAgents.workspace.unselected", { defaultValue: "No workspace selected" });
  const [records, setRecords] = useState<WorkspaceSystemAgentRecord[]>([]);
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:workspaceAgents.${key}`, { defaultValue, ...options })
  );

  const filteredRecords = useMemo(() => {
    const normalizedSearch = debouncedSearch.trim().toLowerCase();
    if (!normalizedSearch) {
      return records;
    }

    return records.filter((record) => (
      [
        record.name,
        record.agentCode,
        record.description,
        record.inheritedFrom,
        record.status,
      ].join(" ").toLowerCase().includes(normalizedSearch)
    ));
  }, [debouncedSearch, records]);
  const pagination = useTablePagination(filteredRecords);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(searchInput.trim());
    }, 240);

    return () => window.clearTimeout(timer);
  }, [searchInput]);

  async function loadWorkspaceAgents() {
    if (!activeWorkspaceCode) {
      setRecords([]);
      return;
    }

    setLoading(true);
    try {
      const nextRecords = dataSource === "api"
        ? await fetchWorkspaceSystemAgents(activeWorkspaceCode)
        : listWorkspaceSystemAgentsMock(activeWorkspaceCode);
      setRecords(nextRecords);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Workspace agents could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadWorkspaceAgents();
  }, [activeWorkspaceCode, dataSource]);

  async function handleToggleAgent(row: WorkspaceSystemAgentRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.agentCode || !canToggleWorkspaceAgent(row)) {
      return;
    }

    setRowBusyCode(row.agentCode);
    try {
      const updated = dataSource === "api"
        ? (row.enabled
          ? await disableWorkspaceSystemAgent(activeWorkspaceCode, row.agentCode)
          : await enableWorkspaceSystemAgent(activeWorkspaceCode, row.agentCode))
        : setWorkspaceSystemAgentEnabledMock(activeWorkspaceCode, row.agentCode, !row.enabled);
      await loadWorkspaceAgents();

      if (row.enabled) {
        toast.success(pageT("messages.disabled", "{{name}} disabled in {{workspace}}", {
          name: updated.name,
          workspace: activeWorkspaceLabel,
        }));
      } else if (updated.enabled) {
        toast.success(pageT("messages.enabled", "{{name}} now follows platform policy", {
          name: updated.name,
        }));
      } else {
        toast.success(pageT("messages.stillDisabled", "{{name}} follows platform policy and remains disabled", {
          name: updated.name,
        }));
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.toggle", "Workspace agent policy update failed"));
    } finally {
      setRowBusyCode("");
    }
  }

  if (!activeWorkspaceCode) {
    return (
      <SurfaceCard>
        <h1 className="text-2xl font-semibold text-[var(--color-on-surface)]">
          {pageT("title", "Agents")}
        </h1>
        <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
          {pageT("emptyWorkspace", "Select a workspace to manage inherited system-agent policy.")}
        </p>
      </SurfaceCard>
    );
  }

  return (
    <div className="space-y-6">
      <PageHeader title={pageT("title", "Agents")} />

      <DataTable<WorkspaceSystemAgentRecord>
        title={pageT("table.title", "Inherited System Agents")}
        rows={pagination.visibleItems}
        rowKey={(row) => `${activeWorkspaceCode}-${row.agentCode}`}
        pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "workspace agents")} {...pagination.paginationProps} />}
        toolbar={(
          <SearchField
            ariaLabel={pageT("table.searchAria", "Search workspace agents")}
            className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
            placeholder={pageT("table.searchPlaceholder", "Search workspace agents...")}
            value={searchInput}
            onChange={setSearchInput}
          />
        )}
        emptyState={pageT("table.empty", "No workspace agents match the current filters.")}
        columns={[
          {
            id: "agent",
            header: pageT("table.columns.agent", "Agent"),
            width: "22%",
            render: (row) => (
              <div>
                <p className="font-medium text-[var(--color-on-surface)]">{row.name}</p>
                <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{row.agentCode}</p>
              </div>
            ),
          },
          {
            id: "description",
            header: pageT("table.columns.description", "Description"),
            width: "24%",
            render: (row) => (
              <p className="text-[13px] text-[var(--color-on-surface-variant)]">{row.description}</p>
            ),
          },
          {
            id: "source",
            header: pageT("table.columns.source", "Source"),
            width: "12%",
            render: (row) => row.source,
          },
          {
            id: "inheritedFrom",
            header: pageT("table.columns.inheritedFrom", "Inherited From"),
            width: "14%",
            render: (row) => row.inheritedFrom,
          },
          {
            id: "status",
            header: pageT("table.columns.status", "Status"),
            width: "12%",
            render: (row) => <AgentStatusBadge status={row.status} />,
          },
          {
            id: "policy",
            header: pageT("table.columns.policy", "Policy"),
            width: "20%",
            render: (row) => resolveWorkspacePolicyLabel(row, (key, defaultValue) => pageT(key, defaultValue)),
          },
          {
            align: "right",
            id: "actions",
            header: pageT("table.columns.actions", "Actions"),
            width: "14%",
            render: (row) => (
              <div className="flex justify-end">
                <AgentPolicySwitch
                  checked={row.enabled}
                  disabled={!canToggleWorkspaceAgent(row) || rowBusyCode === row.agentCode || loading}
                  label={!row.available
                    ? pageT("table.unavailableLabel", "{{name}} unavailable", { name: row.name })
                    : !row.platformEnabled && !row.workspaceOverrideDisabled
                      ? pageT("table.platformDisabledLabel", "{{name}} disabled by platform policy", { name: row.name })
                      : row.workspaceOverrideDisabled
                        ? pageT("table.enableLabel", "Enable {{name}}", { name: row.name })
                        : pageT("table.disableLabel", "Disable {{name}}", { name: row.name })}
                  loading={rowBusyCode === row.agentCode}
                  onToggle={() => void handleToggleAgent(row)}
                />
              </div>
            ),
          },
        ]}
      />
    </div>
  );
}

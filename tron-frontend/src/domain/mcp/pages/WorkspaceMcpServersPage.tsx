import {
  useEffect,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import { AgentPolicySwitch } from "@/domain/agent/components";
import {
  disableWorkspaceMcpServer,
  enableWorkspaceMcpServer,
  fetchWorkspaceMcpServers,
} from "@/domain/mcp/api";
import {
  listWorkspaceMcpServersMock,
  setWorkspaceMcpServerEnabledMock,
} from "@/domain/mcp/mock";
import type { WorkspaceMcpServerRecord } from "@/domain/mcp/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Badge,
  DataTable,
  PageHeader,
  SearchField,
  SurfaceCard,
  TablePagination,
  useLocaleFormatters,
  useTablePagination,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

function healthTone(status: WorkspaceMcpServerRecord["lastHealthStatus"]) {
  switch (status) {
    case "HEALTHY":
      return "success" as const;
    case "UNHEALTHY":
      return "warning" as const;
    default:
      return "info" as const;
  }
}

export function WorkspaceMcpServersPage() {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const dataSource = getServiceDataSource("mcp");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = selectedSecondarySpace?.workspaceCode ?? selectedPrimarySpace?.workspaceCode ?? "";
  const activeWorkspaceLabel = selectedSecondarySpace?.label ?? selectedPrimarySpace?.name ?? t("pages:workspaceMcpServers.workspace.unselected", { defaultValue: "No workspace selected" });
  const [records, setRecords] = useState<WorkspaceMcpServerRecord[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");

  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:workspaceMcpServers.${key}`, { defaultValue, ...options })
  );

  const pagination = useTablePagination(records);

  async function refreshRecords() {
    if (!activeWorkspaceCode) {
      setRecords([]);
      return;
    }

    setLoading(true);
    try {
      const nextRecords = dataSource === "api"
        ? await fetchWorkspaceMcpServers(activeWorkspaceCode, search)
        : listWorkspaceMcpServersMock(activeWorkspaceCode, search);
      setRecords(nextRecords);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Workspace MCP servers could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshRecords();
  }, [activeWorkspaceCode, dataSource, search]);

  function canToggle(row: WorkspaceMcpServerRecord) {
    return row.workspaceOverrideDisabled || row.platformEnabled;
  }

  function resolvePolicyLabel(row: WorkspaceMcpServerRecord) {
    if (!row.platformEnabled) {
      return pageT("table.policyPlatformDisabled", "Platform disabled");
    }
    if (row.workspaceOverrideDisabled) {
      return pageT("table.policyLocalDisable", "Local disable");
    }
    return pageT("table.policyInherited", "Platform baseline");
  }

  async function handleToggle(row: WorkspaceMcpServerRecord) {
    if (!activeWorkspaceCode || rowBusyCode === row.serverCode || !canToggle(row)) {
      return;
    }

    setRowBusyCode(row.serverCode);
    try {
      const updated = dataSource === "api"
        ? (row.effectiveEnabled
          ? await disableWorkspaceMcpServer(activeWorkspaceCode, row.serverCode)
          : await enableWorkspaceMcpServer(activeWorkspaceCode, row.serverCode))
        : setWorkspaceMcpServerEnabledMock(activeWorkspaceCode, row.serverCode, !row.effectiveEnabled);
      await refreshRecords();
      if (row.effectiveEnabled) {
        toast.success(pageT("messages.disabled", "{{name}} disabled in {{workspace}}", {
          name: updated.name,
          workspace: activeWorkspaceLabel,
        }));
      } else if (updated.effectiveEnabled) {
        toast.success(pageT("messages.enabled", "{{name}} now follows platform policy", { name: updated.name }));
      } else {
        toast.success(pageT("messages.stillDisabled", "{{name}} follows platform policy and remains disabled", {
          name: updated.name,
        }));
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.toggle", "Workspace MCP policy update failed"));
    } finally {
      setRowBusyCode("");
    }
  }

  if (!activeWorkspaceCode) {
    return (
      <SurfaceCard>
        <h1 className="text-2xl font-semibold text-[var(--color-on-surface)]">
          {pageT("title", "MCP Servers")}
        </h1>
        <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
          {pageT("emptyWorkspace", "Select a workspace to manage inherited MCP server policy.")}
        </p>
      </SurfaceCard>
    );
  }

  return (
    <div className="space-y-5">
      <PageHeader title={pageT("title", "MCP Servers")} />

      <SurfaceCard className="space-y-2 p-4">
        <p className="text-xs font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          {pageT("workspace.label", "Current workspace")}
        </p>
        <p className="text-sm text-[var(--color-on-surface)]">{activeWorkspaceLabel}</p>
        <p className="text-sm text-[var(--color-on-surface-variant)]">
          {pageT("workspace.description", "Inherited MCP servers follow platform policy unless this workspace adds a local disable override.")}
        </p>
      </SurfaceCard>

      <DataTable<WorkspaceMcpServerRecord>
        title={pageT("table.title", "Effective MCP servers")}
        rows={pagination.visibleItems}
        rowKey={(row) => `${activeWorkspaceCode}-${row.serverCode}`}
        pagination={<TablePagination itemLabel={pageT("table.itemLabel", "MCP servers")} {...pagination.paginationProps} />}
        toolbar={(
          <SearchField
            ariaLabel={pageT("table.searchAria", "Search workspace MCP servers")}
            className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
            placeholder={pageT("table.searchPlaceholder", "Search workspace MCP servers...")}
            value={search}
            onChange={setSearch}
          />
        )}
        emptyState={pageT("table.empty", "No workspace MCP servers match the current filters.")}
        columns={[
          {
            id: "server",
            header: pageT("table.columns.server", "Server"),
            width: "24%",
            render: (row) => (
              <div>
                <p className="font-medium text-[var(--color-on-surface)]">{row.name}</p>
                <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{row.serverCode}</p>
              </div>
            ),
          },
          {
            id: "description",
            header: pageT("table.columns.description", "Description"),
            width: "26%",
            render: (row) => (
              <p className="text-[13px] text-[var(--color-on-surface-variant)]">
                {row.description || pageT("table.noDescription", "No description available.")}
              </p>
            ),
          },
          {
            id: "source",
            header: pageT("table.columns.source", "Source"),
            width: "10%",
            render: () => pageT("table.sourcePlatform", "Platform"),
          },
          {
            id: "health",
            header: pageT("table.columns.health", "Health"),
            width: "16%",
            render: (row) => (
              <div className="space-y-1">
                <Badge tone={healthTone(row.lastHealthStatus)}>
                  {row.lastHealthStatus}
                </Badge>
                {row.lastHealthCheckedAt ? (
                  <p className="text-[11px] text-[var(--color-on-surface-variant)]">
                    {formatDateTime(row.lastHealthCheckedAt, {
                      dateStyle: "medium",
                      timeStyle: "short",
                    })}
                  </p>
                ) : null}
              </div>
            ),
          },
          {
            id: "policy",
            header: pageT("table.columns.policy", "Policy"),
            width: "16%",
            render: (row) => resolvePolicyLabel(row),
          },
          {
            id: "actions",
            header: pageT("table.columns.actions", "Actions"),
            align: "right",
            width: "8%",
            render: (row) => (
              <div className="flex justify-end">
                <AgentPolicySwitch
                  checked={row.effectiveEnabled}
                  disabled={!canToggle(row) || rowBusyCode === row.serverCode || loading}
                  label={row.workspaceOverrideDisabled
                    ? pageT("table.enableLabel", "Enable {{name}}", { name: row.name })
                    : row.platformEnabled
                      ? pageT("table.disableLabel", "Disable {{name}}", { name: row.name })
                      : pageT("table.platformDisabledLabel", "{{name}} disabled by platform policy", { name: row.name })}
                  loading={rowBusyCode === row.serverCode}
                  onToggle={() => void handleToggle(row)}
                />
              </div>
            ),
          },
        ]}
      />
    </div>
  );
}

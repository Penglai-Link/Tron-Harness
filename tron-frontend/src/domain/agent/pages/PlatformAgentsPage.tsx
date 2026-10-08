import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  RefreshCcw,
} from "lucide-react";

import {
  disablePlatformSystemAgent,
  enablePlatformSystemAgent,
  fetchPlatformSystemAgents,
  fetchWorkspaceInternalAgents,
  rescanPlatformSystemAgents,
} from "@/domain/agent/api";
import {
  AgentPolicySwitch,
  AgentStatusBadge,
} from "@/domain/agent/components";
import {
  listPlatformSystemAgentsMock,
  rescanPlatformSystemAgentsMock,
  setPlatformSystemAgentEnabledMock,
} from "@/domain/agent/mock";
import type {
  PlatformSystemAgentRecord,
} from "@/domain/agent/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { DataTable } from "@/components/data-table";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import {
  PageHeader,
  useLocaleFormatters,
  useTablePagination,
} from "@/shared";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";
import { AgentWorkspaceAssignment } from "../components/AgentWorkspaceAssignment";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { INTERNAL_AGENT_REGISTRY_CHANGED_EVENT, PLATFORM_INTERNAL_AGENT_SCOPE } from "@/domain/agent/model";
import { listWorkspaceInternalAgentsMock } from "@/domain/agent/mock";
import { useAuthStore } from "@/store/authStore";
import { WorkspaceAgentsPage } from "./WorkspaceAgentsPage";
import { LoopDesigner } from "../components/LoopDesigner";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";

function formatLastScanned(lastScannedAt: string | null, formatDateTime: (value: string | number | Date) => string) {
  if (!lastScannedAt) {
    return "Not scanned";
  }

  return formatDateTime(lastScannedAt);
}

export function PlatformAgentsPage() {
  const { t } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const queryClient = useQueryClient();
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const dataSource = getServiceDataSource("agent");
  const internalAgents = useQuery({
    queryKey: ["platform-agent-definitions", user, dataSource],
    queryFn: ({ signal }) => dataSource === "api"
      ? fetchWorkspaceInternalAgents(PLATFORM_INTERNAL_AGENT_SCOPE, signal)
      : listWorkspaceInternalAgentsMock(PLATFORM_INTERNAL_AGENT_SCOPE),
  });
  useEffect(() => {
    const refresh = () => {
      void queryClient.invalidateQueries({ queryKey: ["platform-agent-definitions"] });
      void queryClient.invalidateQueries({ queryKey: ["knowledge-base-agents"] });
    };
    window.addEventListener(INTERNAL_AGENT_REGISTRY_CHANGED_EVENT, refresh);
    return () => window.removeEventListener(INTERNAL_AGENT_REGISTRY_CHANGED_EVENT, refresh);
  }, [queryClient]);
  const [systemAgents, setSystemAgents] = useState<PlatformSystemAgentRecord[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [scanning, setScanning] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:settingsAgents.${key}`, { defaultValue, ...options })
  );

  const filteredAgents = useMemo(() => {
    const normalizedSearch = searchQuery.trim().toLowerCase();
    if (!normalizedSearch) {
      return systemAgents;
    }

    return systemAgents.filter((record) => (
      [
        record.name,
        record.agentCode,
        record.description,
        record.source,
        record.status,
      ].join(" ").toLowerCase().includes(normalizedSearch)
    ));
  }, [searchQuery, systemAgents]);
  const pagination = useTablePagination(filteredAgents);
  async function loadPage() {
    try {
      await refreshSystemAgents();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Agent management data could not be loaded"));
    }
  }

  async function refreshSystemAgents() {
    const records = dataSource === "api"
      ? await fetchPlatformSystemAgents()
      : listPlatformSystemAgentsMock();
    setSystemAgents(records);
  }

  useEffect(() => {
    void loadPage();
  }, [dataSource]);

  async function handleScan() {
    if (scanning) {
      return;
    }

    setScanning(true);
    try {
      const result = dataSource === "api"
        ? await rescanPlatformSystemAgents()
        : rescanPlatformSystemAgentsMock();
      setSystemAgents(result.records);
      toast.success(pageT(
        result.availableCount === 0 ? "messages.noneFound" : "messages.found",
        result.availableCount === 0
          ? "Scan complete. No available system agents found."
          : "Scan complete. Found {{count}} available system agents.",
        { count: formatNumber(result.availableCount) },
      ));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.scan", "System-agent scan failed"));
    } finally {
      setScanning(false);
    }
  }

  async function handleToggleAgent(record: PlatformSystemAgentRecord) {
    if (rowBusyCode === record.agentCode || !record.available) {
      return;
    }

    setRowBusyCode(record.agentCode);
    try {
      const updated = dataSource === "api"
        ? (record.enabled
          ? await disablePlatformSystemAgent(record.agentCode)
          : await enablePlatformSystemAgent(record.agentCode))
        : setPlatformSystemAgentEnabledMock(record.agentCode, !record.enabled);
      await refreshSystemAgents();
      toast.success(pageT(
        updated.enabled ? "messages.enabled" : "messages.disabled",
        updated.enabled ? "{{name}} enabled" : "{{name}} disabled",
        { name: updated.name },
      ));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.toggle", "System-agent policy update failed"));
    } finally {
      setRowBusyCode("");
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader title={pageT("title", "Agent Management")} />

      <Tabs defaultValue="agents">
      <TabsList><TabsTrigger value="agents">{t("pages:settingsAgents.title")}</TabsTrigger><TabsTrigger value="loops">{t("pages:agentLoops.title")}</TabsTrigger></TabsList>
      <TabsContent keepMounted value="loops" className="mt-5"><LoopDesigner /></TabsContent>
      <TabsContent keepMounted value="agents" className="mt-5 space-y-6">
      <section aria-label={t("pages:platformData.agentDefinitions")} className="space-y-6">
      <h2 className="text-base font-semibold">{t("pages:platformData.agentDefinitions")}</h2>
      <p className="text-sm text-[var(--color-on-surface-variant)]">{t("pages:platformData.agentDefinitionHelp")}</p>
      <WorkspaceAgentsPage category="internal" scope="platform" />

      <DataTable<PlatformSystemAgentRecord>
        title={pageT("table.title", "System Agents")}
        rows={pagination.visibleItems}
        rowKey={(row) => row.agentCode}
        pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "system agents")} {...pagination.paginationProps} />}
        toolbar={(
          <>
            <SearchField
              ariaLabel={pageT("table.searchAria", "Search system agents")}
              className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
              placeholder={pageT("table.searchPlaceholder", "Search system agents...")}
              value={searchQuery}
              onChange={setSearchQuery}
            />
            <Button
              variant="secondary"
              disabled={scanning}
              aria-busy={scanning}
              onClick={() => void handleScan()}
            >
              <RefreshCcw size={16} aria-hidden="true" />
              {scanning ? pageT("actions.scanning", "Scanning...") : pageT("actions.scan", "Scan")}
            </Button>
          </>
        )}
        emptyState={pageT("table.empty", "No system agents match the current filters.")}
        columns={[
          {
            id: "agent",
            header: pageT("table.columns.agent", "Agent"),
            width: "32%",
            render: (row) => (
              <div>
                <p className="font-medium text-[var(--color-on-surface)]">{row.name}</p>
                <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{row.agentCode}</p>
                <p className="mt-2 text-[13px] text-[var(--color-on-surface-variant)]">{row.description}</p>
              </div>
            ),
          },
          {
            id: "source",
            header: pageT("table.columns.source", "Source"),
            width: "18%",
            render: (row) => row.source,
          },
          {
            id: "status",
            header: pageT("table.columns.status", "Status"),
            width: "16%",
            render: (row) => <AgentStatusBadge status={row.status} />,
          },
          {
            id: "lastScannedAt",
            header: pageT("table.columns.lastScanned", "Last Scanned"),
            width: "20%",
            render: (row) => formatLastScanned(row.lastScannedAt, formatDateTime),
          },
          {
            align: "right",
            id: "actions",
            header: pageT("table.columns.actions", "Actions"),
            width: "14%",
            render: (row) => (
              <div className="flex justify-end gap-3">
                <AgentPolicySwitch
                  checked={row.enabled}
                  disabled={!row.available || rowBusyCode === row.agentCode}
                  label={row.available
                    ? (row.enabled
                      ? pageT("table.disableLabel", "Disable {{name}}", { name: row.name })
                      : pageT("table.enableLabel", "Enable {{name}}", { name: row.name }))
                    : pageT("table.unavailableLabel", "{{name}} unavailable", { name: row.name })}
                  loading={rowBusyCode === row.agentCode}
                  onToggle={() => void handleToggleAgent(row)}
                />
              </div>
            ),
          },
        ]}
      />
      </section>

      <section aria-label={t("pages:platformData.agentAuthorization")} className="space-y-3">
        <h2 className="text-base font-semibold">{t("pages:platformData.agentAuthorization")}</h2>
        <p className="text-sm text-[var(--color-on-surface-variant)]">{t("pages:platformData.agentAuthorizationHelp")}</p>
        {internalAgents.error ? <p role="alert" className="text-[var(--color-error)]">{internalAgents.error.message}</p> : null}
        {internalAgents.isPending ? <p role="status">{t("common:loading")}</p> : null}
        <DataTable
          pagination={null}
          rows={[...(internalAgents.data ?? []), ...systemAgents]}
          rowKey={(row) => row.agentCode}
          emptyState={t("pages:platformData.noAgentDefinitions")}
          columns={[
            { id: "agent", header: pageT("table.columns.agent", "Agent"), render: (row) => <div><p className="font-medium">{row.name}</p><p className="mt-1 font-mono text-xs text-[var(--color-on-surface-variant)]">{row.agentCode}</p></div> },
            { id: "authorization", header: t("pages:platformData.agentAuthorization"), align: "right", render: (row) => <AgentWorkspaceAssignment code={row.agentCode} name={row.name} /> },
          ]}
        />
      </section>
      </TabsContent>
      </Tabs>
    </div>
  );
}

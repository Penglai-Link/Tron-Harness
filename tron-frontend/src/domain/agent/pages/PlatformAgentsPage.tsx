import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  RefreshCcw,
  Save,
} from "lucide-react";

import {
  disablePlatformSystemAgent,
  enablePlatformSystemAgent,
  fetchMainAgentConfig,
  fetchPlatformSystemAgents,
  rescanPlatformSystemAgents,
  updateMainAgentConfig,
} from "@/domain/agent/api";
import {
  AgentPolicySwitch,
  AgentStatusBadge,
} from "@/domain/agent/components";
import {
  getMainAgentConfigMock,
  listPlatformSystemAgentsMock,
  rescanPlatformSystemAgentsMock,
  setPlatformSystemAgentEnabledMock,
  updateMainAgentConfigMock,
} from "@/domain/agent/mock";
import type {
  MainAgentConfig,
  PlatformSystemAgentRecord,
} from "@/domain/agent/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { DataTable } from "@/components/data-table";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import {
  PageHeader,
  SurfaceCard,
  useLocaleFormatters,
  useTablePagination,
} from "@/shared";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";
import {
  Field,
  TextArea,
  TextInput,
} from "@/components/forms";

function formatLastScanned(lastScannedAt: string | null, formatDateTime: (value: string | number | Date) => string) {
  if (!lastScannedAt) {
    return "Not scanned";
  }

  return formatDateTime(lastScannedAt);
}

export function PlatformAgentsPage() {
  const { t } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const dataSource = getServiceDataSource("agent");
  const [mainAgentConfig, setMainAgentConfig] = useState<MainAgentConfig | null>(null);
  const [draft, setDraft] = useState({ personality: "", instructions: "" });
  const [systemAgents, setSystemAgents] = useState<PlatformSystemAgentRecord[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
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
  const hasDraftChanges = mainAgentConfig !== null && (
    mainAgentConfig.personality !== draft.personality
    || mainAgentConfig.instructions !== draft.instructions
  );

  async function loadPage() {
    setLoading(true);
    try {
      const [config, records] = dataSource === "api"
        ? await Promise.all([
          fetchMainAgentConfig(),
          fetchPlatformSystemAgents(),
        ])
        : await Promise.all([
          Promise.resolve(getMainAgentConfigMock()),
          Promise.resolve(listPlatformSystemAgentsMock()),
        ]);
      setMainAgentConfig(config);
      setDraft({
        instructions: config.instructions,
        personality: config.personality,
      });
      setSystemAgents(records);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Agent management data could not be loaded"));
    } finally {
      setLoading(false);
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

  async function handleSave() {
    if (!hasDraftChanges || saving) {
      return;
    }

    setSaving(true);
    try {
      const updated = dataSource === "api"
        ? await updateMainAgentConfig(draft)
        : updateMainAgentConfigMock(draft);
      setMainAgentConfig(updated);
      setDraft({
        instructions: updated.instructions,
        personality: updated.personality,
      });
      toast.success(pageT("messages.saved", "Main Agent saved"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.save", "Main Agent could not be saved"));
    } finally {
      setSaving(false);
    }
  }

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

      <SurfaceCard className="space-y-5">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">
              {pageT("mainAgent.title", "Main Agent")}
            </h2>
          </div>
          <div className="flex flex-col items-end gap-3">
            <Button
              disabled={!hasDraftChanges || saving || loading}
              aria-busy={saving}
              onClick={() => void handleSave()}
            >
              <Save size={16} aria-hidden="true" />
              {saving ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </div>
        </div>

        <div className="grid gap-4 lg:grid-cols-2">
          <Field label={pageT("mainAgent.personality", "Personality")}>
            <TextInput
              aria-label={pageT("mainAgent.personality", "Personality")}
              placeholder={pageT("mainAgent.personalityPlaceholder", "How should the platform main agent behave?")}
              value={draft.personality}
              onChange={(event) => setDraft((current) => ({ ...current, personality: event.target.value }))}
            />
          </Field>
          <div className="hidden lg:block" />
          <Field label={pageT("mainAgent.instructions", "Instructions")}>
            <TextArea
              aria-label={pageT("mainAgent.instructions", "Instructions")}
              className="min-h-36"
              placeholder={pageT("mainAgent.instructionsPlaceholder", "Persistent instructions injected into the runtime main-agent path.")}
              value={draft.instructions}
              onChange={(event) => setDraft((current) => ({ ...current, instructions: event.target.value }))}
            />
          </Field>
        </div>

      </SurfaceCard>

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
              <div className="flex justify-end">
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
    </div>
  );
}

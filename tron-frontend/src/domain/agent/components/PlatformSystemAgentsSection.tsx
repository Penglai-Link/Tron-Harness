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
import { SearchField } from "@/components/search-field";
import {
  useLocaleFormatters,
} from "@/shared";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";
import { AgentWorkspaceAssignment } from "./AgentWorkspaceAssignment";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Field } from "@/components/forms";
import { translateSystemAgentDescription, translateSystemAgentSource } from "../constants/agentPresentation";

const brandImages: Record<string, string> = {
  codex: "/agent-brands/codex-pixel.png",
  opencode: "/agent-brands/opencode-pixel.png",
};

function SystemAgentAvatar({ code, name }: { code: string; name: string }) {
  const image = brandImages[code];
  return <span className="flex size-20 shrink-0 items-center justify-center overflow-hidden rounded-full bg-white ring-1 ring-[var(--color-outline-variant)]">
    {image ? <img src={image} alt={name} className="size-full object-contain [image-rendering:pixelated]" /> : <span className="text-xl font-semibold text-black" aria-hidden="true">{name.slice(0, 2)}</span>}
  </span>;
}

export function PlatformSystemAgentsSection() {
  const { t } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const dataSource = getServiceDataSource("agent");
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
  const [selectedCode, setSelectedCode] = useState<string | null>(null);
  const selectedAgent = systemAgents.find((agent) => agent.agentCode === selectedCode);
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
    if (rowBusyCode || scanning || !record.available) {
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

  return <section aria-label={pageT("table.title", "System Agents")} className="space-y-4">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <h2 className="text-base font-semibold">{pageT("table.title", "System Agents")}</h2>
      <div className="flex flex-wrap items-center gap-3">
        <SearchField ariaLabel={pageT("table.searchAria", "Search system agents")} className="w-full sm:w-60" placeholder={pageT("table.searchPlaceholder", "Search system agents...")} value={searchQuery} onChange={setSearchQuery} />
        <Button variant="secondary" disabled={scanning || Boolean(rowBusyCode)} aria-busy={scanning} onClick={() => void handleScan()}>
          <RefreshCcw size={16} aria-hidden="true" />
          {scanning ? pageT("actions.scanning", "Scanning...") : pageT("actions.scan", "Scan")}
        </Button>
      </div>
    </div>
    <div className="grid grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-3 sm:grid-cols-[repeat(auto-fill,minmax(128px,1fr))]">
      {filteredAgents.map((agent) => <Button key={agent.agentCode} variant="ghost" aria-label={pageT("table.manageLabel", "Manage {{name}}", { name: agent.name })}
        className="group h-auto min-w-0 flex-col gap-2 rounded-xl px-2 py-4 text-center whitespace-normal" onClick={() => setSelectedCode(agent.agentCode)}>
        <span className="transition-transform duration-150 motion-reduce:transition-none group-hover:scale-105"><SystemAgentAvatar code={agent.agentCode} name={agent.name} /></span>
        <span className="block w-full truncate text-sm font-medium">{agent.name}</span>
        <span className="block w-full truncate font-mono text-xs text-[var(--color-on-surface-variant)]">{agent.agentCode}</span>
      </Button>)}
    </div>
    {!filteredAgents.length ? <p className="text-sm text-[var(--color-on-surface-variant)]">{pageT("table.empty", "No system agents match the current filters.")}</p> : null}
    {selectedAgent ? <Dialog disablePointerDismissal open onOpenChange={(open, details) => {
      if (details.reason === "outside-press" || rowBusyCode === selectedAgent.agentCode) { details.cancel(); return; }
      if (!open) setSelectedCode(null);
    }}>
      <DialogContent className="max-h-[90dvh] overflow-y-auto sm:max-w-lg" closeLabel={t("common:actions.close")} showCloseButton={rowBusyCode !== selectedAgent.agentCode}>
        <DialogHeader>
          <DialogTitle>{selectedAgent.name}</DialogTitle>
          <DialogDescription>{selectedAgent.agentCode}</DialogDescription>
        </DialogHeader>
        <div className="flex items-center gap-4">
          <SystemAgentAvatar code={selectedAgent.agentCode} name={selectedAgent.name} />
          <p className="text-sm text-[var(--color-on-surface-variant)]">{translateSystemAgentDescription(t, selectedAgent)}</p>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={pageT("table.columns.status", "Status")}><AgentStatusBadge status={selectedAgent.status} /></Field>
          <Field label={pageT("table.columns.source", "Source")}><p className="text-sm">{translateSystemAgentSource(t, selectedAgent.source)}</p></Field>
          <Field label={pageT("table.columns.lastScanned", "Last Scanned")}><p className="text-sm">{selectedAgent.lastScannedAt ? formatDateTime(selectedAgent.lastScannedAt) : pageT("table.notScanned", "Not scanned")}</p></Field>
          <Field label={pageT("table.columns.actions", "Actions")}>
            <AgentPolicySwitch checked={selectedAgent.enabled} disabled={!selectedAgent.available || Boolean(rowBusyCode) || scanning} loading={rowBusyCode === selectedAgent.agentCode}
              label={!selectedAgent.available ? pageT("table.unavailableLabel", "{{name}} unavailable", { name: selectedAgent.name }) : selectedAgent.enabled ? pageT("table.disableLabel", "Disable {{name}}", { name: selectedAgent.name }) : pageT("table.enableLabel", "Enable {{name}}", { name: selectedAgent.name })}
              onToggle={() => void handleToggleAgent(selectedAgent)} />
          </Field>
        </div>
        <div className="flex items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] pt-4">
          <span className="text-sm font-medium">{t("pages:platformData.agentAuthorization")}</span>
          <AgentWorkspaceAssignment code={selectedAgent.agentCode} name={selectedAgent.name} />
        </div>
      </DialogContent>
    </Dialog> : null}
  </section>;
}

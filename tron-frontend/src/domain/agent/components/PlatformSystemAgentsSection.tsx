import { AgentActivationSwitch } from "./AgentActivationSwitch";
import { AgentAvatar, SYSTEM_AGENT_AVATAR_IDS } from "./AgentAvatarPicker";
import { notifyInternalAgentRegistryChanged } from "../model";
import { AgentAvailabilityAvatar } from "./AgentAvailabilityAvatar";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  disablePlatformSystemAgent,
  enablePlatformSystemAgent,
  fetchPlatformSystemAgents,
  rescanPlatformSystemAgents,
  createPlatformSystemAgent,
  updatePlatformSystemAgent,
  deletePlatformSystemAgent,
} from "@/domain/agent/api";
import {
  listPlatformSystemAgentsMock,
  rescanPlatformSystemAgentsMock,
  setPlatformSystemAgentEnabledMock,
  createPlatformSystemAgentMock,
  updatePlatformSystemAgentMock,
  deletePlatformSystemAgentMock,
} from "@/domain/agent/mock";
import type {
  PlatformSystemAgentRecord,
} from "@/domain/agent/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { SurfaceCard, useLocaleFormatters } from "@/shared";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";
import { AgentWorkspaceAssignment } from "./AgentWorkspaceAssignment";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Field } from "@/components/forms";
import { translateSystemAgentDescription, translateSystemAgentSource } from "../constants/agentPresentation";
import { SystemAgentProfileDialog } from "./SystemAgentProfileDialog";

function SystemAgentAvatar({ code, name, avatarId }: { code: string; name: string; avatarId?: string | null }) {
  const id = avatarId || SYSTEM_AGENT_AVATAR_IDS[code];
  return <span className="flex size-20 shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--color-elevated-surface)] ring-1 ring-[var(--color-outline-variant)]">
    {id ? <AgentAvatar avatarId={id} alt={name} className="size-full" /> : <span className="text-xl font-semibold text-[var(--color-on-surface)]" aria-hidden="true">{name.slice(0, 2)}</span>}
  </span>;
}

export function PlatformSystemAgentsSection() {
  const { t } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const dataSource = getServiceDataSource("agent");
  const [systemAgents, setSystemAgents] = useState<PlatformSystemAgentRecord[]>([]);
  const [scanning, setScanning] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:settingsAgents.${key}`, { defaultValue, ...options })
  );

  const [selectedCode, setSelectedCode] = useState<string | null>(null);
  const [profileDialog, setProfileDialog] = useState<"new" | PlatformSystemAgentRecord | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
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
      notifyInternalAgentRegistryChanged();
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

  async function removeProfile(record: PlatformSystemAgentRecord) {
    setRowBusyCode(record.agentCode);
    try {
      if (dataSource === "api") await deletePlatformSystemAgent(record.agentCode);
      else deletePlatformSystemAgentMock(record.agentCode);
      setConfirmDelete(false); setSelectedCode(null);
      await refreshSystemAgents();
    } catch (error) { toast.error(error instanceof Error ? error.message : pageT("profile.saveFailed", "Could not save system Agent")); }
    finally { setRowBusyCode(""); }
  }

  return <section aria-label={pageT("table.title", "System Agents")} className="space-y-4">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <h2 className="text-base font-semibold">{pageT("table.title", "System Agents")}</h2>
      <div className="flex flex-wrap items-center gap-3">
        <Button disabled={scanning || Boolean(rowBusyCode)} onClick={() => setProfileDialog("new")}>{pageT("profile.new", "New")}</Button>
        <Button disabled={scanning || Boolean(rowBusyCode)} aria-busy={scanning} onClick={() => void handleScan()}>

          {scanning ? pageT("actions.scanning", "Scanning...") : pageT("actions.scan", "Scan")}
        </Button>
      </div>
    </div>
    <SurfaceCard className="min-h-40 p-4">
      {systemAgents.length ? <div className="grid grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-3 sm:grid-cols-[repeat(auto-fill,minmax(128px,1fr))]">
        {systemAgents.map((agent) => <Button key={agent.agentCode} variant="ghost" aria-label={pageT("table.manageLabel", "Manage {{name}}", { name: agent.name })}
          className="group h-auto min-w-0 justify-start flex-col gap-2 rounded-xl px-2 py-4 text-center whitespace-normal" onClick={() => setSelectedCode(agent.agentCode)}>
          <span className="transition-transform duration-150 motion-reduce:transition-none group-hover:scale-105"><AgentAvailabilityAvatar enabled={agent.enabled}><SystemAgentAvatar code={agent.baseAgentCode || agent.agentCode} name={agent.name} avatarId={agent.avatarId} /></AgentAvailabilityAvatar></span>
          <span className="block w-full truncate text-sm font-medium">{agent.name}</span>
          <span className="block w-full truncate font-mono text-xs text-[var(--color-on-surface-variant)]">{agent.agentCode}</span>
          <span className="block h-4 w-full truncate text-xs text-[var(--color-on-surface-variant)]" title={agent.modelId ?? undefined}>{agent.modelId ?? ""}</span>
        </Button>)}
      </div> : <p className="text-sm text-[var(--color-on-surface-variant)]">{pageT("table.empty", "No system agents available.")}</p>}
    </SurfaceCard>
    {selectedAgent ? <Dialog disablePointerDismissal open onOpenChange={(open, details) => {
      if (details.reason === "outside-press" || rowBusyCode === selectedAgent.agentCode) { details.cancel(); return; }
      if (!open) setSelectedCode(null);
    }}>
      <DialogContent className="max-h-[90dvh] overflow-y-auto sm:max-w-lg" closeLabel={t("common:actions.close")} showCloseButton={rowBusyCode !== selectedAgent.agentCode}>
        <DialogHeader>
          <div className="flex items-center justify-between gap-4 pr-8">
            <DialogTitle>{selectedAgent.name}</DialogTitle>
            <AgentActivationSwitch
              checked={selectedAgent.enabled}
              disabled={!selectedAgent.available || Boolean(rowBusyCode) || scanning}
              onChange={() => void handleToggleAgent(selectedAgent)}
            />
          </div>
          <DialogDescription>{selectedAgent.agentCode}</DialogDescription>
        </DialogHeader>
        <div className="flex items-center gap-4">
          <SystemAgentAvatar code={selectedAgent.baseAgentCode || selectedAgent.agentCode} name={selectedAgent.name} avatarId={selectedAgent.avatarId} />
          <p className="text-sm text-[var(--color-on-surface-variant)]">{translateSystemAgentDescription(t, selectedAgent)}</p>
        </div>
        {selectedAgent.modelId ? <Field label={pageT("profile.model", "Model")}><p className="break-all text-sm">{selectedAgent.modelId}</p></Field> : null}
        {<div className="flex justify-end gap-2">
          <Button variant="outline" disabled={Boolean(rowBusyCode)} onClick={() => { setSelectedCode(null); setProfileDialog(selectedAgent); }}>{pageT("profile.edit", "Edit")}</Button>
          {selectedAgent.baseAgentCode && <Button variant="outline" disabled={Boolean(rowBusyCode)} onClick={() => setConfirmDelete(true)}>{pageT("profile.delete", "Delete")}</Button>}
        </div>}
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={pageT("table.columns.source", "Source")}><p className="text-sm">{translateSystemAgentSource(t, selectedAgent.source)}</p></Field>
          <Field label={pageT("table.columns.lastScanned", "Last Scanned")}><p className="text-sm">{selectedAgent.lastScannedAt ? formatDateTime(selectedAgent.lastScannedAt) : pageT("table.notScanned", "Not scanned")}</p></Field>
        </div>
        <div className="flex items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] pt-4">
          <span className="text-sm font-medium">{t("pages:platformData.agentAuthorization")}</span>
          <AgentWorkspaceAssignment code={selectedAgent.agentCode} name={selectedAgent.name} />
        </div>
      </DialogContent>
    </Dialog> : null}
    {profileDialog ? <SystemAgentProfileDialog agents={systemAgents} record={profileDialog === "new" ? undefined : profileDialog} onClose={() => setProfileDialog(null)} onSave={async ({ enabled, ...payload }) => {
      if (profileDialog === "new") {
        if (dataSource === "api") await createPlatformSystemAgent(payload);
        else createPlatformSystemAgentMock(payload);
      } else {
        if (dataSource === "api") await updatePlatformSystemAgent(payload.agentCode, { name: payload.name, description: payload.description, modelId: payload.modelId, reasoningEffort: payload.reasoningEffort, avatarId: payload.avatarId, fastMode: payload.fastMode });
        else updatePlatformSystemAgentMock(payload.agentCode, payload);
      }
      if (profileDialog === "new" ? !enabled : enabled !== profileDialog.enabled) {
        if (dataSource === "api") {
          if (enabled) await enablePlatformSystemAgent(payload.agentCode);
          else await disablePlatformSystemAgent(payload.agentCode);
        } else setPlatformSystemAgentEnabledMock(payload.agentCode, enabled);
      }
      await refreshSystemAgents();
      notifyInternalAgentRegistryChanged();
    }} /> : null}
    {confirmDelete && selectedAgent ? <Dialog open onOpenChange={(open) => { if (!rowBusyCode) setConfirmDelete(open); }}>
      <DialogContent showCloseButton={!rowBusyCode}>
        <DialogHeader><DialogTitle>{pageT("profile.deleteTitle", "Delete system Agent?")}</DialogTitle><DialogDescription>{selectedAgent.name}</DialogDescription></DialogHeader>
        <div className="flex justify-end gap-2"><Button variant="outline" disabled={Boolean(rowBusyCode)} onClick={() => setConfirmDelete(false)}>{pageT("profile.cancel", "Cancel")}</Button><Button disabled={Boolean(rowBusyCode)} onClick={() => void removeProfile(selectedAgent)}>{pageT("profile.delete", "Delete")}</Button></div>
      </DialogContent>
    </Dialog> : null}
  </section>;
}

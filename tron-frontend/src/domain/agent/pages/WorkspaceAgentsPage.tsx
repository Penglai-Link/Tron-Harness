import {
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import {
  fetchToolDomains,
  fetchWorkspaceInternalAgents,
  fetchWorkspaceSystemAgents,
  listTools,
  patchWorkspaceInternalAgentMcpPolicy,
  patchWorkspaceInternalAgentPolicy,
  updateWorkspaceInternalAgentMcpServerAssignments,
  updateWorkspaceInternalAgentToolAssignments,
} from "@/domain/agent/api";
import {
  AgentPolicySwitch,
  AgentStatusBadge,
  InternalAgentMcpServerSelector,
  InternalAgentToolSelector,
} from "@/domain/agent/components";
import {
  translateInheritedFrom,
  translateInternalAgentName,
  translateSystemAgentDescription,
  translateSystemAgentSource,
} from "@/domain/agent/constants/agentPresentation";
import {
  listToolDomainsMock,
  listToolsMock,
  listWorkspaceInternalAgentsMock,
  patchWorkspaceInternalAgentMcpPolicyMock,
  patchWorkspaceInternalAgentPolicyMock,
  updateWorkspaceInternalAgentMcpServerAssignmentsMock,
  updateWorkspaceInternalAgentToolAssignmentsMock,
  listWorkspaceSystemAgentsMock,
  setWorkspaceSystemAgentEnabledMock,
} from "@/domain/agent/mock";
import {
  disableWorkspaceSystemAgent,
  enableWorkspaceSystemAgent,
} from "@/domain/agent/api";
import type {
  PlatformToolRecord,
  ToolCatalogDomainRecord,
  WorkspaceInternalAgentMcpPolicyPayload,
  WorkspaceInternalAgentPolicyPayload,
  WorkspaceInternalAgentRecord,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";
import {
  fetchWorkspaceMcpServers,
  listWorkspaceMcpServersMock,
} from "@/domain/mcp";
import type { WorkspaceMcpServerRecord } from "@/domain/mcp/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { DataTable } from "@/components/data-table";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import {
  PageHeader,
  SurfaceCard,
  useTablePagination,
  useWorkspaceSpaces,
} from "@/shared";
import { useToast } from "@/components/ui/toast";
import { SelectItem } from "@/components/ui/select";

interface InternalAgentDraft {
  expandedDomainPersistenceMode: WorkspaceInternalAgentRecord["expandedDomainPersistenceMode"];
  maxSteps: number;
  maxToolCalls: number;
  maxParallelToolCalls: number;
  mcpAccessMode: WorkspaceInternalAgentRecord["mcpAccessMode"];
  selectedServerCodes: string[];
  selectedToolCodes: string[];
  toolAssignmentMode: WorkspaceInternalAgentRecord["toolAssignmentMode"];
  toolExposureMode: WorkspaceInternalAgentRecord["toolExposureMode"];
}

type InternalSaveState = "idle" | "pending" | "saving" | "saved" | "error";

function normalizeToolCodes(toolCodes: string[]) {
  return [...new Set(toolCodes)].sort((left, right) => left.localeCompare(right));
}

function normalizeServerCodes(serverCodes: string[]) {
  return [...new Set(serverCodes)].sort((left, right) => left.localeCompare(right));
}

function hasSameToolCodes(left: string[], right: string[]) {
  if (left.length !== right.length) {
    return false;
  }

  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) {
      return false;
    }
  }

  return true;
}

function hasSameServerCodes(left: string[], right: string[]) {
  if (left.length !== right.length) {
    return false;
  }

  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) {
      return false;
    }
  }

  return true;
}

function draftFromRecord(record: WorkspaceInternalAgentRecord): InternalAgentDraft {
  return {
    expandedDomainPersistenceMode: record.expandedDomainPersistenceMode,
    maxSteps: record.maxSteps,
    maxToolCalls: record.maxToolCalls,
    maxParallelToolCalls: record.maxParallelToolCalls,
    mcpAccessMode: record.mcpAccessMode,
    selectedServerCodes: normalizeServerCodes(record.selectedServerCodes),
    selectedToolCodes: normalizeToolCodes(record.selectedToolCodes),
    toolAssignmentMode: record.toolAssignmentMode,
    toolExposureMode: record.toolExposureMode,
  };
}

function hasDraftChanges(record: WorkspaceInternalAgentRecord, draft: InternalAgentDraft) {
  return (
    record.toolAssignmentMode !== draft.toolAssignmentMode
    || record.toolExposureMode !== draft.toolExposureMode
    || record.expandedDomainPersistenceMode !== draft.expandedDomainPersistenceMode
    || record.maxSteps !== draft.maxSteps
    || record.maxToolCalls !== draft.maxToolCalls
    || record.maxParallelToolCalls !== draft.maxParallelToolCalls
    || record.mcpAccessMode !== draft.mcpAccessMode
    || !hasSameServerCodes(normalizeServerCodes(record.selectedServerCodes), normalizeServerCodes(draft.selectedServerCodes))
    || !hasSameToolCodes(normalizeToolCodes(record.selectedToolCodes), normalizeToolCodes(draft.selectedToolCodes))
  );
}

function buildDraft(
  baseRecord: WorkspaceInternalAgentRecord,
  currentDraft: InternalAgentDraft,
  patch: Partial<InternalAgentDraft>,
) {
  return {
    ...currentDraft,
    ...patch,
    selectedServerCodes: patch.selectedServerCodes
      ? normalizeServerCodes(patch.selectedServerCodes)
      : currentDraft.selectedServerCodes ?? normalizeServerCodes(baseRecord.selectedServerCodes),
    selectedToolCodes: patch.selectedToolCodes
      ? normalizeToolCodes(patch.selectedToolCodes)
      : currentDraft.selectedToolCodes ?? normalizeToolCodes(baseRecord.selectedToolCodes),
  };
}

export type WorkspaceAgentCategory = "internal" | "system";

interface WorkspaceAgentsPageProps {
  category: WorkspaceAgentCategory;
}

export function WorkspaceAgentsPage({ category }: WorkspaceAgentsPageProps) {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const dataSource = getServiceDataSource("agent");
  const mcpDataSource = getServiceDataSource("mcp");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode =
    selectedSecondarySpace?.workspaceCode
    ?? selectedPrimarySpace?.workspaceCode
    ?? "";
  const activeWorkspaceLabel =
    selectedSecondarySpace?.label
    ?? selectedPrimarySpace?.name
    ?? t("pages:workspaceAgents.workspace.unselected", { defaultValue: "No workspace selected" });

  const [internalAgents, setInternalAgents] = useState<WorkspaceInternalAgentRecord[]>([]);
  const [toolDomains, setToolDomains] = useState<ToolCatalogDomainRecord[]>([]);
  const [tools, setTools] = useState<PlatformToolRecord[]>([]);
  const [workspaceMcpServers, setWorkspaceMcpServers] = useState<WorkspaceMcpServerRecord[]>([]);
  const [systemAgents, setSystemAgents] = useState<WorkspaceSystemAgentRecord[]>([]);
  const [internalDrafts, setInternalDrafts] = useState<Record<string, InternalAgentDraft>>({});
  const [internalSaveStates, setInternalSaveStates] = useState<Record<string, InternalSaveState>>({});
  const [systemSearch, setSystemSearch] = useState("");
  const [debouncedSystemSearch, setDebouncedSystemSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [busyKeys, setBusyKeys] = useState<Record<string, boolean>>({});
  const internalSaveTimersRef = useRef<Record<string, number>>({});
  const internalSaveFlashTimersRef = useRef<Record<string, number>>({});

  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:workspaceAgents.${key}`, { defaultValue, ...options })
  );
  const internalPageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:workspaceAgents.internal.${key}`, { defaultValue, ...options })
  );

  const systemTableT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:workspaceAgents.table.${key}`, { defaultValue, ...options })
  );
  const internalAgentsPage = category === "internal";
  const pageTitle = internalAgentsPage
    ? internalPageT("title", "Internal Agents")
    : systemTableT("title", "Inherited System Agents");

  function fieldLabel(
    key:
      | "expandedDomainPersistenceMode"
      | "maxSteps"
      | "maxToolCalls"
      | "maxParallelToolCalls"
      | "mcpAccessMode"
      | "toolAssignmentMode"
      | "toolExposureMode",
    defaultValue: string,
    align: "end" | "start" = "start",
  ) {
    return (
      <span className="inline-flex items-center gap-1.5">
        <span>{internalPageT(`labels.${key}`, defaultValue)}</span>
        <InfoTooltip
          align={align}
          ariaLabel={internalPageT(`help.${key}.ariaLabel`, `Explain ${defaultValue}`)}
        >
          <p>{internalPageT(`help.${key}.summary`, "")}</p>
          <div className="space-y-1">
            <p>
              <strong>{internalPageT("help.optionLabel", "Options")}</strong>
            </p>
            <p>{internalPageT(`help.${key}.optionOne`, "")}</p>
            <p>{internalPageT(`help.${key}.optionTwo`, "")}</p>
          </div>
        </InfoTooltip>
      </span>
    );
  }

  function isBusy(code: string) {
    return Boolean(busyKeys[code]);
  }

  function setBusy(code: string, busy: boolean) {
    setBusyKeys((current) => {
      if (busy) {
        if (current[code]) {
          return current;
        }
        return {
          ...current,
          [code]: true,
        };
      }

      if (!current[code]) {
        return current;
      }

      const next = { ...current };
      delete next[code];
      return next;
    });
  }

  function clearInternalSaveTimer(agentCode: string) {
    const timer = internalSaveTimersRef.current[agentCode];
    if (timer) {
      window.clearTimeout(timer);
      delete internalSaveTimersRef.current[agentCode];
    }
  }

  function clearInternalSaveFlashTimer(agentCode: string) {
    const timer = internalSaveFlashTimersRef.current[agentCode];
    if (timer) {
      window.clearTimeout(timer);
      delete internalSaveFlashTimersRef.current[agentCode];
    }
  }

  function setInternalSaveState(agentCode: string, nextState: InternalSaveState) {
    setInternalSaveStates((current) => {
      if (current[agentCode] === nextState) {
        return current;
      }
      return {
        ...current,
        [agentCode]: nextState,
      };
    });
  }

  function updateDraftMap(next: WorkspaceInternalAgentRecord[]) {
    const nextDrafts = next.reduce<Record<string, InternalAgentDraft>>((accum, item) => {
      accum[item.agentCode] = draftFromRecord(item);
      return accum;
    }, {});
    setInternalDrafts(nextDrafts);
  }

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSystemSearch(systemSearch.trim().toLowerCase());
    }, 200);

    return () => window.clearTimeout(timer);
  }, [systemSearch]);

  useEffect(() => {
    void refreshWorkspaceData();
  }, [activeWorkspaceCode, category, dataSource, mcpDataSource]);

  useEffect(() => (
    () => {
      Object.keys(internalSaveTimersRef.current).forEach(clearInternalSaveTimer);
      Object.keys(internalSaveFlashTimersRef.current).forEach(clearInternalSaveFlashTimer);
    }
  ), []);

  useEffect(() => {
    if (!activeWorkspaceCode) {
      Object.keys(internalSaveTimersRef.current).forEach(clearInternalSaveTimer);
      return;
    }

    internalAgents.forEach((agent) => {
      const agentCode = agent.agentCode;
      const draft = internalDrafts[agentCode];
      const saveState = internalSaveStates[agentCode] ?? "idle";
      const hasChanges = draft ? hasDraftChanges(agent, draft) : false;

      if (!draft || !hasChanges || saveState !== "pending" || isBusy(`internal-save:${agentCode}`) || loading) {
        clearInternalSaveTimer(agentCode);
        return;
      }

      clearInternalSaveTimer(agentCode);
      internalSaveTimersRef.current[agentCode] = window.setTimeout(() => {
        void handleSaveInternalAgent(agentCode);
      }, 450);
    });
  }, [activeWorkspaceCode, internalAgents, internalDrafts, internalSaveStates, loading]);

  const filteredSystemAgents = systemAgents.filter((record) => {
    const normalizedSearch = debouncedSystemSearch;
    if (!normalizedSearch) {
      return true;
    }

    return (
      [
        record.name,
        record.agentCode,
        record.description,
        record.inheritedFrom,
        record.status,
        translateSystemAgentDescription(t, record),
        translateSystemAgentSource(t, record.source),
        translateInheritedFrom(t, record.inheritedFrom),
      ].join(" ").toLowerCase().includes(normalizedSearch)
    );
  });

  const systemPagination = useTablePagination(filteredSystemAgents);

  function resolveWorkspacePolicyLabel(record: WorkspaceSystemAgentRecord) {
    if (!record.available) {
      return systemTableT("policyUnavailable", "Unavailable");
    }
    if (!record.platformEnabled) {
      return systemTableT("policyPlatformDisabled", "Platform disabled");
    }
    if (record.workspaceOverrideDisabled) {
      return systemTableT("policyLocalDisable", "Local disable");
    }
    return systemTableT("policyInherited", "Platform baseline");
  }

  function canToggleWorkspaceAgent(record: WorkspaceSystemAgentRecord) {
    if (!record.available) {
      return false;
    }
    return record.workspaceOverrideDisabled || record.platformEnabled;
  }

  function hasToolChanges(record: WorkspaceInternalAgentRecord, draft: InternalAgentDraft) {
    return !hasSameToolCodes(normalizeToolCodes(record.selectedToolCodes), normalizeToolCodes(draft.selectedToolCodes));
  }

  function hasMcpServerChanges(record: WorkspaceInternalAgentRecord, draft: InternalAgentDraft) {
    return !hasSameServerCodes(normalizeServerCodes(record.selectedServerCodes), normalizeServerCodes(draft.selectedServerCodes));
  }

  function setInternalDraft(agentCode: string, patch: Partial<InternalAgentDraft>) {
    const baseRecord = internalAgents.find((item) => item.agentCode === agentCode);
    if (!baseRecord) {
      return;
    }

    let nextDraft = draftFromRecord(baseRecord);
    setInternalDrafts((current) => {
      nextDraft = buildDraft(baseRecord, current[agentCode] ?? draftFromRecord(baseRecord), patch);
      return {
        ...current,
        [agentCode]: nextDraft,
      };
    });

    clearInternalSaveFlashTimer(agentCode);
    setInternalSaveState(agentCode, hasDraftChanges(baseRecord, nextDraft) ? "pending" : "idle");
  }

  function updateLimitDraft(
    agentCode: string,
    key: "maxSteps" | "maxToolCalls" | "maxParallelToolCalls",
    valueAsNumber: number,
  ) {
    const draft = internalDrafts[agentCode];
    if (!draft) {
      return;
    }

    const nextValue = Number.isFinite(valueAsNumber)
      ? Math.max(1, Math.trunc(valueAsNumber))
      : draft[key];
    setInternalDraft(agentCode, {
      [key]: nextValue,
    } as Partial<InternalAgentDraft>);
  }

  function handleToolSelection(agentCode: string, toolCode: string, checked: boolean) {
    const draft = internalDrafts[agentCode];
    if (!draft) return;

    const next = new Set(draft.selectedToolCodes);
    if (checked) {
      next.add(toolCode);
    } else {
      next.delete(toolCode);
    }

    setInternalDraft(agentCode, {
      selectedToolCodes: [...next],
      toolAssignmentMode: "selected_tools",
    });
  }

  function handleBulkToolSelection(agentCode: string, toolCodes: string[], checked: boolean) {
    const draft = internalDrafts[agentCode];
    if (!draft || toolCodes.length === 0) return;

    const next = new Set(draft.selectedToolCodes);
    toolCodes.forEach((toolCode) => {
      if (checked) {
        next.add(toolCode);
      } else {
        next.delete(toolCode);
      }
    });

    setInternalDraft(agentCode, {
      selectedToolCodes: [...next],
      toolAssignmentMode: "selected_tools",
    });
  }

  function handleServerSelection(agentCode: string, serverCode: string, checked: boolean) {
    const draft = internalDrafts[agentCode];
    if (!draft) return;

    const next = new Set(draft.selectedServerCodes);
    if (checked) {
      next.add(serverCode);
    } else {
      next.delete(serverCode);
    }

    setInternalDraft(agentCode, {
      mcpAccessMode: "selected_servers",
      selectedServerCodes: [...next],
    });
  }

  function handleBulkServerSelection(agentCode: string, serverCodes: string[], checked: boolean) {
    const draft = internalDrafts[agentCode];
    if (!draft || serverCodes.length === 0) return;

    const next = new Set(draft.selectedServerCodes);
    serverCodes.forEach((serverCode) => {
      if (checked) {
        next.add(serverCode);
      } else {
        next.delete(serverCode);
      }
    });

    setInternalDraft(agentCode, {
      mcpAccessMode: "selected_servers",
      selectedServerCodes: [...next],
    });
  }

  async function refreshWorkspaceData() {
    if (!activeWorkspaceCode) {
      setInternalAgents([]);
      setToolDomains([]);
      setTools([]);
      setWorkspaceMcpServers([]);
      setSystemAgents([]);
      setInternalDrafts({});
      setInternalSaveStates({});
      setBusyKeys({});
      Object.keys(internalSaveTimersRef.current).forEach(clearInternalSaveTimer);
      Object.keys(internalSaveFlashTimersRef.current).forEach(clearInternalSaveFlashTimer);
      return;
    }

    setLoading(true);
    try {
      if (internalAgentsPage) {
        const [nextInternalAgents, nextToolDomains, nextTools, nextWorkspaceMcpServers] = await Promise.all([
          dataSource === "api"
            ? fetchWorkspaceInternalAgents(activeWorkspaceCode)
            : Promise.resolve(listWorkspaceInternalAgentsMock(activeWorkspaceCode)),
          dataSource === "api"
            ? fetchToolDomains()
            : Promise.resolve(listToolDomainsMock()),
          dataSource === "api"
            ? listTools()
            : Promise.resolve(listToolsMock()),
          mcpDataSource === "api"
            ? fetchWorkspaceMcpServers(activeWorkspaceCode)
            : Promise.resolve(listWorkspaceMcpServersMock(activeWorkspaceCode)),
        ]);

        setInternalAgents(nextInternalAgents);
        setToolDomains(nextToolDomains);
        setTools(nextTools);
        setWorkspaceMcpServers(nextWorkspaceMcpServers);
        updateDraftMap(nextInternalAgents);
        setInternalSaveStates({});
      } else {
        const nextSystemAgents = dataSource === "api"
          ? await fetchWorkspaceSystemAgents(activeWorkspaceCode)
          : await Promise.resolve(listWorkspaceSystemAgentsMock(activeWorkspaceCode));
        setSystemAgents(nextSystemAgents);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Workspace agent policy could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  async function refreshInternalAgents() {
    if (!activeWorkspaceCode) return;

    const nextInternalAgents = dataSource === "api"
      ? await fetchWorkspaceInternalAgents(activeWorkspaceCode)
      : listWorkspaceInternalAgentsMock(activeWorkspaceCode);
    setInternalAgents(nextInternalAgents);
    updateDraftMap(nextInternalAgents);
  }

  async function handleSaveInternalAgent(agentCode: string) {
    const row = internalAgents.find((item) => item.agentCode === agentCode);
    const draft = internalDrafts[agentCode];
    if (!row || !draft || isBusy(`internal-save:${agentCode}`)) {
      return;
    }
    if (!hasDraftChanges(row, draft)) {
      return;
    }

    const payload: WorkspaceInternalAgentPolicyPayload = {};
    if (row.toolAssignmentMode !== draft.toolAssignmentMode) {
      payload.toolAssignmentMode = draft.toolAssignmentMode;
    }
    if (row.toolExposureMode !== draft.toolExposureMode) {
      payload.toolExposureMode = draft.toolExposureMode;
    }
    if (row.expandedDomainPersistenceMode !== draft.expandedDomainPersistenceMode) {
      payload.expandedDomainPersistenceMode = draft.expandedDomainPersistenceMode;
    }
    if (row.maxSteps !== draft.maxSteps) {
      payload.maxSteps = draft.maxSteps;
    }
    if (row.maxToolCalls !== draft.maxToolCalls) {
      payload.maxToolCalls = draft.maxToolCalls;
    }
    if (row.maxParallelToolCalls !== draft.maxParallelToolCalls) {
      payload.maxParallelToolCalls = draft.maxParallelToolCalls;
    }
    const mcpPayload: WorkspaceInternalAgentMcpPolicyPayload = {};
    if (row.mcpAccessMode !== draft.mcpAccessMode) {
      mcpPayload.mcpAccessMode = draft.mcpAccessMode;
    }

    const shouldSaveTools = draft.toolAssignmentMode === "selected_tools"
      ? hasToolChanges(row, draft)
      : false;
    const shouldSaveServers = draft.mcpAccessMode === "selected_servers"
      ? hasMcpServerChanges(row, draft)
      : false;

    clearInternalSaveTimer(agentCode);
    clearInternalSaveFlashTimer(agentCode);
    setBusy(`internal-save:${agentCode}`, true);
    setInternalSaveState(agentCode, "saving");
    try {
      if (Object.keys(payload).length > 0) {
        if (dataSource === "api") {
          await patchWorkspaceInternalAgentPolicy(activeWorkspaceCode, agentCode, payload);
        } else {
          patchWorkspaceInternalAgentPolicyMock(activeWorkspaceCode, agentCode, payload);
        }
      }
      if (Object.keys(mcpPayload).length > 0) {
        if (dataSource === "api") {
          await patchWorkspaceInternalAgentMcpPolicy(activeWorkspaceCode, agentCode, mcpPayload);
        } else {
          patchWorkspaceInternalAgentMcpPolicyMock(activeWorkspaceCode, agentCode, mcpPayload);
        }
      }
      if (shouldSaveTools) {
        if (dataSource === "api") {
          await updateWorkspaceInternalAgentToolAssignments(activeWorkspaceCode, agentCode, draft.selectedToolCodes);
        } else {
          updateWorkspaceInternalAgentToolAssignmentsMock(activeWorkspaceCode, agentCode, draft.selectedToolCodes);
        }
      }
      if (shouldSaveServers) {
        if (dataSource === "api") {
          await updateWorkspaceInternalAgentMcpServerAssignments(activeWorkspaceCode, agentCode, draft.selectedServerCodes);
        } else {
          updateWorkspaceInternalAgentMcpServerAssignmentsMock(activeWorkspaceCode, agentCode, draft.selectedServerCodes);
        }
      }
      await refreshInternalAgents();
      setInternalSaveState(agentCode, "saved");
      internalSaveFlashTimersRef.current[agentCode] = window.setTimeout(() => {
        setInternalSaveState(agentCode, "idle");
        delete internalSaveFlashTimersRef.current[agentCode];
      }, 1600);
    } catch (error) {
      setInternalSaveState(agentCode, "error");
      toast.error(error instanceof Error ? error.message : internalPageT("errors.save", "Internal agent policy update failed"));
    } finally {
      setBusy(`internal-save:${agentCode}`, false);
    }
  }

  async function handleToggleSystemAgent(row: WorkspaceSystemAgentRecord) {
    if (!activeWorkspaceCode || isBusy(`system:${row.agentCode}`) || !canToggleWorkspaceAgent(row)) {
      return;
    }

    setBusy(`system:${row.agentCode}`, true);
    try {
      const updated = dataSource === "api"
        ? (row.enabled
          ? await disableWorkspaceSystemAgent(activeWorkspaceCode, row.agentCode)
          : await enableWorkspaceSystemAgent(activeWorkspaceCode, row.agentCode))
        : setWorkspaceSystemAgentEnabledMock(activeWorkspaceCode, row.agentCode, !row.enabled);
      await refreshWorkspaceData();
      if (row.enabled) {
        toast.success(pageT("messages.disabled", "{{name}} disabled in {{workspace}}", {
          name: updated.name,
          workspace: activeWorkspaceLabel,
        }));
      } else if (updated.enabled) {
        toast.success(pageT("messages.enabled", "{{name}} now follows platform policy", { name: updated.name }));
      } else {
        toast.success(pageT("messages.stillDisabled", "{{name}} follows platform policy and remains disabled", {
          name: updated.name,
        }));
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.toggle", "Workspace agent policy update failed"));
    } finally {
      setBusy(`system:${row.agentCode}`, false);
    }
  }

  if (!activeWorkspaceCode) {
    return (
      <SurfaceCard>
        <h1 className="text-2xl font-semibold text-[var(--color-on-surface)]">
          {pageTitle}
        </h1>
        <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
          {internalAgentsPage
            ? pageT("emptyInternalWorkspace", "Select a workspace to manage internal agents.")
            : pageT("emptyWorkspace", "Select a workspace to manage inherited system-agent policy.")}
        </p>
      </SurfaceCard>
    );
  }

  return (
    <div className="space-y-5">
      <PageHeader title={pageTitle} />

      {internalAgentsPage ? (
        <SurfaceCard className="space-y-3 p-4">
        {internalAgents.length === 0 ? (
          <p className="px-1 py-2 text-sm text-[var(--color-on-surface-variant)]">
            {internalPageT("empty", "No internal agents found for this workspace.")}
          </p>
        ) : null}

        {internalAgents.map((agent) => {
          const draft = internalDrafts[agent.agentCode] ?? draftFromRecord(agent);
          const isSaving = isBusy(`internal-save:${agent.agentCode}`);
          const internalBusy = isSaving || loading;
          const saveState = internalSaveStates[agent.agentCode] ?? "idle";

          let statusLabel = "";
          let statusClassName = "text-[var(--color-on-surface-variant)]";
          if (saveState === "pending") {
            statusLabel = internalPageT("status.pending", "Changes will be saved automatically");
          } else if (saveState === "saving") {
            statusLabel = internalPageT("status.saving", "Saving...");
          } else if (saveState === "saved") {
            statusLabel = internalPageT("status.saved", "Saved automatically");
          } else if (saveState === "error") {
            statusLabel = internalPageT("status.error", "Autosave failed. Update again to retry.");
            statusClassName = "text-[var(--color-error)]";
          }

          return (
            <section key={agent.agentCode} className="rounded-2xl bg-[var(--color-surface-container-low)] p-4">
              <div className="flex flex-wrap items-start justify-between gap-2.5">
                <div>
                  <h3 className="text-base font-semibold text-[var(--color-on-surface)]">
                    {translateInternalAgentName(t, agent)}
                  </h3>
                  <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">
                    {internalPageT("agentCode", "{{code}}", { code: agent.agentCode })}
                  </p>
                </div>
                <div className="min-h-5 text-right text-xs">
                  {statusLabel ? (
                    <p className={statusClassName}>
                      {statusLabel}
                    </p>
                  ) : null}
                </div>
              </div>

              <div className="mt-3 grid gap-3 lg:grid-cols-2">
                <Field label={fieldLabel("mcpAccessMode", "MCP access mode")}>
                  <SelectInput
                    disabled={internalBusy}
                    value={draft.mcpAccessMode}
                    onValueChange={(event) => {
                      setInternalDraft(agent.agentCode, {
                        mcpAccessMode: event as WorkspaceInternalAgentRecord["mcpAccessMode"],
                      });
                    }}
                  >
                    <SelectItem value="disabled">
                      {internalPageT("modes.mcpDisabled", "Disabled")}
                    </SelectItem>
                    <SelectItem value="all_enabled_servers">
                      {internalPageT("modes.allEnabledServers", "All enabled servers")}
                    </SelectItem>
                    <SelectItem value="selected_servers">
                      {internalPageT("modes.selectedServers", "Selected servers")}
                    </SelectItem>
                  </SelectInput>
                </Field>
                <Field label={fieldLabel("toolAssignmentMode", "Tool assignment mode")}>
                  <SelectInput
                    disabled={internalBusy}
                    value={draft.toolAssignmentMode}
                    onValueChange={(event) => {
                      setInternalDraft(agent.agentCode, {
                        toolAssignmentMode: event as WorkspaceInternalAgentRecord["toolAssignmentMode"],
                      });
                    }}
                  >
                    <SelectItem value="all_platform_tools">
                      {internalPageT("modes.allTools", "All platform tools")}
                    </SelectItem>
                    <SelectItem value="selected_tools">
                      {internalPageT("modes.selectedTools", "Selected tools")}
                    </SelectItem>
                  </SelectInput>
                </Field>
                <Field label={fieldLabel("toolExposureMode", "Tool exposure mode", "end")}>
                  <SelectInput
                    disabled={internalBusy}
                    value={draft.toolExposureMode}
                    onValueChange={(event) => {
                      setInternalDraft(agent.agentCode, {
                        toolExposureMode: event as WorkspaceInternalAgentRecord["toolExposureMode"],
                      });
                    }}
                  >
                    <SelectItem value="direct_list">
                      {internalPageT("modes.directList", "Direct list")}
                    </SelectItem>
                    <SelectItem value="domain_discovery">
                      {internalPageT("modes.domainDiscovery", "Domain discovery")}
                    </SelectItem>
                  </SelectInput>
                </Field>
                <Field label={fieldLabel("expandedDomainPersistenceMode", "Expanded domain persistence mode")}>
                  <SelectInput
                    disabled={internalBusy}
                    value={draft.expandedDomainPersistenceMode}
                    onValueChange={(event) => {
                      setInternalDraft(agent.agentCode, {
                        expandedDomainPersistenceMode:
                          event as WorkspaceInternalAgentRecord["expandedDomainPersistenceMode"],
                      });
                    }}
                  >
                    <SelectItem value="sticky_expanded">
                      {internalPageT("modes.stickyExpanded", "Sticky expanded")}
                    </SelectItem>
                    <SelectItem value="ephemeral_expanded">
                      {internalPageT("modes.ephemeralExpanded", "Ephemeral expanded")}
                    </SelectItem>
                  </SelectInput>
                </Field>
                <Field label={fieldLabel("maxSteps", "Max steps")}>
                  <TextInput
                    aria-label={internalPageT("labels.maxSteps", "Max steps")}
                    disabled={internalBusy}
                    min={1}
                    step={1}
                    type="number"
                    value={draft.maxSteps}
                    onChange={(event) => {
                      updateLimitDraft(agent.agentCode, "maxSteps", event.currentTarget.valueAsNumber);
                    }}
                  />
                </Field>
                <Field label={fieldLabel("maxToolCalls", "Max tool calls")}>
                  <TextInput
                    aria-label={internalPageT("labels.maxToolCalls", "Max total tool calls")}
                    disabled={internalBusy}
                    min={1}
                    step={1}
                    type="number"
                    value={draft.maxToolCalls}
                    onChange={(event) => {
                      updateLimitDraft(agent.agentCode, "maxToolCalls", event.currentTarget.valueAsNumber);
                    }}
                  />
                </Field>
                <Field label={fieldLabel("maxParallelToolCalls", "Max parallel tool calls", "end")}>
                  <TextInput
                    aria-label={internalPageT("labels.maxParallelToolCalls", "Max parallel tool calls")}
                    disabled={internalBusy}
                    min={1}
                    step={1}
                    type="number"
                    value={draft.maxParallelToolCalls}
                    onChange={(event) => {
                      updateLimitDraft(agent.agentCode, "maxParallelToolCalls", event.currentTarget.valueAsNumber);
                    }}
                  />
                </Field>
              </div>

              {draft.mcpAccessMode === "selected_servers" ? (
                <InternalAgentMcpServerSelector
                  busy={internalBusy}
                  selectedServerCodes={draft.selectedServerCodes}
                  servers={workspaceMcpServers}
                  onSetServersChecked={(serverCodes, checked) => {
                    handleBulkServerSelection(agent.agentCode, serverCodes, checked);
                  }}
                  onToggleServer={(serverCode, checked) => {
                    handleServerSelection(agent.agentCode, serverCode, checked);
                  }}
                />
              ) : null}

              {draft.toolAssignmentMode === "selected_tools" ? (
                <InternalAgentToolSelector
                  busy={internalBusy}
                  selectedToolCodes={draft.selectedToolCodes}
                  toolDomains={toolDomains}
                  tools={tools}
                  onSetToolsChecked={(toolCodes, checked) => {
                    handleBulkToolSelection(agent.agentCode, toolCodes, checked);
                  }}
                  onToggleTool={(toolCode, checked) => {
                    handleToolSelection(agent.agentCode, toolCode, checked);
                  }}
                />
              ) : null}
            </section>
          );
        })}
        </SurfaceCard>
      ) : null}

      {!internalAgentsPage ? (
        <DataTable<WorkspaceSystemAgentRecord>
        rows={systemPagination.visibleItems}
        rowKey={(row) => `${activeWorkspaceCode}-${row.agentCode}`}
        pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "workspace agents")} {...systemPagination.paginationProps} />}
        toolbar={(
          <SearchField
            ariaLabel={systemTableT("searchAria", "Search workspace agents")}
            className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
            placeholder={systemTableT("searchPlaceholder", "Search workspace agents...")}
            value={systemSearch}
            onChange={setSystemSearch}
          />
        )}
        emptyState={systemTableT("empty", "No workspace agents match the current filters.")}
        columns={[
          {
            id: "agent",
            header: systemTableT("columns.agent", "Agent"),
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
            header: systemTableT("columns.description", "Description"),
            width: "24%",
            render: (row) => (
              <p className="text-[13px] text-[var(--color-on-surface-variant)]">
                {translateSystemAgentDescription(t, row)}
              </p>
            ),
          },
          {
            id: "source",
            header: systemTableT("columns.source", "Source"),
            width: "12%",
            render: (row) => translateSystemAgentSource(t, row.source),
          },
          {
            id: "inheritedFrom",
            header: systemTableT("columns.inheritedFrom", "Inherited From"),
            width: "14%",
            render: (row) => translateInheritedFrom(t, row.inheritedFrom),
          },
          {
            id: "status",
            header: systemTableT("columns.status", "Status"),
            width: "12%",
            render: (row) => <AgentStatusBadge status={row.status} />,
          },
          {
            id: "policy",
            header: systemTableT("columns.policy", "Policy"),
            width: "18%",
            render: (row) => resolveWorkspacePolicyLabel(row),
          },
          {
            id: "actions",
            header: systemTableT("columns.actions", "Actions"),
            align: "right",
            width: "16%",
            render: (row) => (
              <div className="flex justify-end">
                <AgentPolicySwitch
                  checked={row.enabled}
                  disabled={!canToggleWorkspaceAgent(row) || isBusy(`system:${row.agentCode}`) || loading}
                  label={!row.available
                    ? systemTableT("unavailableLabel", "{{name}} unavailable", { name: row.name })
                    : !row.platformEnabled && !row.workspaceOverrideDisabled
                      ? systemTableT("platformDisabledLabel", "{{name}} disabled by platform policy", { name: row.name })
                      : row.workspaceOverrideDisabled
                        ? systemTableT("enableLabel", "Enable {{name}}", { name: row.name })
                        : systemTableT("disableLabel", "Disable {{name}}", { name: row.name })}
                  loading={isBusy(`system:${row.agentCode}`)}
                  onToggle={() => void handleToggleSystemAgent(row)}
                />
              </div>
            ),
          },
        ]}
        />
      ) : null}
    </div>
  );
}

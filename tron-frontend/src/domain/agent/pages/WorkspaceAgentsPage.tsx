import {
  Fragment,
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown } from "lucide-react";

import {
  fetchToolDomains,
  fetchWorkspaceInternalAgents,
  fetchWorkspaceSystemAgents,
  listTools,
  patchWorkspaceInternalAgentMcpPolicy,
  patchWorkspaceInternalAgentPolicy,
  patchWorkspaceInternalAgentDefinition,
  resetWorkspaceInternalAgentPolicy,
  updateWorkspaceInternalAgentMcpServerAssignments,
  updateWorkspaceInternalAgentToolAssignments,
} from "@/domain/agent/api";
import {
  AgentPolicySwitch,
  AgentStatusBadge,
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
  patchWorkspaceInternalAgentDefinitionMock,
  resetWorkspaceInternalAgentPolicyMock,
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
  fetchPlatformMcpServers,
  listPlatformMcpServersMock,
  listWorkspaceMcpServersMock,
} from "@/domain/mcp";
import type { WorkspaceMcpServerRecord } from "@/domain/mcp/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { DataTable } from "@/components/data-table";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { CreateInternalAgentDialog } from "../components/CreateInternalAgentDialog";
import { InternalAgentKnowledgeField } from "../components/InternalAgentKnowledgeField";
import { AgentRolePromptField } from "../components/AgentRolePromptField";
import { InternalAgentAssignmentDialog, type InternalAgentAssignmentKind } from "../components/InternalAgentAssignmentDialog";
import { PLATFORM_INTERNAL_AGENT_SCOPE, notifyInternalAgentRegistryChanged } from "../model";
import { Button } from "@/components/ui/button";
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

import { AgentAvatar, AgentAvatarPicker } from "../components/AgentAvatarPicker";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useQueryClient } from "@tanstack/react-query";
import { setAgentKnowledgeLibraries } from "@/domain/knowledge-base/api/knowledgeBaseApi";
import { InternalAgentBrowseItem } from "../components/InternalAgentBrowseItem";
import { InternalAgentGroupField } from "../components/InternalAgentGroupField";
import { useInternalAgentGroups } from "../hooks/useInternalAgentGroups";
import { groupInternalAgents } from "../model/internalAgentGroups";
import { AgentLoopField } from "../components/AgentLoopField";

interface InternalAgentDraft {
  groupName: string;
  avatarId: string;
  loopCode: string;
  name: string;
  rolePrompt: string;
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
    avatarId: record.avatarId || "robot",
    groupName: record.groupName || "",
    name: record.name,
    rolePrompt: record.rolePrompt,
    loopCode: record.loopCode || "builtin.react-loop",
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

function mergeSavedDraft(
  record: WorkspaceInternalAgentRecord,
  submitted: InternalAgentDraft,
  latest: InternalAgentDraft,
) {
  let merged = draftFromRecord(record);
  for (const key of Object.keys(merged) as (keyof InternalAgentDraft)[]) {
    const currentValue = latest[key];
    const submittedValue = submitted[key];
    const changedSinceSubmission = Array.isArray(currentValue) && Array.isArray(submittedValue)
      ? !hasSameToolCodes(currentValue, submittedValue)
      : currentValue !== submittedValue;
    if (changedSinceSubmission) merged = { ...merged, [key]: currentValue };
  }
  return merged;
}

function hasDraftChanges(record: WorkspaceInternalAgentRecord, draft: InternalAgentDraft) {
  return (
    (record.groupName || "") !== draft.groupName
    || (record.avatarId || "robot") !== draft.avatarId
    || record.name !== draft.name
    || (record.loopCode || "builtin.react-loop") !== draft.loopCode
    || record.rolePrompt !== draft.rolePrompt
    ||
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
  allowCreate?: boolean;
  scope?: "platform" | "workspace";
}

export function WorkspaceAgentsPage({ category, scope = "workspace", allowCreate = true }: WorkspaceAgentsPageProps) {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const dataSource = getServiceDataSource("agent");
  const mcpDataSource = getServiceDataSource("mcp");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode =
    scope === "platform" ? PLATFORM_INTERNAL_AGENT_SCOPE : selectedSecondarySpace?.workspaceCode
    ?? selectedPrimarySpace?.workspaceCode
    ?? "";
  const activeWorkspaceLabel =
    selectedSecondarySpace?.label
    ?? selectedPrimarySpace?.name
    ?? t("pages:workspaceAgents.workspace.unselected", { defaultValue: "No workspace selected" });

  const queryClient = useQueryClient();
  const [editingAgentCode, setEditingAgentCode] = useState<string | null>(null);
  const [restoreBaselines, setRestoreBaselines] = useState<Record<string, WorkspaceInternalAgentRecord>>({});
  const [knowledgeDrafts, setKnowledgeDrafts] = useState<Record<string, string[]>>({});
  const [assignmentDialog, setAssignmentDialog] = useState<{ agentCode: string; workspaceCode: string; kind: InternalAgentAssignmentKind } | null>(null);
  const [internalAgents, setInternalAgents] = useState<WorkspaceInternalAgentRecord[]>([]);
  const [toolDomains, setToolDomains] = useState<ToolCatalogDomainRecord[]>([]);
  const [tools, setTools] = useState<PlatformToolRecord[]>([]);
  const [workspaceMcpServers, setWorkspaceMcpServers] = useState<WorkspaceMcpServerRecord[]>([]);
  const [systemAgents, setSystemAgents] = useState<WorkspaceSystemAgentRecord[]>([]);
  const [internalDrafts, setInternalDrafts] = useState<Record<string, InternalAgentDraft>>({});
  const [internalSaveStates, setInternalSaveStates] = useState<Record<string, InternalSaveState>>({});
  const [internalSaveErrors, setInternalSaveErrors] = useState<Record<string, string>>({});
  const [internalSearch, setInternalSearch] = useState("");
  const [groupFilter, setGroupFilter] = useState("all");
  const { groups: catalogGroups } = useInternalAgentGroups(category === "internal" ? activeWorkspaceCode : "");
  const [systemSearch, setSystemSearch] = useState("");
  const [debouncedSystemSearch, setDebouncedSystemSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [busyKeys, setBusyKeys] = useState<Record<string, boolean>>({});
  const internalDraftsRef = useRef<Record<string, InternalAgentDraft>>({});

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
  const pageTitle = scope === "platform" ? internalPageT("inheritance.title", "Global internal agents") : internalAgentsPage
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
    internalDraftsRef.current = nextDrafts;
    setInternalDrafts(nextDrafts);
  }

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSystemSearch(systemSearch.trim().toLowerCase());
    }, 200);

    return () => window.clearTimeout(timer);
  }, [systemSearch]);

  useEffect(() => {
    setEditingAgentCode(null);
    setKnowledgeDrafts({});
    setRestoreBaselines({});
    void refreshWorkspaceData();
  }, [activeWorkspaceCode, category, dataSource, mcpDataSource, scope]);

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

    // React may defer state updaters. Keep consecutive input events on the latest
    // draft and compute save status without relying on updater execution timing.
    const nextDraft = buildDraft(
      baseRecord,
      internalDraftsRef.current[agentCode] ?? draftFromRecord(baseRecord),
      patch,
    );
    internalDraftsRef.current = { ...internalDraftsRef.current, [agentCode]: nextDraft };
    setInternalDrafts((current) => ({ ...current, [agentCode]: nextDraft }));

    setInternalSaveState(agentCode, hasDraftChanges(baseRecord, nextDraft) ? "pending" : "idle");
  }

  function updateLimitDraft(
    agentCode: string,
    key: "maxSteps" | "maxToolCalls" | "maxParallelToolCalls",
    valueAsNumber: number,
  ) {
    const draft = internalDraftsRef.current[agentCode];
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

  function openAssignmentDialog(agentCode: string, kind: InternalAgentAssignmentKind) {
    setAssignmentDialog({ agentCode, kind, workspaceCode: activeWorkspaceCode });
  }

  async function refreshWorkspaceData() {
    if (!activeWorkspaceCode) {
      setInternalAgents([]);
      setToolDomains([]);
      setTools([]);
      setWorkspaceMcpServers([]);
      setSystemAgents([]);
      internalDraftsRef.current = {};
      setInternalDrafts({});
      setInternalSaveStates({});
      setInternalSaveErrors({});
      setBusyKeys({});
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
          scope === "platform"
            ? (mcpDataSource === "api" ? fetchPlatformMcpServers() : Promise.resolve(listPlatformMcpServersMock()))
              .then((servers) => servers.map((server): WorkspaceMcpServerRecord => ({ ...server,
                platformEnabled: server.enabled, effectiveEnabled: server.enabled,
                source: "platform", workspaceOverrideDisabled: false,
              })))
            : mcpDataSource === "api"
              ? fetchWorkspaceMcpServers(activeWorkspaceCode)
              : Promise.resolve(listWorkspaceMcpServersMock(activeWorkspaceCode)),
        ]);

        setInternalAgents(nextInternalAgents);
        setToolDomains(nextToolDomains);
        setTools(nextTools);
        setWorkspaceMcpServers(nextWorkspaceMcpServers);
        updateDraftMap(nextInternalAgents);
        setInternalSaveStates({});
        setInternalSaveErrors({});
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

  async function refreshCreatedAgent() {
    const records = dataSource === "api"
      ? await fetchWorkspaceInternalAgents(activeWorkspaceCode)
      : listWorkspaceInternalAgentsMock(activeWorkspaceCode);
    setInternalAgents(records);
    setInternalDrafts((current) => Object.fromEntries(records.map((record) => [record.agentCode,
      current[record.agentCode] ?? draftFromRecord(record),
    ])));
  }

  async function refreshInternalAgent(agentCode: string, submittedDraft?: InternalAgentDraft) {
    if (!activeWorkspaceCode) return;

    const nextInternalAgents = dataSource === "api"
      ? await fetchWorkspaceInternalAgents(activeWorkspaceCode)
      : listWorkspaceInternalAgentsMock(activeWorkspaceCode);
    const savedAgent = nextInternalAgents.find((agent) => agent.agentCode === agentCode);
    if (!savedAgent) return;

    // Other rows may have pending edits while this row's save is in flight.
    // Apply the authoritative response only to the row whose save completed.
    setInternalAgents((current) => current.map((agent) => (
      agent.agentCode === agentCode ? savedAgent : agent
    )));
    // Keep edits made while saving; only unchanged fields adopt the server response.
    const latestDraft = internalDraftsRef.current[agentCode];
    const savedDraft = submittedDraft && latestDraft
      ? mergeSavedDraft(savedAgent, submittedDraft, latestDraft)
      : draftFromRecord(savedAgent);
    internalDraftsRef.current = { ...internalDraftsRef.current, [agentCode]: savedDraft };
    setInternalDrafts((current) => ({ ...current, [agentCode]: savedDraft }));
    return hasDraftChanges(savedAgent, savedDraft);
  }

  async function handleSaveInternalAgent(agentCode: string) {
    const row = restoreBaselines[agentCode] ?? internalAgents.find((item) => item.agentCode === agentCode);
    const draft = internalDraftsRef.current[agentCode];
    if (!row || row.builtIn || !draft || isBusy(`internal-save:${agentCode}`) || isBusy(`internal-reset:${agentCode}`)) {
      return;
    }
    if (!hasDraftChanges(row, draft) && !knowledgeDrafts[agentCode] && !restoreBaselines[agentCode]) {
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
      && (row.toolAssignmentMode !== "selected_tools" || hasToolChanges(row, draft));
    const shouldSaveServers = draft.mcpAccessMode === "selected_servers"
      ? hasMcpServerChanges(row, draft)
      : false;

    setBusy(`internal-save:${agentCode}`, true);
    setInternalSaveState(agentCode, "saving");
    try {
      if (restoreBaselines[agentCode]) {
        if (dataSource === "api") await resetWorkspaceInternalAgentPolicy(activeWorkspaceCode, agentCode);
        else resetWorkspaceInternalAgentPolicyMock(activeWorkspaceCode, agentCode);
      }
      if ((row.groupName || "") !== draft.groupName || row.name !== draft.name || row.rolePrompt !== draft.rolePrompt || (row.loopCode || "builtin.react-loop") !== draft.loopCode || (row.avatarId || "robot") !== draft.avatarId) {
        const definition = {
          ...((row.groupName || "") !== draft.groupName ? { groupName: draft.groupName } : {}),
          ...((row.avatarId || "robot") !== draft.avatarId ? { avatarId: draft.avatarId } : {}),
          ...((row.loopCode || "builtin.react-loop") !== draft.loopCode ? { loopCode: draft.loopCode } : {}),
          ...(row.name !== draft.name ? { name: draft.name } : {}),
          ...(row.rolePrompt !== draft.rolePrompt ? { rolePrompt: draft.rolePrompt } : {}),
        };
        if (dataSource === "api") {
          await patchWorkspaceInternalAgentDefinition(activeWorkspaceCode, agentCode, definition);
        } else {
          patchWorkspaceInternalAgentDefinitionMock(activeWorkspaceCode, agentCode, definition);
        }
      }
      // Persist the visible selection before changing its policy mode. The all-tools
      // response is an effective list, so it may differ from stored assignments.
      if (shouldSaveTools) {
        if (dataSource === "api") {
          await updateWorkspaceInternalAgentToolAssignments(activeWorkspaceCode, agentCode, draft.selectedToolCodes);
        } else {
          updateWorkspaceInternalAgentToolAssignmentsMock(activeWorkspaceCode, agentCode, draft.selectedToolCodes);
        }
      }
      if (Object.keys(payload).length > 0) {
        if (dataSource === "api") {
          await patchWorkspaceInternalAgentPolicy(activeWorkspaceCode, agentCode, payload);
        } else {
          patchWorkspaceInternalAgentPolicyMock(activeWorkspaceCode, agentCode, payload);
        }
      }
      if (shouldSaveServers) {
        if (dataSource === "api") {
          await updateWorkspaceInternalAgentMcpServerAssignments(activeWorkspaceCode, agentCode, draft.selectedServerCodes);
        } else {
          updateWorkspaceInternalAgentMcpServerAssignmentsMock(activeWorkspaceCode, agentCode, draft.selectedServerCodes);
        }
      }
      if (Object.keys(mcpPayload).length > 0) {
        if (dataSource === "api") {
          await patchWorkspaceInternalAgentMcpPolicy(activeWorkspaceCode, agentCode, mcpPayload);
        } else {
          patchWorkspaceInternalAgentMcpPolicyMock(activeWorkspaceCode, agentCode, mcpPayload);
        }
      }
      if (knowledgeDrafts[agentCode]) {
        await setAgentKnowledgeLibraries(agentCode, knowledgeDrafts[agentCode]);
        await Promise.all(["knowledge-agent-libraries", "knowledge-base", "knowledge-bases", "platform-knowledge-bases", "knowledge-library-agent-grants"].map((key) => queryClient.invalidateQueries({ queryKey: [key] })));
        setKnowledgeDrafts((current) => { const next = { ...current }; delete next[agentCode]; return next; });
      }
      const hasPendingChanges = await refreshInternalAgent(agentCode, draft);
      notifyInternalAgentRegistryChanged();
      setInternalSaveState(agentCode, hasPendingChanges ? "pending" : "saved");
      if (!hasPendingChanges) {
        setEditingAgentCode(null);
        toast.success(internalPageT("status.saved", "Saved"));
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : internalPageT("errors.save", "Internal agent policy update failed");
      setInternalSaveErrors((current) => ({ ...current, [agentCode]: message }));
      setInternalSaveState(agentCode, "error");
      toast.error(message);
    } finally {
      setBusy(`internal-save:${agentCode}`, false);
    }
  }

  async function handleRestoreInheritance(agentCode: string) {
    setBusy(`internal-reset:${agentCode}`, true);
    try {
      const platformRecords = dataSource === "api"
        ? await fetchWorkspaceInternalAgents(PLATFORM_INTERNAL_AGENT_SCOPE)
        : listWorkspaceInternalAgentsMock(PLATFORM_INTERNAL_AGENT_SCOPE);
      const baseline = platformRecords.find((record) => record.agentCode === agentCode);
      if (!baseline) throw new Error(internalPageT("errors.reset", "Internal policy reset failed"));
      const draft = draftFromRecord(baseline);
      setRestoreBaselines((current) => ({ ...current, [agentCode]: baseline }));
      internalDraftsRef.current = { ...internalDraftsRef.current, [agentCode]: draft };
      setInternalDrafts((current) => ({ ...current, [agentCode]: draft }));
      setInternalSaveState(agentCode, "pending");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : internalPageT("errors.reset", "Internal policy reset failed"));
    } finally { setBusy(`internal-reset:${agentCode}`, false); }
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
    <div className={internalAgentsPage && scope === "platform" ? "space-y-4" : "space-y-5"}>
      {internalAgentsPage && scope === "platform" ? (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="text-base font-semibold">{t("pages:platformData.agentDefinitions")}</h2>
        </div>
      ) : <PageHeader title={pageTitle} />}

      {internalAgentsPage ? (
        <div className="space-y-5">
        <div className="flex flex-wrap items-center gap-3">
          <SearchField ariaLabel={internalPageT("groups.search", "Search agents or groups")} placeholder={internalPageT("groups.search", "Search agents or groups")} className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]" value={internalSearch} onChange={setInternalSearch} />
          <SelectInput aria-label={internalPageT("groups.label", "Group")} className="w-full sm:w-48" value={groupFilter} onValueChange={(value) => setGroupFilter(value || "all")}>
            <SelectItem value="all">{internalPageT("groups.all", "All groups")}</SelectItem>
            <SelectItem value="ungrouped">{internalPageT("groups.ungrouped", "Ungrouped")}</SelectItem>
            {[...new Set([...catalogGroups, ...internalAgents.flatMap((agent) => agent.groupName ? [agent.groupName] : [])])].sort().map((name) => <SelectItem key={name} value={`group:${name}`}>{name}</SelectItem>)}
          </SelectInput>
          {allowCreate ? <div className="sm:ml-auto"><CreateInternalAgentDialog key={activeWorkspaceCode} workspaceCode={activeWorkspaceCode} onCreated={refreshCreatedAgent} /></div> : null}
        </div>
        {groupInternalAgents(internalAgents, internalSearch, groupFilter, (agent) => translateInternalAgentName(t, agent)).length === 0 ? (
          <p className="px-1 py-2 text-sm text-[var(--color-on-surface-variant)]">
            {internalPageT("groups.empty", "No agents match the current filters.")}
          </p>
        ) : null}

        {groupInternalAgents(internalAgents, internalSearch, groupFilter, (agent) => translateInternalAgentName(t, agent)).map(({ name: groupName, agents }) => <details key={`${activeWorkspaceCode}:${groupName}`} open className="group/section border-t border-[var(--color-outline-variant)] pt-3">
          <summary className="mb-3 flex cursor-pointer list-none items-center gap-2 text-sm font-medium marker:hidden [&::-webkit-details-marker]:hidden">
            <ChevronDown aria-hidden="true" size={16} className="-rotate-90 transition-transform group-open/section:rotate-0" />
            <span className="min-w-0 truncate" title={groupName || undefined}>{groupName || internalPageT("groups.ungrouped", "Ungrouped")}</span>
            <span className="text-xs font-normal text-[var(--color-on-surface-variant)]">{agents.length}</span>
          </summary>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,128px),1fr))] gap-x-4 gap-y-5">
        {agents.map((agent) => {
          const draft = internalDrafts[agent.agentCode] ?? draftFromRecord(agent);
          const isSaving = isBusy(`internal-save:${agent.agentCode}`);
          const editingDisabled = Boolean(agent.builtIn) || isSaving || isBusy(`internal-reset:${agent.agentCode}`) || loading;
          const internalBusy = isSaving || isBusy(`internal-reset:${agent.agentCode}`) || loading;
          const saveState = internalSaveStates[agent.agentCode] ?? "idle";

          let statusLabel = "";
          let statusClassName = "text-[var(--color-on-surface-variant)]";
          if (saveState === "pending") {
            statusLabel = internalPageT("status.pending", "Unsaved changes");
          } else if (saveState === "saving") {
            statusLabel = internalPageT("status.saving", "Saving...");
          } else if (saveState === "saved") {
            statusLabel = internalPageT("status.saved", "Saved");
          } else if (saveState === "error") {
            statusLabel = internalPageT("status.error", "Save failed. Please retry.");
            if (internalSaveErrors[agent.agentCode]) statusLabel += ` ${internalSaveErrors[agent.agentCode]}`;
            statusClassName = "text-[var(--color-error)]";
          }

          return (
            <Fragment key={agent.agentCode}>
              <InternalAgentBrowseItem agent={agent} name={translateInternalAgentName(t, agent)} disabled={loading} onOpen={() => {
                const nextDraft = draftFromRecord(agent);
                internalDraftsRef.current = { ...internalDraftsRef.current, [agent.agentCode]: nextDraft };
                setInternalDrafts((current) => ({ ...current, [agent.agentCode]: nextDraft }));
                setInternalSaveState(agent.agentCode, "idle");
                setKnowledgeDrafts({});
                setRestoreBaselines({});
                setEditingAgentCode(agent.agentCode);
              }} />
              {editingAgentCode === agent.agentCode ? <Dialog disablePointerDismissal open onOpenChange={(open, details) => {
                if (details.reason === "outside-press" || internalBusy) { details.cancel(); return; }
                if (!open) { setEditingAgentCode(null); setAssignmentDialog(null); }
              }}>
              <DialogContent className="flex max-h-[90dvh] flex-col overflow-hidden sm:max-w-4xl" showCloseButton={!internalBusy} closeLabel={internalPageT("definition.close", "Close")}>
              <DialogHeader>
                <DialogTitle>{internalPageT(agent.builtIn ? "groups.view" : "editAgent", agent.builtIn ? "View {{name}}" : "Edit {{name}}", { name: translateInternalAgentName(t, agent) })}</DialogTitle>
                <DialogDescription>{internalPageT("agentCode", "{{code}}", { code: agent.agentCode })}</DialogDescription>
              </DialogHeader>
              <div className="tron-scrollbar min-h-0 overflow-y-auto px-1 pb-1">
                {scope !== "platform" || statusLabel ? <div className="text-right text-xs">
                  {scope !== "platform" ? <p className="text-[var(--color-on-surface-variant)]">{!agent.platformDefined ? internalPageT("inheritance.local", "Workspace definition")
                      : agent.source === "workspace_override" ? internalPageT("inheritance.override", "Workspace override")
                        : internalPageT("inheritance.inherited", "Inherited from platform")}</p> : null}
                  {scope !== "platform" && !agent.builtIn && agent.platformDefined && agent.source === "workspace_override" ? <Button variant="outline" size="sm" disabled={internalBusy} onClick={() => void handleRestoreInheritance(agent.agentCode)}>{internalPageT("inheritance.restore", "Restore inheritance")}</Button> : null}
                  <p className={`min-h-4 ${statusClassName}`} role="status" aria-live="polite" aria-atomic="true">
                    {statusLabel}
                  </p>
                </div> : null}

              <div className="mt-3 space-y-3">
                {agent.builtIn ? <p className="text-sm text-[var(--color-on-surface-variant)]">{internalPageT("groups.readOnly", "Built-in agent definitions are read-only.")}</p> : null}
                {agent.builtIn ? <AgentAvatar avatarId={draft.avatarId} className="size-24 rounded-full" /> : <AgentAvatarPicker value={draft.avatarId} disabled={editingDisabled} onChange={(avatarId) => setInternalDraft(agent.agentCode, { avatarId })} />}
                <AgentLoopField value={draft.loopCode} disabled={editingDisabled} onChange={(loopCode) => setInternalDraft(agent.agentCode, { loopCode })} />
                <Field label={internalPageT("definition.name", "Name")}>
                  <TextInput aria-label={internalPageT("definition.name", "Name")} maxLength={100} disabled={editingDisabled} value={draft.name} onChange={(event) => setInternalDraft(agent.agentCode, { name: event.target.value })} />
                </Field>
                <InternalAgentGroupField workspaceCode={activeWorkspaceCode} value={draft.groupName} disabled={editingDisabled} onChange={(groupName) => setInternalDraft(agent.agentCode, { groupName })} />
                <AgentRolePromptField key={`${activeWorkspaceCode}:${agent.agentCode}`} workspaceCode={activeWorkspaceCode} disabled={editingDisabled} value={draft.rolePrompt} onChange={(value) => setInternalDraft(agent.agentCode, { rolePrompt: value })} />
              </div>

              <div className="mt-3 grid gap-3 lg:grid-cols-2">
                <Field className="w-full [&>div:first-child]:min-h-[var(--tron-button-height-sm,1.75rem)]" label={fieldLabel("mcpAccessMode", "MCP access mode")}
                  actions={draft.mcpAccessMode === "selected_servers" ? (
                    <Button type="button" size="sm" variant="outline" disabled={editingDisabled} onClick={() => openAssignmentDialog(agent.agentCode, "mcp")}>
                      {internalPageT("assignmentDialog.editServers", "Choose servers ({{count}})", { count: draft.selectedServerCodes.length })}
                    </Button>
                  ) : undefined}>

                  <SelectInput
                    disabled={editingDisabled}
                    value={draft.mcpAccessMode}
                    onValueChange={(event) => {
                      if (event === "selected_servers") openAssignmentDialog(agent.agentCode, "mcp");
                      else setInternalDraft(agent.agentCode, {
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
                <Field className="w-full [&>div:first-child]:min-h-[var(--tron-button-height-sm,1.75rem)]" label={fieldLabel("toolAssignmentMode", "Tool assignment mode")}
                  actions={draft.toolAssignmentMode === "selected_tools" ? (
                    <Button type="button" size="sm" variant="outline" disabled={editingDisabled} onClick={() => openAssignmentDialog(agent.agentCode, "tools")}>
                      {internalPageT("assignmentDialog.editTools", "Choose tools ({{count}})", { count: draft.selectedToolCodes.length })}
                    </Button>
                  ) : undefined}>

                  <SelectInput
                    disabled={editingDisabled}
                    value={draft.toolAssignmentMode}
                    onValueChange={(event) => {
                      if (event === "selected_tools") openAssignmentDialog(agent.agentCode, "tools");
                      else setInternalDraft(agent.agentCode, {
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
                <InternalAgentKnowledgeField key={`${activeWorkspaceCode}:${agent.agentCode}:knowledge`} agentCode={agent.agentCode} agentName={draft.name} disabled={editingDisabled} selectedIds={knowledgeDrafts[agent.agentCode]} onSelectionChange={agent.builtIn ? undefined : (ids) => setKnowledgeDrafts((current) => ({ ...current, [agent.agentCode]: ids }))} />
              </div>

              <details className="group mt-3 border-t border-[var(--color-outline-variant)]">
                <summary className="flex cursor-pointer list-none items-center justify-between gap-3 py-3 text-sm font-medium marker:hidden [&::-webkit-details-marker]:hidden">
                  {internalPageT("advancedSettings", "Advanced settings")}
                  <ChevronDown aria-hidden="true" size={16} className="shrink-0 transition-transform group-open:rotate-180" />
                </summary>
                <div className="grid gap-3 lg:grid-cols-2">
                  <Field label={fieldLabel("toolExposureMode", "Tool exposure mode", "end")}>
                    <SelectInput
                      disabled={editingDisabled}
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
                      disabled={editingDisabled}
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
                      disabled={editingDisabled}
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
                      disabled={editingDisabled}
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
                      disabled={editingDisabled}
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
              </details>

              {assignmentDialog?.workspaceCode === activeWorkspaceCode && assignmentDialog.agentCode === agent.agentCode ? (
                <InternalAgentAssignmentDialog
                  key={`${activeWorkspaceCode}:${agent.agentCode}:${assignmentDialog.kind}`}
                  kind={assignmentDialog.kind}
                  agentName={draft.name}
                  busy={editingDisabled}
                  initialSelectedCodes={assignmentDialog.kind === "mcp"
                    ? draft.selectedServerCodes
                    : draft.toolAssignmentMode === "selected_tools" ? draft.selectedToolCodes : []}
                  servers={workspaceMcpServers}
                  tools={tools}
                  toolDomains={toolDomains}
                  onClose={() => setAssignmentDialog(null)}
                  onConfirm={(codes) => {
                    setInternalDraft(agent.agentCode, assignmentDialog.kind === "mcp"
                      ? { mcpAccessMode: "selected_servers", selectedServerCodes: codes }
                      : { toolAssignmentMode: "selected_tools", selectedToolCodes: codes });
                    setAssignmentDialog(null);
                  }}
                />
              ) : null}
              </div>
              <DialogFooter>
                <Button variant="outline" disabled={internalBusy} onClick={() => { setEditingAgentCode(null); setAssignmentDialog(null); }}>{agent.builtIn ? internalPageT("definition.close", "Close") : internalPageT("definition.cancel", "Cancel")}</Button>
                {!agent.builtIn ? <Button disabled={internalBusy || !draft.name.trim() || (!hasDraftChanges(agent, draft) && !knowledgeDrafts[agent.agentCode] && !restoreBaselines[agent.agentCode])} onClick={() => void handleSaveInternalAgent(agent.agentCode)}>{isSaving ? internalPageT("actions.saving", "Saving...") : internalPageT("actions.save", "Save")}</Button> : null}
              </DialogFooter>
              </DialogContent>
              </Dialog> : null}
            </Fragment>
          );
        })}
          </div>
        </details>)}
        </div>
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
